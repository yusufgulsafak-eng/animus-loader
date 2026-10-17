//! The supplied ORIJINAL Shadows payload has a pinned converter and append-only
//! injector. Keep the index journal durable before starting that injector and
//! retain independent copies with the Loader's normal recovery material.
use crate::{
    error::{LoaderError, Result},
    models::{Action, ActionType, ChangeKind, ChangeRecord, Installation},
    security::path::resolve_inside,
    storage::{copy_file, hash_file, write_json_atomic},
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

pub const MANIFEST_SHA256: &str =
    "ee397ca77e96e0a1c53fff3cf0d26cd58b64a99cdad8c69d9ef34014d87ff6e4";
pub const TARGETS: [(&str, &str); 5] = [
    ("DataPC_boot", "DataPC_boot.forge"),
    ("DataPC_boot_patch_01", "DataPC_boot_patch_01.forge"),
    ("DataPC_boot_patch_02", "DataPC_boot_patch_02.forge"),
    ("dlc_20_boot", "dlc_20/DataPC_boot_20_dlc.forge"),
    (
        "dlc_20_boot_patch_01",
        "dlc_20/DataPC_boot_20_dlc_patch_01.forge",
    ),
];
const EXTERNAL: &str = ".animus-shadows-backup";
const TRANSACTION: &str = "shadows/transaction.json";
const INSTALL: &str = include_str!("shadows_install.ps1");
const UNINSTALL: &str = include_str!("shadows_uninstall.ps1");

fn invalid(message: impl Into<String>) -> LoaderError {
    LoaderError::Integrity(message.into())
}

pub fn validate_action(action: &Action) -> Result<()> {
    if action.kind != ActionType::InstallShadows
        || action.source.as_deref() != Some("MANIFEST_SHA256.txt")
        || action.destination != TARGETS[0].1
        || !action.backup
        || !action.options.is_empty()
        || action.expected_sha256.as_deref() != Some(MANIFEST_SHA256)
    {
        return Err(invalid(
            "INSTALL_SHADOWS: sabit ORIJINAL paket profili ve backup=true gerekli",
        ));
    }
    Ok(())
}

fn is_link(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    false
}

/// Check every component, including an existing dangling link. Windows junctions
/// and alternate data streams are also refused for this fixed package format.
fn safe_path(root: &Path, relative: &str) -> Result<PathBuf> {
    if relative.contains(['\\', ':', '\0'])
        || relative
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
    {
        return Err(LoaderError::UnsafePath(relative.into()));
    }
    let candidate = resolve_inside(root, relative)?;
    let mut current = root.to_path_buf();
    for part in relative.split('/') {
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if is_link(&metadata) => {
                return Err(LoaderError::UnsafePath(relative.into()))
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(candidate)
}

fn regular(root: &Path, relative: &str) -> Result<PathBuf> {
    let path = safe_path(root, relative)?;
    if !path.is_file() {
        return Err(invalid(format!("Dosya eksik: {}", path.display())));
    }
    Ok(path)
}

fn bounded_json<T: DeserializeOwned>(root: &Path, relative: &str) -> Result<T> {
    let path = regular(root, relative)?;
    if path.metadata()?.len() > 1024 * 1024 {
        return Err(invalid("Shadows kaydi cok buyuk"));
    }
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(
        bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes),
    )?)
}

fn geometry(path: &Path) -> Result<(u64, u64, u64)> {
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    let mut magic = [0; 8];
    file.read_exact(&mut magic)?;
    if &magic != b"scimitar" || length < 0x480 {
        return Err(invalid("Desteklenmeyen FORGE basligi"));
    }
    file.seek(SeekFrom::Start(0x41a))?;
    let mut count = [0; 4];
    file.read_exact(&mut count)?;
    let count = u32::from_le_bytes(count) as u64;
    let prefix = 0x462 + count * 24;
    if count == 0 || count > 20_000_000 || prefix > length || prefix > 512 * 1024 * 1024 {
        return Err(invalid("Desteklenmeyen FORGE indeks araligi"));
    }
    Ok((length, prefix, count))
}

pub fn disk_bytes(game: &Path) -> Result<u64> {
    let mut bytes = 2 * 1024 * 1024 * 1024;
    for (_, path) in TARGETS {
        let (_, prefix, _) = geometry(&regular(game, path)?)?;
        // Game index backup, Loader mirror, independent recovery mirror.
        bytes += prefix * 3;
    }
    Ok(bytes)
}

#[derive(Debug, Serialize, Deserialize)]
struct Transaction {
    schema: u16,
    transaction: String,
    game_root: String,
    mirror_ready: bool,
    may_have_written: bool,
    journal_sha256: Option<String>,
}
#[derive(Deserialize)]
struct Owner {
    transaction: String,
}
#[derive(Debug, Deserialize, PartialEq, Eq)]
struct UndoFile {
    path: String,
    group: String,
    original_length: u64,
    prefix_length: u64,
    entry_count: u64,
    prefix_sha256: String,
    original_sha256: String,
    original_suffix_sha256: String,
    patched_sha256: String,
    backup: String,
}
#[derive(Deserialize)]
struct UndoJournal {
    schema_version: u16,
    loader_transaction: String,
    edition: String,
    engine: String,
    status: String,
    files: Vec<UndoFile>,
}

fn sha(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn transaction(game: &Path, backup: &Path, change: &ChangeRecord) -> Result<Transaction> {
    if change.kind != ChangeKind::ShadowsPatch
        || change.path != TARGETS[0].1
        || change.backup_path.as_deref() != Some(TRANSACTION)
        || change.secondary_path.is_some()
    {
        return Err(invalid("Shadows Loader kaydi uyusmuyor"));
    }
    let path = regular(backup, TRANSACTION)?;
    if let Some(hash) = &change.installed_sha256 {
        if hash_file(&path)? != *hash {
            return Err(invalid("Shadows Loader kaydi bozuk"));
        }
    }
    let value: Transaction = bounded_json(backup, TRANSACTION)?;
    if value.schema != 1
        || uuid::Uuid::parse_str(&value.transaction).is_err()
        || backup.file_name().and_then(|n| n.to_str()) != Some(value.transaction.as_str())
        || game.canonicalize()? != PathBuf::from(&value.game_root)
    {
        return Err(invalid("Shadows kurulum kimligi/oyun klasoru uyusmuyor"));
    }
    Ok(value)
}

fn undo(root: &Path, tx: &Transaction) -> Result<UndoJournal> {
    let owner: Owner = bounded_json(root, "loader-owner.json")?;
    let journal: UndoJournal = bounded_json(root, "install.json")?;
    if owner.transaction != tx.transaction
        || journal.loader_transaction != tx.transaction
        || journal.schema_version != 2
        || journal.edition != "ORIJINAL"
        || journal.engine != "bounded-index-append-v1"
        || journal.files.len() != TARGETS.len()
        || ![
            "prepared",
            "applying",
            "installed",
            "rolled_back",
            "uninstalling",
            "uninstalled",
        ]
        .contains(&journal.status.as_str())
    {
        return Err(invalid("Shadows geri alma kaydi uyusmuyor"));
    }
    let mut paths = HashSet::new();
    for file in &journal.files {
        if !TARGETS
            .iter()
            .any(|(group, path)| file.group == *group && file.path == *path)
            || !paths.insert(file.path.clone())
            || file.backup != format!("index/{}.bin", file.group)
            || file.entry_count == 0
            || file.entry_count > 20_000_000
            || file.prefix_length != 0x462 + file.entry_count * 24
            || file.prefix_length > 512 * 1024 * 1024
            || file.original_length < file.prefix_length
            || !sha(&file.prefix_sha256)
            || !sha(&file.original_sha256)
            || !sha(&file.original_suffix_sha256)
            || (!file.patched_sha256.is_empty() && !sha(&file.patched_sha256))
            || (journal.status == "installed" && !sha(&file.patched_sha256))
        {
            return Err(invalid("Shadows geri alma dosya profili bozuk"));
        }
        let saved = regular(root, &file.backup)?;
        if saved.metadata()?.len() != file.prefix_length || hash_file(&saved)? != file.prefix_sha256
        {
            return Err(invalid(format!(
                "Shadows indeks yedegi bozuk: {}",
                file.path
            )));
        }
    }
    Ok(journal)
}

fn saved_undo(backup: &Path, tx: &Transaction) -> Result<UndoJournal> {
    let root = safe_path(backup, "shadows/undo")?;
    let hash = tx
        .journal_sha256
        .as_deref()
        .filter(|s| sha(s))
        .ok_or_else(|| invalid("Shadows yedek hash eksik"))?;
    if hash_file(&regular(&root, "install.json")?)? != hash {
        return Err(invalid("Shadows geri alma kaydi degismis"));
    }
    undo(&root, tx)
}

fn assert_stopped() -> Result<()> {
    let system = sysinfo::System::new_all();
    if system.processes().values().any(|p| {
        p.name()
            .to_string_lossy()
            .eq_ignore_ascii_case("ACShadows.exe")
    }) {
        return Err(LoaderError::Conflict(
            "Assassin's Creed Shadows acik; oyunu kapatin".into(),
        ));
    }
    Ok(())
}

#[cfg(windows)]
mod job {
    use super::*;
    use std::{ffi::c_void, os::windows::io::AsRawHandle};
    #[repr(C)]
    #[derive(Default)]
    struct Basic {
        process_time: i64,
        job_time: i64,
        flags: u32,
        min: usize,
        max: usize,
        active: u32,
        affinity: usize,
        priority: u32,
        scheduling: u32,
    }
    #[repr(C)]
    #[derive(Default)]
    struct Limits {
        basic: Basic,
        io: [u64; 6],
        process_memory: usize,
        job_memory: usize,
        peak_process: usize,
        peak_job: usize,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn CreateJobObjectW(attributes: *const c_void, name: *const u16) -> *mut c_void;
        fn SetInformationJobObject(
            job: *mut c_void,
            class: i32,
            info: *const c_void,
            size: u32,
        ) -> i32;
        fn AssignProcessToJobObject(job: *mut c_void, process: *mut c_void) -> i32;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }
    pub struct ChildJob(*mut c_void);
    impl Drop for ChildJob {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
    impl ChildJob {
        pub fn attach(child: &mut std::process::Child) -> Result<Self> {
            let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
            if handle.is_null() {
                let error = std::io::Error::last_os_error();
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.into());
            }
            let guard = Self(handle);
            let mut limits = Limits::default();
            limits.basic.flags = 0x2000; // JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            let ok = unsafe {
                SetInformationJobObject(
                    handle,
                    9,
                    &limits as *const _ as *const c_void,
                    std::mem::size_of::<Limits>() as u32,
                ) != 0
                    && AssignProcessToJobObject(handle, child.as_raw_handle()) != 0
            };
            if !ok {
                let error = std::io::Error::last_os_error();
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.into());
            }
            Ok(guard)
        }
    }
}

fn run_script(
    game: &Path,
    backup: &Path,
    tx: &Transaction,
    package: Option<&Path>,
    progress: &mut impl FnMut(u8, &str),
    may_have_written: &mut bool,
) -> Result<()> {
    *may_have_written = false;
    #[cfg(not(windows))]
    {
        let _ = (game, backup, tx, package, progress, INSTALL, UNINSTALL);
        Err(invalid("Shadows kurulumu Windows gerektirir"))
    }
    #[cfg(windows)]
    {
        use std::{
            collections::VecDeque,
            io::{BufRead, BufReader},
            os::windows::process::CommandExt,
            process::{Command, Stdio},
        };
        let scripts = tempfile::Builder::new()
            .prefix("shadows-")
            .tempdir_in(backup)?;
        let script = scripts.path().join("loader-shadows.ps1");
        fs::write(
            &script,
            if package.is_some() {
                INSTALL
            } else {
                UNINSTALL
            },
        )?;
        let root =
            std::env::var_os("SystemRoot").ok_or_else(|| invalid("Windows SystemRoot eksik"))?;
        let powershell = PathBuf::from(root).join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let log = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(backup.join("shadows-process.log"))?;
        let mut command = Command::new(powershell);
        command
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(script)
            .arg("-GameRoot")
            .arg(game)
            .arg("-LoaderTransaction")
            .arg(&tx.transaction)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(log))
            .creation_flags(0x08000000);
        if let Some(package) = package {
            command.arg("-PackageRoot").arg(package).arg("-Fast");
        }
        let mut child = command.spawn()?;
        *may_have_written = true;
        let _job = job::ChildJob::attach(&mut child)?;
        let mut tail = VecDeque::new();
        for line in BufReader::new(
            child
                .stdout
                .take()
                .ok_or_else(|| invalid("Shadows cikti kanali eksik"))?,
        )
        .lines()
        {
            let line = line?;
            if line == "[ANIMUS_NO_GAME_WRITES]" {
                *may_have_written = false;
            }
            if let Some((percent, message)) = line
                .strip_prefix("[ANIMUS_PROGRESS]")
                .and_then(|s| s.split_once('|'))
            {
                if let Ok(percent) = percent.parse::<u8>() {
                    progress(percent.min(100), message);
                }
            }
            if tail.len() == 12 {
                tail.pop_front();
            }
            tail.push_back(line);
        }
        let status = child.wait()?;
        if !status.success() {
            return Err(invalid(format!(
                "Shadows islemi durdu ({status}): {}. Gunluk: {}",
                tail.into_iter().collect::<Vec<_>>().join(" | "),
                backup.join("shadows-process.log").display()
            )));
        }
        Ok(())
    }
}

fn durable_copy(source: &Path, destination: &Path) -> Result<()> {
    copy_file(source, destination)?;
    // FlushFileBuffers on Windows requires a writable handle.
    fs::OpenOptions::new()
        .write(true)
        .open(destination)?
        .sync_all()?;
    let root = crate::storage::data_root()?.join("backups");
    if let Ok(relative) = destination.strip_prefix(root) {
        let recovery = crate::storage::recovery_backups_root()?.join(relative);
        fs::OpenOptions::new()
            .write(true)
            .open(recovery)?
            .sync_all()?;
    }
    Ok(())
}

fn persist(installation: &Installation, backup: &Path, registration: &Path) -> Result<()> {
    write_json_atomic(&backup.join("journal.json"), installation)?;
    write_json_atomic(&backup.join("metadata.json"), installation)?;
    write_json_atomic(registration, installation)
}

fn validate_package(package: &Path) -> Result<()> {
    let manifest = regular(package, "MANIFEST_SHA256.txt")?;
    if hash_file(&manifest)? != MANIFEST_SHA256 {
        return Err(invalid("Desteklenmeyen Shadows paket manifesti"));
    }
    let mut count = 0;
    let mut paths = HashSet::new();
    for line in fs::read_to_string(manifest)?.lines() {
        let (hash, path) = line
            .split_once("  ")
            .ok_or_else(|| invalid("Bozuk Shadows paket manifesti"))?;
        if !sha(hash)
            || !path.starts_with("data/")
            || !paths.insert(path.to_string())
            || hash_file(&regular(package, path)?)? != hash
        {
            return Err(invalid(format!("Shadows paket butunlugu bozuk: {path}")));
        }
        count += 1;
    }
    if count != 1314 {
        return Err(invalid("Shadows paketinde 1314 kaynak dosyasi gerekli"));
    }
    Ok(())
}

pub fn install(
    action: &Action,
    package: &Path,
    game: &Path,
    backup: &Path,
    installation: &mut Installation,
    registration: &Path,
    mut progress: impl FnMut(u8, &str),
) -> Result<()> {
    validate_action(action)?;
    validate_package(package)?;
    disk_bytes(game)?;
    assert_stopped()?;
    let external = safe_path(game, EXTERNAL)?;
    if fs::symlink_metadata(&external).is_ok() {
        return Err(LoaderError::Conflict(
            "Mevcut Shadows yedegi var; once onceki yamayi kaldirin. Yedegi silmeyin.".into(),
        ));
    }
    fs::create_dir_all(backup.join("shadows"))?;
    let mut tx = Transaction {
        schema: 1,
        transaction: installation.backup_id.clone(),
        game_root: game.canonicalize()?.to_string_lossy().into_owned(),
        mirror_ready: false,
        may_have_written: true,
        journal_sha256: None,
    };
    write_json_atomic(&backup.join(TRANSACTION), &tx)?;
    installation.changes.push(ChangeRecord {
        kind: ChangeKind::ShadowsPatch,
        path: TARGETS[0].1.into(),
        secondary_path: None,
        backup_path: Some(TRANSACTION.into()),
        original_sha256: None,
        installed_sha256: None,
    });
    // Register a pending transaction before any helper can write game data.
    persist(installation, backup, registration)?;
    let mut may_have_written = true;
    if let Err(error) = run_script(
        game,
        backup,
        &tx,
        Some(package),
        &mut progress,
        &mut may_have_written,
    ) {
        tx.may_have_written = may_have_written;
        write_json_atomic(&backup.join(TRANSACTION), &tx)?;
        return Err(error);
    }
    let journal = undo(&external, &tx)?;
    if journal.status != "installed" {
        return Err(invalid("Shadows kurulumu tamamlanmadi"));
    }
    let mirror = backup.join("shadows/undo");
    fs::create_dir_all(mirror.join("index"))?;
    for relative in ["loader-owner.json", "install.json"]
        .into_iter()
        .chain(journal.files.iter().map(|f| f.backup.as_str()))
    {
        let target = mirror.join(relative);
        durable_copy(&regular(&external, relative)?, &target)?;
    }
    tx.mirror_ready = true;
    tx.journal_sha256 = Some(hash_file(&mirror.join("install.json"))?);
    write_json_atomic(&backup.join(TRANSACTION), &tx)?;
    installation
        .changes
        .last_mut()
        .ok_or_else(|| invalid("Shadows kurulum kaydi eksik"))?
        .installed_sha256 = Some(hash_file(&backup.join(TRANSACTION))?);
    persist(installation, backup, registration)?;
    Ok(())
}

pub fn verify(game: &Path, backup: &Path, change: &ChangeRecord) -> Result<bool> {
    let tx = transaction(game, backup, change)?;
    if !tx.mirror_ready {
        return Ok(false);
    }
    let journal = saved_undo(backup, &tx)?;
    for file in journal.files {
        let live = regular(game, &file.path)?;
        if hash_file(&live)? != file.patched_sha256 {
            return Ok(false);
        }
    }
    Ok(true)
}

fn archive_incomplete(game: &Path, external: &Path) -> Result<()> {
    fs::rename(
        external,
        game.join(format!(".animus-shadows-history-{}", uuid::Uuid::new_v4())),
    )?;
    Ok(())
}

pub fn restore(game: &Path, backup: &Path, change: &ChangeRecord) -> Result<()> {
    assert_stopped()?;
    let tx = transaction(game, backup, change)?;
    let saved = if tx.mirror_ready {
        Some(saved_undo(backup, &tx)?)
    } else {
        None
    };
    let external = safe_path(game, EXTERNAL)?;
    if !external.exists() {
        let Some(journal) = &saved else {
            if !tx.may_have_written {
                return Ok(());
            }
            return Err(invalid("Bekleyen Shadows kurulumunun oyun yedegi eksik; guvenli geri alma dogrulanamadi. Kayit korundu."));
        };
        let mut original = true;
        for file in &journal.files {
            original &= hash_file(&regular(game, &file.path)?)? == file.original_sha256;
        }
        if original {
            return Ok(());
        }
        fs::create_dir_all(external.join("index"))?;
        let mirror = backup.join("shadows/undo");
        for relative in ["loader-owner.json", "install.json"]
            .into_iter()
            .chain(journal.files.iter().map(|f| f.backup.as_str()))
        {
            durable_copy(&regular(&mirror, relative)?, &external.join(relative))?;
        }
    }
    let owner_path = safe_path(&external, "loader-owner.json")?;
    if !owner_path.exists() {
        // A kill between directory creation and the first owner write cannot
        // have touched a forge. Only accept an empty directory tree here.
        for entry in walkdir::WalkDir::new(&external) {
            let entry = entry.map_err(|e| invalid(e.to_string()))?;
            if !entry.file_type().is_dir()
                || is_link(&entry.metadata().map_err(|e| invalid(e.to_string()))?)
            {
                return Err(invalid("Shadows yedek klasorunun sahibi dogrulanamadi"));
            }
        }
        return archive_incomplete(game, &external);
    }
    let owner: Owner = bounded_json(&external, "loader-owner.json")?;
    if owner.transaction != tx.transaction {
        return Err(invalid("Shadows yedegi baska kuruluma ait"));
    }
    if !safe_path(&external, "install.json")?.exists() {
        // The installer only starts modifying forges after the complete journal
        // has been atomically installed. Keep unfinished index copies as history.
        if tx.mirror_ready {
            return Err(invalid("Shadows etkin geri alma kaydi eksik"));
        }
        return archive_incomplete(game, &external);
    }
    let journal = undo(&external, &tx)?;
    if let Some(saved) = saved {
        if journal.files != saved.files {
            return Err(invalid("Shadows etkin yedegi Loader kopyasiyla uyusmuyor"));
        }
    }
    // All five bodies and snapshots are preflighted by the embedded uninstaller
    // before its first restore write. Force never bypasses those checks.
    run_script(game, backup, &tx, None, &mut |_, _| {}, &mut true)?;
    for file in &journal.files {
        if hash_file(&regular(game, &file.path)?)? != file.original_sha256 {
            return Err(invalid("Shadows orijinal dosya hash geri alinamadi"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ActionType;
    #[test]
    fn rejects_renamed_and_unpinned_actions() {
        let mut action = Action {
            id: uuid::Uuid::new_v4().to_string(),
            kind: ActionType::InstallShadows,
            source: Some("MANIFEST_SHA256.txt".into()),
            destination: TARGETS[0].1.into(),
            backup: true,
            expected_sha256: Some(MANIFEST_SHA256.into()),
            options: Default::default(),
        };
        assert!(validate_action(&action).is_ok());
        action.expected_sha256 = Some("0".repeat(64));
        assert!(validate_action(&action).is_err());
        action.expected_sha256 = Some(MANIFEST_SHA256.into());
        action.destination = "another.forge".into();
        assert!(validate_action(&action).is_err());
    }
    #[test]
    fn forge_preflight_rejects_bad_geometry_before_writes() {
        let temp = tempfile::tempdir().unwrap();
        for (_, relative) in TARGETS {
            let path = temp.path().join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut bytes = vec![0; 4096];
            bytes[..8].copy_from_slice(b"scimitar");
            bytes[0x41a..0x41e].copy_from_slice(&3u32.to_le_bytes());
            fs::write(path, bytes).unwrap();
        }
        assert!(disk_bytes(temp.path()).is_ok());
        let path = temp.path().join(TARGETS[0].1);
        let mut bytes = fs::read(&path).unwrap();
        bytes[0x41a..0x41e].copy_from_slice(&u32::MAX.to_le_bytes());
        fs::write(&path, &bytes).unwrap();
        assert!(disk_bytes(temp.path()).is_err());
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    #[cfg(unix)]
    #[test]
    fn refuses_dangling_backup_link() {
        let temp = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(temp.path().join("missing"), temp.path().join(EXTERNAL))
            .unwrap();
        assert!(safe_path(temp.path(), EXTERNAL).is_err());
    }

    #[test]
    fn missing_pending_backup_never_claims_a_restore() {
        let temp = tempfile::tempdir().unwrap();
        let game = temp.path().join("game");
        fs::create_dir(&game).unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let backup = temp.path().join(&id);
        fs::create_dir(&backup).unwrap();
        let untouched = game.join(TARGETS[0].1);
        fs::write(&untouched, b"original game").unwrap();
        let mut tx = Transaction {
            schema: 1,
            transaction: id,
            game_root: game.canonicalize().unwrap().to_string_lossy().into_owned(),
            mirror_ready: false,
            may_have_written: true,
            journal_sha256: None,
        };
        write_json_atomic(&backup.join(TRANSACTION), &tx).unwrap();
        let change = ChangeRecord {
            kind: ChangeKind::ShadowsPatch,
            path: TARGETS[0].1.into(),
            secondary_path: None,
            backup_path: Some(TRANSACTION.into()),
            original_sha256: None,
            installed_sha256: None,
        };
        assert!(restore(&game, &backup, &change).is_err());
        assert_eq!(fs::read(&untouched).unwrap(), b"original game");
        // The child explicitly confirmed a failure before any forge write.
        tx.may_have_written = false;
        write_json_atomic(&backup.join(TRANSACTION), &tx).unwrap();
        restore(&game, &backup, &change).unwrap();
        assert_eq!(fs::read(untouched).unwrap(), b"original game");
    }

    #[test]
    fn owned_incomplete_snapshots_are_retained_as_history() {
        let temp = tempfile::tempdir().unwrap();
        let game = temp.path().join("game");
        fs::create_dir(&game).unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let backup = temp.path().join(&id);
        fs::create_dir(&backup).unwrap();
        let tx = Transaction {
            schema: 1,
            transaction: id.clone(),
            game_root: game.canonicalize().unwrap().to_string_lossy().into_owned(),
            mirror_ready: false,
            may_have_written: true,
            journal_sha256: None,
        };
        write_json_atomic(&backup.join(TRANSACTION), &tx).unwrap();
        let external = game.join(EXTERNAL);
        fs::create_dir_all(external.join("index")).unwrap();
        write_json_atomic(
            &external.join("loader-owner.json"),
            &serde_json::json!({"transaction": id}),
        )
        .unwrap();
        fs::write(external.join("index/unfinished.bin"), b"partial snapshot").unwrap();
        let change = ChangeRecord {
            kind: ChangeKind::ShadowsPatch,
            path: TARGETS[0].1.into(),
            secondary_path: None,
            backup_path: Some(TRANSACTION.into()),
            original_sha256: None,
            installed_sha256: None,
        };
        restore(&game, &backup, &change).unwrap();
        assert!(!external.exists());
        let history = fs::read_dir(game).unwrap().next().unwrap().unwrap().path();
        assert_eq!(
            fs::read(history.join("index/unfinished.bin")).unwrap(),
            b"partial snapshot"
        );
    }
}
