//! Native FAT2 v10 update. Only the new, uncompressed entry is downloaded.
//! A durable undo record is registered BEFORE either game file is changed.
//! The same recovery material is also mirrored inside the game root so a
//! deleted LocalAppData backup/registration can be rebuilt without touching
//! unrelated game data.
use crate::{
    error::{LoaderError, Result},
    models::{Action, ChangeKind, ChangeRecord, Installation},
    security::path::resolve_inside,
    storage::{hash_file, read_json, write_json_atomic},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

const MAX_PAYLOAD: u64 = 128 * 1024 * 1024;
const MAX_FAT: u64 = 64 * 1024 * 1024;
const MAX_RECOVERY_JSON: u64 = 1024 * 1024;
const RECOVERY_SCHEMA: u16 = 1;
const RECOVERY_DIR: &str = ".animus-recovery";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Options {
    pub fat_path: String,
    fat_entry_hash: String,
    base_dat_sha256: String,
    base_fat_sha256: String,
    payload_sha256: String,
    alignment: u64,
    compression: String,
}

fn need(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(LoaderError::Integrity(message.into()))
    }
}
fn digest(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}
fn is_hash(s: &str, n: usize) -> bool {
    s.len() == n && s.bytes().all(|c| c.is_ascii_hexdigit())
}

fn relative(s: &str) -> Result<()> {
    need(
        !s.is_empty()
            && !s.contains([':', '\\', '\0'])
            && !s.starts_with('/')
            && s.split('/')
                .all(|p| !p.is_empty() && p != "." && p != ".." && !p.ends_with([' ', '.'])),
        "FAT/DAT yolu güvenli relative yol olmalıdır",
    )
}
pub fn options(a: &Action) -> Result<Options> {
    let o: Options = serde_json::from_value(serde_json::to_value(&a.options)?)?;
    relative(&a.destination)?;
    relative(&o.fat_path)?;
    relative(a.source.as_deref().unwrap_or(""))?;
    need(
        a.backup && o.alignment == 8 && o.compression == "none",
        "FAT/DAT seçenekleri geçersiz",
    )?;
    need(
        a.destination.ends_with(".dat")
            && o.fat_path.ends_with(".fat")
            && Path::new(&a.destination).with_extension("fat") == Path::new(&o.fat_path),
        "Eşleşen DAT/FAT çifti gerekli",
    )?;
    need(
        is_hash(&o.fat_entry_hash, 16)
            && [&o.base_dat_sha256, &o.base_fat_sha256, &o.payload_sha256]
                .iter()
                .all(|s| is_hash(s, 64)),
        "FAT/DAT SHA-256 veya kayıt kimliği geçersiz",
    )?;
    Ok(o)
}

fn regular(root: &Path, rel: &str) -> Result<PathBuf> {
    relative(rel)?;
    let path = resolve_inside(root, rel)?;
    let mut part = root.canonicalize()?;
    for component in rel.split('/') {
        part.push(component);
        let m = fs::symlink_metadata(&part)?;
        need(
            !m.file_type().is_symlink(),
            "FAT/DAT sembolik bağlantı olamaz",
        )?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            need(
                m.file_attributes() & 0x400 == 0,
                "FAT/DAT reparse bağlantısı olamaz",
            )?;
        }
    }
    need(path.is_file(), "FAT/DAT dosyası bulunamadı")?;
    Ok(path)
}
fn open_dat(path: &Path) -> Result<File> {
    let mut o = OpenOptions::new();
    o.read(true).write(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        o.share_mode(0);
    }
    Ok(o.open(path)?)
}
fn safe_file(path: &Path) -> Result<()> {
    let m = fs::symlink_metadata(path)?;
    need(
        m.is_file(),
        "FAT/DAT kurtarma dosyası normal dosya olmalıdır",
    )?;
    need(
        !m.file_type().is_symlink(),
        "FAT/DAT kurtarma dosyası sembolik bağlantı olamaz",
    )?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        need(
            m.file_attributes() & 0x400 == 0,
            "FAT/DAT kurtarma dosyası reparse bağlantısı olamaz",
        )?;
    }
    Ok(())
}
fn bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let f = File::open(path)?;
    need(
        f.metadata()?.len() <= limit,
        "FAT/DAT dosya boyutu sınırı aşıldı",
    )?;
    let mut bytes = Vec::new();
    f.take(limit + 1).read_to_end(&mut bytes)?;
    need(
        bytes.len() as u64 <= limit,
        "FAT/DAT dosya boyutu sınırı aşıldı",
    )?;
    Ok(bytes)
}
fn bounded_safe(path: &Path, limit: u64) -> Result<Vec<u8>> {
    safe_file(path)?;
    bounded(path, limit)
}
fn u32_at(b: &[u8], p: usize) -> u32 {
    u32::from_le_bytes(b[p..p + 4].try_into().unwrap())
}
fn put(b: &mut [u8], p: usize, value: u32) {
    b[p..p + 4].copy_from_slice(&value.to_le_bytes());
}

fn parse(fat: &[u8], dat_len: u64, target: u64) -> Result<usize> {
    need(
        fat.len() >= 32 && u32_at(fat, 0) == 0x46415432 && u32_at(fat, 4) == 10,
        "FAT2 v10 gerekli",
    )?;
    let count = u32_at(fat, 20) as usize;
    need(
        count > 1 && count <= (fat.len() - 24) / 20,
        "Tam FAT arşivi gerekli; mini FAT kabul edilmez",
    )?;
    let mut found = None;
    for i in 0..count {
        let p = 24 + 20 * i;
        let u = u32_at(fat, p + 8);
        let o = u32_at(fat, p + 12);
        let c = u32_at(fat, p + 16);
        let offset = (o as u64) * 8 + (c >> 29) as u64;
        let size = if u & 3 == 0 { u >> 2 } else { c & 0x1fffffff } as u64;
        need(
            offset <= dat_len && size <= dat_len - offset,
            "FAT/DAT kayıt sınırı hatası",
        )?;
        if u64::from_le_bytes(fat[p..p + 8].try_into().unwrap()) == target {
            need(found.is_none(), "FAT hedef kaydı birden fazla")?;
            found = Some(p);
        }
    }
    found.ok_or_else(|| LoaderError::Integrity("FAT hedef kaydı bulunamadı".into()))
}

fn hash_prefix(file: &mut File, length: u64) -> Result<Sha256> {
    file.seek(SeekFrom::Start(0))?;
    let mut h = Sha256::new();
    let mut remaining = length;
    let mut b = vec![0; 1024 * 1024];
    while remaining > 0 {
        let want = remaining.min(b.len() as u64) as usize;
        file.read_exact(&mut b[..want])?;
        h.update(&b[..want]);
        remaining -= want as u64;
    }
    Ok(h)
}
fn save_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut f = OpenOptions::new().write(true).create_new(true).open(path)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    Ok(())
}
fn replace_local(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if path.exists() {
        let m = fs::symlink_metadata(path)?;
        need(
            m.is_file() && !m.file_type().is_symlink(),
            "FAT/DAT kurtarma hedefi normal dosya olmalıdır",
        )?;
        fs::remove_file(path)?;
    }
    save_new(path, bytes)
}
fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut temp = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    temp.persist(path).map_err(|e| LoaderError::Io(e.error))?;
    Ok(())
}

#[derive(Clone, Serialize, Deserialize)]
struct Undo {
    process_name: Option<String>,
    dat: String,
    fat: String,
    original_len: u64,
    offset: u64,
    target: u64,
    original_dat_hash: String,
    original_fat_hash: String,
    final_dat_hash: String,
    final_fat_hash: String,
    payload_hash: String,
    original_fat_file: String,
    payload_file: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct RecoveryCapsule {
    schema_version: u16,
    installation: Installation,
    undo: Undo,
}

struct RecoveryPaths {
    dir: PathBuf,
    meta: PathBuf,
    original_fat: PathBuf,
    payload: PathBuf,
}

struct RecoveryMaterial {
    capsule: RecoveryCapsule,
    original_fat: Vec<u8>,
    payload: Vec<u8>,
    paths: RecoveryPaths,
}

fn recovery_dir(game: &Path, create: bool) -> Result<Option<PathBuf>> {
    let dir = game.join(RECOVERY_DIR);
    if !dir.exists() {
        if !create {
            return Ok(None);
        }
        fs::create_dir(&dir)?;
    }
    let m = fs::symlink_metadata(&dir)?;
    need(m.is_dir(), "Animus kurtarma yolu klasör olmalıdır")?;
    need(
        !m.file_type().is_symlink(),
        "Animus kurtarma klasörü sembolik bağlantı olamaz",
    )?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        need(
            m.file_attributes() & 0x400 == 0,
            "Animus kurtarma klasörü reparse bağlantısı olamaz",
        )?;
    }
    Ok(Some(dir))
}
fn recovery_paths(game: &Path, target: u64, create: bool) -> Result<Option<RecoveryPaths>> {
    let Some(dir) = recovery_dir(game, create)? else {
        return Ok(None);
    };
    let stem = format!("fat-dat-{target:016x}");
    Ok(Some(RecoveryPaths {
        meta: dir.join(format!("{stem}.json")),
        original_fat: dir.join(format!("{stem}.original-fat.bin")),
        payload: dir.join(format!("{stem}.payload.bin")),
        dir,
    }))
}
fn cleanup_recovery(paths: &RecoveryPaths) {
    for path in [&paths.meta, &paths.original_fat, &paths.payload] {
        let _ = fs::remove_file(path);
    }
    let _ = fs::remove_dir(&paths.dir);
}
fn validate_capsule_for_change(c: &RecoveryCapsule, change: &ChangeRecord) -> Result<()> {
    need(
        c.schema_version == RECOVERY_SCHEMA,
        "FAT/DAT kurtarma şeması desteklenmiyor",
    )?;
    need(
        c.undo.dat == change.path
            && Some(&c.undo.fat) == change.secondary_path.as_ref()
            && Some(&c.undo.original_dat_hash) == change.original_sha256.as_ref()
            && Some(&c.undo.final_dat_hash) == change.installed_sha256.as_ref(),
        "FAT/DAT kurtarma kimliği farklı",
    )
}
fn load_recovery_paths(paths: RecoveryPaths) -> Result<RecoveryMaterial> {
    safe_file(&paths.meta)?;
    need(
        fs::metadata(&paths.meta)?.len() <= MAX_RECOVERY_JSON,
        "FAT/DAT kurtarma kaydı çok büyük",
    )?;
    let capsule: RecoveryCapsule = read_json(&paths.meta)?;
    need(
        capsule.schema_version == RECOVERY_SCHEMA,
        "FAT/DAT kurtarma şeması desteklenmiyor",
    )?;
    let original_fat = bounded_safe(&paths.original_fat, MAX_FAT)?;
    let payload = bounded_safe(&paths.payload, MAX_PAYLOAD)?;
    need(
        digest(&original_fat).eq_ignore_ascii_case(&capsule.undo.original_fat_hash)
            && digest(&payload).eq_ignore_ascii_case(&capsule.undo.payload_hash),
        "FAT/DAT oyun içi kurtarma kopyası bozuk",
    )?;
    Ok(RecoveryMaterial {
        capsule,
        original_fat,
        payload,
        paths,
    })
}
fn load_recovery_for_action(a: &Action, game: &Path) -> Result<Option<RecoveryMaterial>> {
    let o = options(a)?;
    let target = u64::from_str_radix(&o.fat_entry_hash, 16).unwrap();
    let Some(paths) = recovery_paths(game, target, false)? else {
        return Ok(None);
    };
    if !paths.meta.is_file() {
        return Ok(None);
    }
    let r = load_recovery_paths(paths)?;
    need(
        r.capsule.undo.target == target
            && r.capsule.undo.dat == a.destination
            && r.capsule.undo.fat == o.fat_path,
        "FAT/DAT kurtarma kaydı bu action'a ait değil",
    )?;
    Ok(Some(r))
}
fn load_recovery_for_change(game: &Path, change: &ChangeRecord) -> Result<Option<RecoveryMaterial>> {
    let Some(dir) = recovery_dir(game, false)? else {
        return Ok(None);
    };
    let mut found = None;
    for entry in fs::read_dir(&dir)? {
        let path = entry?.path();
        if path.extension().and_then(|v| v.to_str()) != Some("json") {
            continue;
        }
        safe_file(&path)?;
        need(
            fs::metadata(&path)?.len() <= MAX_RECOVERY_JSON,
            "FAT/DAT kurtarma kaydı çok büyük",
        )?;
        let capsule: RecoveryCapsule = read_json(&path)?;
        if capsule.undo.dat != change.path
            || Some(&capsule.undo.fat) != change.secondary_path.as_ref()
        {
            continue;
        }
        validate_capsule_for_change(&capsule, change)?;
        need(found.is_none(), "Birden fazla FAT/DAT kurtarma kaydı bulundu")?;
        let paths = recovery_paths(game, capsule.undo.target, false)?
            .ok_or_else(|| LoaderError::Integrity("FAT/DAT kurtarma klasörü kayıp".into()))?;
        need(paths.meta == path, "FAT/DAT kurtarma dosya adı geçersiz")?;
        found = Some(load_recovery_paths(paths)?);
    }
    Ok(found)
}
fn write_recovery(
    game: &Path,
    installation: &Installation,
    undo: &Undo,
    original_fat: &[u8],
    payload: &[u8],
) -> Result<()> {
    let paths = recovery_paths(game, undo.target, true)?
        .ok_or_else(|| LoaderError::Integrity("Animus kurtarma klasörü oluşturulamadı".into()))?;
    if paths.meta.exists() || paths.original_fat.exists() || paths.payload.exists() {
        cleanup_recovery(&paths);
        fs::create_dir_all(&paths.dir)?;
    }
    let result = (|| -> Result<()> {
        save_new(&paths.original_fat, original_fat)?;
        save_new(&paths.payload, payload)?;
        let capsule = RecoveryCapsule {
            schema_version: RECOVERY_SCHEMA,
            installation: installation.clone(),
            undo: undo.clone(),
        };
        save_new(&paths.meta, &serde_json::to_vec(&capsule)?)?;
        Ok(())
    })();
    if result.is_err() {
        cleanup_recovery(&paths);
    }
    result
}
fn write_backup_from_recovery(
    backup: &Path,
    recovery: &RecoveryMaterial,
    installation: &Installation,
) -> Result<()> {
    let expected_id = backup
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or_default();
    need(
        expected_id == installation.backup_id,
        "FAT/DAT backup kimliği eşleşmiyor",
    )?;
    fs::create_dir_all(backup.join("files"))?;
    let undo = &recovery.capsule.undo;
    replace_local(
        &backup.join(&undo.original_fat_file),
        &recovery.original_fat,
    )?;
    replace_local(&backup.join(&undo.payload_file), &recovery.payload)?;
    replace_local(
        &backup.join("fat-dat-undo.json"),
        &serde_json::to_vec(undo)?,
    )?;
    if backup.join("journal.json").exists() {
        fs::remove_file(backup.join("journal.json"))?;
    }
    if backup.join("metadata.json").exists() {
        fs::remove_file(backup.join("metadata.json"))?;
    }
    write_json_atomic(&backup.join("journal.json"), installation)?;
    write_json_atomic(&backup.join("metadata.json"), installation)?;
    Ok(())
}
fn rewrite_recovery_installation(
    recovery: &RecoveryMaterial,
    installation: &Installation,
) -> Result<()> {
    let capsule = RecoveryCapsule {
        schema_version: RECOVERY_SCHEMA,
        installation: installation.clone(),
        undo: recovery.capsule.undo.clone(),
    };
    if recovery.paths.meta.exists() {
        fs::remove_file(&recovery.paths.meta)?;
    }
    save_new(&recovery.paths.meta, &serde_json::to_vec(&capsule)?)
}
fn change_from_undo(undo: &Undo) -> ChangeRecord {
    ChangeRecord {
        kind: ChangeKind::PatchedArchive,
        path: undo.dat.clone(),
        secondary_path: Some(undo.fat.clone()),
        backup_path: Some("fat-dat-undo.json".into()),
        original_sha256: Some(undo.original_dat_hash.clone()),
        installed_sha256: Some(undo.final_dat_hash.clone()),
    }
}
fn load_backup_material(
    backup: &Path,
    change: &ChangeRecord,
) -> Result<(Undo, Vec<u8>, Vec<u8>)> {
    let rel = change
        .backup_path
        .as_deref()
        .ok_or_else(|| LoaderError::Integrity("FAT/DAT yedek kaydı eksik".into()))?;
    let undo: Undo = read_json(&regular(backup, rel)?)?;
    need(
        undo.dat == change.path
            && Some(&undo.fat) == change.secondary_path.as_ref()
            && Some(&undo.original_dat_hash) == change.original_sha256.as_ref()
            && Some(&undo.final_dat_hash) == change.installed_sha256.as_ref(),
        "FAT/DAT yedek kimliği farklı",
    )?;
    let original = bounded(&regular(backup, &undo.original_fat_file)?, MAX_FAT)?;
    let payload = bounded(&regular(backup, &undo.payload_file)?, MAX_PAYLOAD)?;
    need(
        digest(&original).eq_ignore_ascii_case(&undo.original_fat_hash)
            && digest(&payload).eq_ignore_ascii_case(&undo.payload_hash),
        "FAT/DAT yedeği bozuk",
    )?;
    Ok((undo, original, payload))
}
fn load_material(
    game: &Path,
    backup: &Path,
    change: &ChangeRecord,
) -> Result<(Undo, Vec<u8>, Vec<u8>)> {
    if backup.join("metadata.json").is_file() {
        if let Ok(material) = load_backup_material(backup, change) {
            return Ok(material);
        }
    }
    if let Some(recovery) = load_recovery_for_change(game, change)? {
        let expected_id = backup
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or_default();
        need(
            expected_id == recovery.capsule.installation.backup_id,
            "FAT/DAT kurtarma backup kimliği farklı",
        )?;
        write_backup_from_recovery(backup, &recovery, &recovery.capsule.installation)?;
        return Ok((
            recovery.capsule.undo.clone(),
            recovery.original_fat,
            recovery.payload,
        ));
    }
    load_backup_material(backup, change)
}

pub fn install(
    a: &Action,
    archive: &Path,
    game: &Path,
    backup: &Path,
    installation: &mut Installation,
    registration: &Path,
    process_name: Option<&str>,
) -> Result<()> {
    let o = options(a)?;
    let source = regular(archive, a.source.as_deref().unwrap())?;
    let dat_path = regular(game, &a.destination)?;
    let fat_path = regular(game, &o.fat_path)?;
    let payload = bounded(&source, MAX_PAYLOAD)?;
    need(
        !payload.is_empty() && digest(&payload).eq_ignore_ascii_case(&o.payload_sha256),
        "Çeviri SHA-256 uyuşmuyor",
    )?;

    if let Some(recovery) = load_recovery_for_action(a, game)? {
        let current_dat = hash_file(&dat_path)?;
        let current_fat = hash_file(&fat_path)?;
        let u = recovery.capsule.undo.clone();
        if current_dat.eq_ignore_ascii_case(&u.final_dat_hash)
            && current_fat.eq_ignore_ascii_case(&u.final_fat_hash)
        {
            let expected_matches = a
                .expected_sha256
                .as_ref()
                .map(|v| v.eq_ignore_ascii_case(&u.final_dat_hash))
                .unwrap_or(true);
            need(
                u.original_dat_hash.eq_ignore_ascii_case(&o.base_dat_sha256)
                    && u.original_fat_hash.eq_ignore_ascii_case(&o.base_fat_sha256)
                    && u.payload_hash.eq_ignore_ascii_case(&o.payload_sha256)
                    && recovery.payload == payload
                    && expected_matches,
                "Farklı bir Animus FAT/DAT yaması kurulu; otomatik sahiplenme güvenli değil",
            )?;
            installation.changes.clear();
            installation.changes.push(change_from_undo(&u));
            let updated_capsule = RecoveryMaterial {
                capsule: RecoveryCapsule {
                    schema_version: RECOVERY_SCHEMA,
                    installation: installation.clone(),
                    undo: u.clone(),
                },
                original_fat: recovery.original_fat.clone(),
                payload: recovery.payload.clone(),
                paths: recovery.paths,
            };
            write_backup_from_recovery(backup, &updated_capsule, installation)?;
            rewrite_recovery_installation(&updated_capsule, installation)?;
            write_json_atomic(registration, installation)?;
            return Ok(());
        }
        if current_dat.eq_ignore_ascii_case(&u.original_dat_hash)
            && current_fat.eq_ignore_ascii_case(&u.original_fat_hash)
        {
            cleanup_recovery(&recovery.paths);
        } else {
            return Err(LoaderError::Integrity(
                "FAT/DAT kurtarma kaydı var ancak oyun dosyaları beklenen kurulu veya temiz durumda değil"
                    .into(),
            ));
        }
    }

    let fat = bounded(&fat_path, MAX_FAT)?;
    need(
        digest(&fat).eq_ignore_ascii_case(&o.base_fat_sha256),
        "Oyun FAT sürümü bu yamayla uyumlu değil",
    )?;
    let mut dat = open_dat(&dat_path)?;
    let length = dat.metadata()?.len();
    let mut base_hash = hash_prefix(&mut dat, length)?;
    need(
        hex::encode(base_hash.clone().finalize()).eq_ignore_ascii_case(&o.base_dat_sha256),
        "Oyun DAT sürümü bu yamayla uyumlu değil",
    )?;
    let target = u64::from_str_radix(&o.fat_entry_hash, 16).unwrap();
    let entry = parse(&fat, length, target)?;
    let offset = length
        .checked_add(7)
        .ok_or_else(|| LoaderError::Integrity("DAT boyut taşması".into()))?
        & !7;
    need(offset / 8 <= u32::MAX as u64, "DAT offset kapasitesi aşıldı")?;
    let pad = vec![0; (offset - length) as usize];
    let mut updated = fat.clone();
    put(&mut updated, entry + 8, (payload.len() as u32) << 2);
    put(&mut updated, entry + 12, (offset / 8) as u32);
    put(&mut updated, entry + 16, payload.len() as u32);
    parse(&updated, offset + payload.len() as u64, target)?;
    base_hash.update(&pad);
    base_hash.update(&payload);
    let final_hash = hex::encode(base_hash.finalize());
    if let Some(expected) = &a.expected_sha256 {
        need(
            final_hash.eq_ignore_ascii_case(expected),
            "Beklenen final DAT SHA-256 uyuşmuyor",
        )?;
    }
    fs::create_dir_all(backup.join("files"))?;
    let undo = Undo {
        process_name: process_name.map(str::to_owned),
        dat: a.destination.clone(),
        fat: o.fat_path.clone(),
        original_len: length,
        offset,
        target,
        original_dat_hash: o.base_dat_sha256,
        original_fat_hash: o.base_fat_sha256,
        final_dat_hash: final_hash.clone(),
        final_fat_hash: digest(&updated),
        payload_hash: o.payload_sha256,
        original_fat_file: "files/fat-original.bin".into(),
        payload_file: "files/fat-payload.bin".into(),
    };
    save_new(&backup.join(&undo.original_fat_file), &fat)?;
    save_new(&backup.join(&undo.payload_file), &payload)?;
    save_new(
        &backup.join("fat-dat-undo.json"),
        &serde_json::to_vec(&undo)?,
    )?;
    installation.changes.push(change_from_undo(&undo));
    write_recovery(game, installation, &undo, &fat, &payload)?;
    write_json_atomic(&backup.join("journal.json"), installation)?;
    write_json_atomic(&backup.join("metadata.json"), installation)?;
    write_json_atomic(registration, installation)?;
    dat.seek(SeekFrom::End(0))?;
    dat.write_all(&pad)?;
    dat.write_all(&payload)?;
    dat.sync_all()?;
    let mut extracted = vec![0; payload.len()];
    dat.seek(SeekFrom::Start(offset))?;
    dat.read_exact(&mut extracted)?;
    need(extracted == payload, "DAT içinden tekrar okunan çeviri farklı")?;
    need(
        hex::encode(hash_prefix(&mut dat, offset + payload.len() as u64)?.finalize())
            == undo.final_dat_hash,
        "Final DAT doğrulanamadı",
    )?;
    need(
        hash_file(&fat_path)?.eq_ignore_ascii_case(&undo.original_fat_hash),
        "FAT işlem sırasında değişti",
    )?;
    atomic_replace(&fat_path, &updated)?;
    need(
        bounded(&fat_path, MAX_FAT)? == updated,
        "Final FAT doğrulanamadı",
    )?;
    Ok(())
}

pub fn verify(game: &Path, backup: &Path, change: &ChangeRecord) -> Result<bool> {
    let (u, _, _) = load_material(game, backup, change)?;
    let dat = regular(game, &u.dat)?;
    let fat = regular(game, &u.fat)?;
    Ok(hash_file(&dat)?.eq_ignore_ascii_case(&u.final_dat_hash)
        && hash_file(&fat)?.eq_ignore_ascii_case(&u.final_fat_hash))
}

pub fn restore(game: &Path, backup: &Path, change: &ChangeRecord) -> Result<()> {
    let (u, original, payload) = load_material(game, backup, change)?;
    if let Some(name) = &u.process_name {
        let system = sysinfo::System::new_all();
        need(
            !system
                .processes()
                .values()
                .any(|p| p.name().to_string_lossy().eq_ignore_ascii_case(name)),
            "Geri almadan önce oyunu kapatın",
        )?;
    }
    let dat_path = regular(game, &u.dat)?;
    let fat_path = regular(game, &u.fat)?;
    need(
        digest(&original).eq_ignore_ascii_case(&u.original_fat_hash)
            && digest(&payload).eq_ignore_ascii_case(&u.payload_hash),
        "FAT/DAT yedeği bozuk",
    )?;
    need(
        u.offset >= u.original_len && u.offset - u.original_len < 8 && u.offset % 8 == 0,
        "Geri alma uzunluğu geçersiz",
    )?;
    parse(&original, u.original_len, u.target)?;
    let current_fat = hash_file(&fat_path)?;
    need(
        current_fat.eq_ignore_ascii_case(&u.original_fat_hash)
            || current_fat.eq_ignore_ascii_case(&u.final_fat_hash),
        "FAT sonradan değişmiş; geri alma durduruldu",
    )?;
    let mut dat = open_dat(&dat_path)?;
    let len = dat.metadata()?.len();
    need(
        len >= u.original_len && len <= u.offset + payload.len() as u64,
        "DAT uzunluğu sonradan değişmiş",
    )?;
    need(
        hex::encode(hash_prefix(&mut dat, u.original_len)?.finalize())
            .eq_ignore_ascii_case(&u.original_dat_hash),
        "Orijinal DAT bölgesi değişmiş; geri alma durduruldu",
    )?;
    let mut suffix = vec![0; (len - u.original_len) as usize];
    dat.read_exact(&mut suffix)?;
    let mut expected = vec![0; (u.offset - u.original_len) as usize];
    expected.extend_from_slice(&payload);
    need(
        suffix == expected[..suffix.len()],
        "DAT sonuna farklı veri eklenmiş; geri alma durduruldu",
    )?;
    if !current_fat.eq_ignore_ascii_case(&u.original_fat_hash) {
        atomic_replace(&fat_path, &original)?;
    }
    dat.set_len(u.original_len)?;
    dat.sync_all()?;
    need(
        hash_file(&fat_path)?.eq_ignore_ascii_case(&u.original_fat_hash)
            && hex::encode(hash_prefix(&mut dat, u.original_len)?.finalize())
                .eq_ignore_ascii_case(&u.original_dat_hash),
        "FAT/DAT geri alma doğrulanamadı",
    )?;
    if let Some(paths) = recovery_paths(game, u.target, false)? {
        cleanup_recovery(&paths);
    }
    Ok(())
}

pub fn disk_bytes(a: &Action, game: &Path) -> Result<u64> {
    let o = options(a)?;
    let fat = regular(game, &o.fat_path)?;
    regular(game, &a.destination)?;
    Ok(MAX_PAYLOAD * 3 + fs::metadata(fat)?.len() * 4 + 2 * 1024 * 1024)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ActionType;

    struct Fixture {
        _temp: tempfile::TempDir,
        game: PathBuf,
        archive: PathBuf,
        backup: PathBuf,
        registration: PathBuf,
        action: Action,
        installation: Installation,
        dat: Vec<u8>,
        fat: Vec<u8>,
        payload: Vec<u8>,
    }
    impl Fixture {
        fn new() -> Self {
            let temp = tempfile::tempdir().unwrap();
            let game = temp.path().join("game");
            let archive = temp.path().join("archive");
            let backup = temp.path().join("backup");
            for p in [&game, &archive, &backup] {
                fs::create_dir_all(p).unwrap();
            }
            let dat = (0..73).collect::<Vec<u8>>();
            let payload = "ÇEVİRMENLER:\nINTELPOL — ANİMUS ÇEVİRİ"
                .as_bytes()
                .to_vec();
            let mut fat = vec![0; 72];
            put(&mut fat, 0, 0x46415432);
            put(&mut fat, 4, 10);
            put(&mut fat, 20, 2);
            fat[24..32].copy_from_slice(&123u64.to_le_bytes());
            put(&mut fat, 32, 16 << 2);
            put(&mut fat, 36, 0);
            put(&mut fat, 40, 16);
            fat[44..52].copy_from_slice(&456u64.to_le_bytes());
            put(&mut fat, 52, 12 << 2 | 1);
            put(&mut fat, 56, 2);
            put(&mut fat, 60, 8);
            fs::write(game.join("patch.dat"), &dat).unwrap();
            fs::write(game.join("patch.fat"), &fat).unwrap();
            fs::write(archive.join("ceviri.bin"), &payload).unwrap();
            let action = Action {
                id: uuid::Uuid::new_v4().to_string(),
                kind: ActionType::AppendFatDat,
                source: Some("ceviri.bin".into()),
                destination: "patch.dat".into(),
                backup: true,
                expected_sha256: None,
                options: serde_json::from_value(serde_json::json!({
                    "fat_path":"patch.fat", "fat_entry_hash":"000000000000007B", "base_dat_sha256":digest(&dat),
                    "base_fat_sha256":digest(&fat), "payload_sha256":digest(&payload), "alignment":8, "compression":"none"
                }))
                .unwrap(),
            };
            let installation = Installation {
                schema_version: 1,
                game_id: 1,
                game_name: "fixture".into(),
                patch_id: 1,
                patch_version: "1.1.0".into(),
                game_root: game.display().to_string(),
                backup_id: "backup".into(),
                created_at: "fixture".into(),
                active: true,
                changes: vec![],
            };
            let registration = temp.path().join("registration.json");
            Self {
                _temp: temp,
                game,
                archive,
                backup,
                registration,
                action,
                installation,
                dat,
                fat,
                payload,
            }
        }
        fn apply(&mut self) -> Result<()> {
            install(
                &self.action,
                &self.archive,
                &self.game,
                &self.backup,
                &mut self.installation,
                &self.registration,
                None,
            )
        }
        fn undo(&self) -> Result<()> {
            restore(
                &self.game,
                &self.backup,
                &self.installation.changes[0],
            )
        }
        fn unchanged(&self) {
            assert_eq!(fs::read(self.game.join("patch.dat")).unwrap(), self.dat);
            assert_eq!(fs::read(self.game.join("patch.fat")).unwrap(), self.fat);
        }
    }

    #[test]
    fn install_extract_verify_and_restore() {
        let mut f = Fixture::new();
        f.apply().unwrap();
        let dat = fs::read(f.game.join("patch.dat")).unwrap();
        let fat = fs::read(f.game.join("patch.fat")).unwrap();
        assert_eq!(&dat[..73], &f.dat);
        assert_eq!(&dat[73..80], &[0; 7]);
        let offset = u32_at(&fat, 36) as usize * 8 + (u32_at(&fat, 40) >> 29) as usize;
        let size = (u32_at(&fat, 32) >> 2) as usize;
        assert_eq!(&dat[offset..offset + size], &f.payload);
        assert_eq!(&fat[..32], &f.fat[..32]);
        assert_eq!(&fat[44..], &f.fat[44..]);
        assert!(verify(&f.game, &f.backup, &f.installation.changes[0]).unwrap());
        let registered: Installation = read_json(&f.registration).unwrap();
        assert_eq!(registered.changes.len(), 1);
        assert!(f.game.join(RECOVERY_DIR).is_dir());
        f.undo().unwrap();
        f.unchanged();
        f.undo().unwrap();
    }

    #[test]
    fn deleted_local_backup_self_heals_from_game_recovery() {
        let mut f = Fixture::new();
        f.apply().unwrap();
        fs::remove_dir_all(&f.backup).unwrap();
        assert!(verify(&f.game, &f.backup, &f.installation.changes[0]).unwrap());
        assert!(f.backup.join("metadata.json").is_file());
        assert!(f.backup.join("fat-dat-undo.json").is_file());
        f.undo().unwrap();
        f.unchanged();
    }

    #[test]
    fn deleted_registration_and_backup_are_repaired_by_install() {
        let mut f = Fixture::new();
        f.apply().unwrap();
        let before_dat = fs::read(f.game.join("patch.dat")).unwrap();
        let before_fat = fs::read(f.game.join("patch.fat")).unwrap();
        fs::remove_file(&f.registration).unwrap();
        fs::remove_dir_all(&f.backup).unwrap();

        let repaired_backup = f._temp.path().join("repaired");
        fs::create_dir_all(&repaired_backup).unwrap();
        let mut repaired = Installation {
            schema_version: 1,
            game_id: 1,
            game_name: "fixture".into(),
            patch_id: 1,
            patch_version: "1.1.0".into(),
            game_root: f.game.display().to_string(),
            backup_id: "repaired".into(),
            created_at: "repaired".into(),
            active: true,
            changes: vec![],
        };
        install(
            &f.action,
            &f.archive,
            &f.game,
            &repaired_backup,
            &mut repaired,
            &f.registration,
            None,
        )
        .unwrap();
        assert_eq!(fs::read(f.game.join("patch.dat")).unwrap(), before_dat);
        assert_eq!(fs::read(f.game.join("patch.fat")).unwrap(), before_fat);
        assert_eq!(repaired.changes.len(), 1);
        assert!(repaired_backup.join("metadata.json").is_file());
        assert!(f.registration.is_file());
        restore(&f.game, &repaired_backup, &repaired.changes[0]).unwrap();
        f.unchanged();
    }

    #[test]
    fn wrong_base_or_payload_never_changes_game() {
        for key in ["base_dat_sha256", "base_fat_sha256", "payload_sha256"] {
            let mut f = Fixture::new();
            f.action
                .options
                .insert(key.into(), serde_json::json!("0".repeat(64)));
            assert!(f.apply().is_err());
            f.unchanged();
            assert!(!f.registration.exists());
        }
    }

    #[test]
    fn malformed_fat_rejected() {
        for mode in 0..4 {
            let mut f = Fixture::new();
            match mode {
                0 => put(&mut f.fat, 20, 1),
                1 => put(&mut f.fat, 36, 100),
                2 => f.fat[44..52].copy_from_slice(&123u64.to_le_bytes()),
                _ => put(&mut f.fat, 4, 11),
            }
            fs::write(f.game.join("patch.fat"), &f.fat).unwrap();
            f.action.options.insert(
                "base_fat_sha256".into(),
                serde_json::json!(digest(&f.fat)),
            );
            assert!(f.apply().is_err());
            f.unchanged();
        }
    }

    #[test]
    fn paths_and_options_fail_closed() {
        for path in [
            "../patch.fat",
            "C:/patch.fat",
            "/patch.fat",
            "x/../patch.fat",
            "x\\patch.fat",
            "x./patch.fat",
        ] {
            let mut f = Fixture::new();
            f.action
                .options
                .insert("fat_path".into(), serde_json::json!(path));
            assert!(f.apply().is_err());
            f.unchanged();
        }
        let mut f = Fixture::new();
        f.action
            .options
            .insert("execute".into(), serde_json::json!("evil.exe"));
        assert!(f.apply().is_err());
        f.unchanged();
    }

    #[test]
    fn interrupted_appends_and_fat_swap_are_recoverable() {
        for length in [73, 76, 80, 85] {
            let mut f = Fixture::new();
            f.apply().unwrap();
            fs::write(f.game.join("patch.fat"), &f.fat).unwrap();
            open_dat(&f.game.join("patch.dat"))
                .unwrap()
                .set_len(length)
                .unwrap();
            f.undo().unwrap();
            f.unchanged();
        }
        let mut f = Fixture::new();
        f.apply().unwrap();
        fs::write(f.game.join("patch.fat"), &f.fat).unwrap();
        f.undo().unwrap();
        f.unchanged();
    }

    #[test]
    fn modified_game_or_all_recovery_copies_corrupt_is_preserved() {
        for mode in 0..4 {
            let mut f = Fixture::new();
            f.apply().unwrap();
            match mode {
                0 => {
                    let mut d = open_dat(&f.game.join("patch.dat")).unwrap();
                    d.write_all(b"X").unwrap();
                }
                1 => {
                    let mut d = OpenOptions::new()
                        .append(true)
                        .open(f.game.join("patch.dat"))
                        .unwrap();
                    d.write_all(b"USER").unwrap();
                }
                2 => fs::write(f.game.join("patch.fat"), b"USER FAT").unwrap(),
                _ => {
                    fs::write(
                        f.backup.join("files/fat-original.bin"),
                        b"BAD BACKUP",
                    )
                    .unwrap();
                    let r = load_recovery_for_change(&f.game, &f.installation.changes[0])
                        .unwrap()
                        .unwrap();
                    fs::write(r.paths.original_fat, b"BAD RECOVERY").unwrap();
                }
            }
            let before_dat = fs::read(f.game.join("patch.dat")).unwrap();
            let before_fat = fs::read(f.game.join("patch.fat")).unwrap();
            assert!(f.undo().is_err());
            assert_eq!(fs::read(f.game.join("patch.dat")).unwrap(), before_dat);
            assert_eq!(fs::read(f.game.join("patch.fat")).unwrap(), before_fat);
        }
    }

    #[test]
    fn fat_only_tampering_fails_verification() {
        let mut f = Fixture::new();
        f.apply().unwrap();
        fs::write(f.game.join("patch.fat"), &f.fat).unwrap();
        assert!(!verify(&f.game, &f.backup, &f.installation.changes[0]).unwrap());
    }

    #[test]
    fn registration_error_happens_before_game_write() {
        let mut f = Fixture::new();
        fs::create_dir(&f.registration).unwrap();
        assert!(f.apply().is_err());
        f.unchanged();
    }
}
