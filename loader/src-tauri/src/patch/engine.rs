use crate::{
    backup::{backup_root, installation_path},
    error::{LoaderError, Result},
    game_detection::validate_root,
    models::{
        Action, ActionType, ChangeKind, ChangeRecord, DryRun, Installation, Manifest, Progress,
        UninstallReport, Verification,
    },
    patch::actions::PatchActionHandler,
    security::{
        archive::extract_safe,
        path::{resolve_inside, validate_relative},
    },
    storage::{copy_file, hash_file, read_json, write_json_atomic},
};
use chrono::Utc;
use std::{
    fs,
    path::{Path, PathBuf},
};
use sysinfo::{Disks, System};
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;


// ============================================================
// ANIMUS MANAGED PLAYSTATION ROOT
// ============================================================

/// Bu iki kayit normal PC oyunu degil; MediaFire paketindeki oyun imaji
/// Animus'un kendi yonettigi klasore kurulur.
fn is_managed_ps_game(manifest: &Manifest) -> bool {
    matches!(
        manifest.game.slug.as_str(),
        "silent-hill-1" | "resident-evil-code-veronica-2000"
    )
}

/// PS oyunlari icin kullanicidan oyun klasoru istemeyiz.
/// Yalnizca Animus'un bu oyun icin ayirdigi LocalAppData klasorune izin verilir.
/// Diger tum oyunlarda mevcut klasik oyun-koku dogrulamasi aynen devam eder.
fn validate_install_root(manifest: &Manifest, game_root: &Path) -> Result<()> {
    if !is_managed_ps_game(manifest) {
        return validate_root(game_root, &manifest.detection.required_files);
    }

    let local = dirs::data_local_dir().ok_or_else(|| {
        LoaderError::Other("Windows LocalAppData klasoru bulunamadi.".into())
    })?;

    let expected = local
        .join("AnimusPatchLoader")
        .join("emulated-games")
        .join(format!("game-{}", manifest.game.id));

    // dry-run ve kurulumdan once klasorun gercekten var olmasini garanti eder.
    fs::create_dir_all(&expected)?;

    let actual = game_root.canonicalize().map_err(|error| {
        LoaderError::Other(format!(
            "Animus PlayStation oyun klasoru okunamadi: {error}"
        ))
    })?;

    let expected = expected.canonicalize().map_err(|error| {
        LoaderError::Other(format!(
            "Animus PlayStation hedef klasoru okunamadi: {error}"
        ))
    })?;

    if actual != expected {
        return Err(LoaderError::Other(
            "PlayStation oyunu yalnizca Animus'un yonettigi oyun klasorune kurulabilir.".into(),
        ));
    }

    Ok(())
}

pub fn validate_manifest(manifest: &Manifest) -> Result<()> {
    if manifest.schema_version != 1 {
        return Err(LoaderError::Manifest(format!(
            "Desteklenmeyen schema version: {}",
            manifest.schema_version
        )));
    }
    if manifest.game.id == 0 || manifest.patch.id == 0 || manifest.patch.version.trim().is_empty() {
        return Err(LoaderError::Manifest("Game/patch kimliği eksik".into()));
    }
    if manifest.install_actions.is_empty() {
        return Err(LoaderError::Manifest("Install action listesi boş".into()));
    }
    if manifest.archive.size == 0
        || manifest.archive.sha256.len() != 64
        || !manifest
            .archive
            .sha256
            .chars()
            .all(|c| c.is_ascii_hexdigit())
    {
        return Err(LoaderError::Manifest(
            "Archive bütünlük bilgisi geçersiz".into(),
        ));
    }
    if !manifest.backup.automatic || manifest.integrity.conflict_policy != "abort" {
        return Err(LoaderError::Manifest(
            "Güvensiz backup/conflict politikası".into(),
        ));
    }
    if is_managed_ps_game(manifest) {
        // PS oyunlarinda Windows .exe / mevcut oyun klasoru zorunlu degildir.
        // Dolu bir detection yolu varsa yine guvenlik kontrolunden gecir.
        if !manifest.detection.executable.trim().is_empty() {
            validate_relative(&manifest.detection.executable)?;
        }
        for file in manifest
            .detection
            .required_files
            .iter()
            .chain(manifest.detection.optional_files.iter())
        {
            if !file.trim().is_empty() {
                validate_relative(file)?;
            }
        }
    } else {
        validate_relative(&manifest.detection.executable)?;
        for file in manifest
            .detection
            .required_files
            .iter()
            .chain(manifest.detection.optional_files.iter())
        {
            validate_relative(file)?;
        }
    }
    for action in &manifest.install_actions {
        if action.kind == ActionType::AppendFatDat {
            crate::patch::fat_dat::options(action)?;
            if !version_at_least(&manifest.patch.minimum_loader_version, "0.1.1") {
                return Err(LoaderError::Manifest("APPEND_FAT_DAT en az Loader 0.1.1 gerektirir".into()));
            }
            if manifest.detection.process_name.as_deref().unwrap_or("").trim().is_empty() {
                return Err(LoaderError::Manifest("FAT/DAT işlemi için oyun process_name gerekli".into()));
            }
            if manifest.install_actions.len() != 1 {
                return Err(LoaderError::Manifest("APPEND_FAT_DAT tek action olmalıdır".into()));
            }
        }
        uuid::Uuid::parse_str(&action.id)
            .map_err(|_| LoaderError::Manifest(format!("Action UUID geçersiz: {}", action.id)))?;
        validate_relative(&action.destination)?;
        if matches!(
            action.kind,
            ActionType::CopyFile
                | ActionType::CopyDirectory
                | ActionType::ReplaceFile
                | ActionType::MoveFile
                | ActionType::RenameFile
        ) {
            validate_relative(action.source.as_deref().ok_or_else(|| {
                LoaderError::Manifest(format!("Action source eksik: {}", action.id))
            })?)?;
        }
    }
    Ok(())
}

/// "1.2.0" >= "1.1.3" karsilastirmasi. Pre-release/build eki yok sayilir.
pub fn version_at_least(current: &str, minimum: &str) -> bool {
    fn parts(value: &str) -> Vec<u64> {
        value
            .trim()
            .split(['-', '+'])
            .next()
            .unwrap_or("")
            .split('.')
            .map(|piece| piece.trim().parse::<u64>().unwrap_or(0))
            .collect()
    }
    let (current, minimum) = (parts(current), parts(minimum));
    for index in 0..current.len().max(minimum.len()) {
        let a = current.get(index).copied().unwrap_or(0);
        let b = minimum.get(index).copied().unwrap_or(0);
        if a != b {
            return a > b;
        }
    }
    true
}

/// Yama, kendisinden yeni bir loader istiyorsa kurulum baslamadan durdurulur.
/// Bu kontrol eskiden hicbir yerde yapilmiyordu; manifestteki alan olu veriydi.
pub fn ensure_loader_version(manifest: &Manifest, loader_version: &str) -> Result<()> {
    let minimum = manifest.patch.minimum_loader_version.trim();
    if minimum.is_empty() || version_at_least(loader_version, minimum) {
        return Ok(());
    }
    Err(LoaderError::Manifest(format!(
        "Bu yama en az {minimum} sürümünde loader gerektiriyor (kurulu: {loader_version}). Önce loader'ı güncelleyin."
    )))
}

pub fn dry_run(manifest: &Manifest, game_root: &Path) -> Result<DryRun> {
    validate_manifest(manifest)?;
    validate_install_root(manifest, game_root)?;
    let mut result = DryRun {
        created_files: 0,
        changed_files: 0,
        deleted_files: 0,
        backup_files: 0,
        download_bytes: manifest.archive.size,
        estimated_disk_bytes: manifest.archive.size.saturating_mul(2),
        warnings: vec![],
    };
    for action in &manifest.install_actions {
        let destination = resolve_inside(game_root, &action.destination)?;
        match action.kind {
            ActionType::AppendFatDat => {
                result.changed_files += 2;
                result.backup_files += 2;
                result.estimated_disk_bytes = result.estimated_disk_bytes.saturating_add(
                    crate::patch::fat_dat::disk_bytes(action, game_root)?);
            }
            ActionType::CopyFile | ActionType::ReplaceFile => {
                if destination.exists() {
                    result.changed_files += 1;
                    result.backup_files += 1
                } else {
                    result.created_files += 1
                }
            }
            ActionType::CopyDirectory => {
                result.created_files += 1;
                result.warnings.push(
                    "Directory içindeki kesin dosya sayısı ZIP açıldıktan sonra hesaplanır.".into(),
                )
            }
            ActionType::DeleteFile => {
                if destination.exists() {
                    result.deleted_files += 1;
                    result.backup_files += 1
                }
            }
            ActionType::DeleteDirectory => {
                if destination.exists() {
                    result.deleted_files += WalkDir::new(&destination)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter(|e| e.file_type().is_file())
                        .count() as u64;
                    result.backup_files = result.deleted_files
                }
            }
            ActionType::CreateDirectory => {
                if !destination.exists() {
                    result.created_files += 1
                }
            }
            ActionType::MoveFile | ActionType::RenameFile => {
                result.changed_files += 1;
                if destination.exists() {
                    result.backup_files += 1
                }
            }
        }
    }
    result.estimated_disk_bytes = result
        .estimated_disk_bytes
        .saturating_add(result.backup_files.saturating_mul(4 * 1024 * 1024));
    Ok(result)
}

pub fn install(
    app: &AppHandle,
    manifest: &Manifest,
    game_root: &Path,
    archive_url: &str,
    loader_version: &str,
    force: bool,
) -> Result<Installation> {
    validate_manifest(manifest)?;
    ensure_loader_version(manifest, loader_version)?;
    validate_install_root(manifest, game_root)?;
    assert_not_running(manifest.detection.process_name.as_deref())?;
    let plan = dry_run(manifest, game_root)?;
    ensure_disk_space(game_root, plan.estimated_disk_bytes)?;
    let _ = crate::logging::event(
        "info",
        "install",
        &format!(
            "Kurulum başladı: game_id={}, patch={}",
            manifest.game.id, manifest.patch.version
        ),
    );
    let _ = app.emit(
        "patch-progress",
        Progress {
            stage: "prepare".into(),
            percent: 2,
            message: "Kurulum hazırlanıyor".into(),
            downloaded_bytes: None,
            total_bytes: None,
            bytes_per_second: None,
        },
    );
    let temp = tempfile::Builder::new().prefix("animus-patch-").tempdir()?;
    // Arsiv kalici onbellege iner: baglanti koparsa ayni dosyadan devam edilir.
    let archive = crate::download::cache_path(&manifest.archive.sha256)?;
    crate::download::download(
        app,
        archive_url,
        &archive,
        manifest.archive.size,
        &manifest.archive.sha256,
    )?;
    let extracted = temp.path().join("extracted");
    extract_safe(&archive, &extracted)?;
    let _ = app.emit(
        "patch-progress",
        Progress {
            stage: "extract".into(),
            percent: 58,
            message: "Arşiv güvenli dizine çıkarıldı".into(),
            downloaded_bytes: None,
            total_bytes: None,
            bytes_per_second: None,
        },
    );
    // Guncelleme akisi: yeni surum uygulanmadan once onceki yama geri alinir.
    // Aksi halde yeni backup "orijinal" diye zaten yamali dosyalari kaydeder ve
    // "Yamayi Kaldir" oyunu hicbir zaman gercek vanilla haline donduremez.
    if let Some(previous) = crate::backup::find_installation(manifest.game.id)? {
        restore_previous(game_root, &previous, force)?;
        let _ = app.emit(
            "patch-progress",
            Progress {
                stage: "restore".into(),
                percent: 59,
                message: "Önceki yama kaldırıldı, oyun orijinal haline döndürüldü".into(),
                downloaded_bytes: None,
                total_bytes: None,
                bytes_per_second: None,
            },
        );
    }

    let backup_id = uuid::Uuid::new_v4().to_string();
    let backup = backup_root(&backup_id)?;
    fs::create_dir_all(backup.join("files"))?;
    let mut installation = Installation {
        schema_version: 1,
        game_id: manifest.game.id,
        game_name: manifest.game.name.clone(),
        patch_id: manifest.patch.id,
        patch_version: manifest.patch.version.clone(),
        game_root: game_root.display().to_string(),
        backup_id: backup_id.clone(),
        created_at: Utc::now().to_rfc3339(),
        active: true,
        changes: vec![],
    };
    let journal = backup.join("journal.json");
    let handlers: Vec<Box<dyn PatchActionHandler>> = vec![
        Box::new(CopyHandler),
        Box::new(DeleteHandler),
        Box::new(DirectoryHandler),
        Box::new(MoveHandler),
    ];
    for (index, action) in manifest.install_actions.iter().enumerate() {
        if action.kind == ActionType::AppendFatDat {
            let registration = installation_path(manifest.game.id)?;
            if let Err(error) = crate::patch::fat_dat::install(
                action, &extracted, game_root, &backup, &mut installation, &registration,
                manifest.detection.process_name.as_deref(),
            ) {
                match rollback_changes(game_root, &backup, &installation.changes) {
                    Ok(()) => crate::backup::close_installation(&installation)?,
                    Err(rollback) => return Err(LoaderError::Other(format!(
                        "{error}. Geri alma da durdu: {rollback}. Yedek korundu: {}", backup.display()))),
                }
                return Err(error);
            }
            continue;
        }
        let handler = handlers
            .iter()
            .find(|h| h.supports(action))
            .ok_or_else(|| {
                LoaderError::Manifest(format!("Handler bulunamadı: {:?}", action.kind))
            })?;
        if let Err(error) = handler.apply(
            action,
            &extracted,
            game_root,
            &backup,
            &mut installation.changes,
        ) {
            let _ = crate::logging::event(
                "error",
                "rollback",
                &format!("Action başarısız, rollback uygulanıyor: {}", error),
            );
            let _ = rollback_changes(game_root, &backup, &installation.changes);
            return Err(error);
        }
        if let Some(expected) = &action.expected_sha256 {
            if !matches!(
                action.kind,
                ActionType::DeleteFile | ActionType::DeleteDirectory | ActionType::CreateDirectory
            ) {
                let destination = resolve_inside(game_root, &action.destination)?;
                if !destination.is_file()
                    || !hash_file(&destination)?.eq_ignore_ascii_case(expected)
                {
                    let _ = rollback_changes(game_root, &backup, &installation.changes);
                    return Err(LoaderError::Integrity(format!(
                        "Action sonucu doğrulanamadı: {}",
                        action.destination
                    )));
                }
            }
        }
        write_json_atomic(&journal, &installation)?;
        let percent = 60 + (((index + 1) * 35 / manifest.install_actions.len()) as u8);
        let _ = app.emit(
            "patch-progress",
            Progress {
                stage: "install".into(),
                percent,
                message: format!(
                    "İşlem {}/{} uygulandı",
                    index + 1,
                    manifest.install_actions.len()
                ),
                downloaded_bytes: None,
                total_bytes: None,
                bytes_per_second: None,
            },
        );
    }
    if manifest.integrity.verify_after_install {
        let check = verify_records_with_backup(game_root, &backup, &installation.changes)?;
        if !check.valid {
            let _ = rollback_changes(game_root, &backup, &installation.changes);
            return Err(LoaderError::Integrity(check.conflicts.join(", ")));
        }
    }
    write_json_atomic(&backup.join("metadata.json"), &installation)?;
    write_json_atomic(&installation_path(manifest.game.id)?, &installation)?;
    let _ = fs::remove_file(&archive);
    let _ = crate::logging::event(
        "info",
        "install",
        &format!("Kurulum tamamlandı: game_id={}", manifest.game.id),
    );
    let _ = app.emit(
        "patch-progress",
        Progress {
            stage: "complete".into(),
            percent: 100,
            message: "Yama başarıyla kuruldu".into(),
            downloaded_bytes: None,
            total_bytes: None,
            bytes_per_second: None,
        },
    );
    Ok(installation)
}

fn is_playstation_disc_image(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str(),
        "iso" | "chd" | "cso" | "cue" | "bin" | "img" | "mdf" | "pbp"
    )
}

/// MediaFire/ZIP tarafında özellikle Türkçe karakterli büyük oyun imajlarının
/// dosya adı ZIP code-page farkı yüzünden manifestteki adla birebir
/// eşleşmeyebilir. Önce tam yolu kullanırız. Tam yol yoksa yalnızca
/// PlayStation disk-imajı uzantılarında kontrollü fallback uygularız.
///
/// Güvenlik:
/// - Arşiv dışına çıkılmaz; yalnızca extract_safe ile açılmış kök taranır.
/// - Aynı uzantıda birden fazla aday varsa, expected_sha256 varsa hash ile seçilir.
/// - Hash yoksa yalnızca TEK aday olduğunda otomatik seçilir.
/// - Belirsiz durumda tahmin yapılmaz ve kurulum durdurulur.
fn resolve_archive_copy_source(
    archive: &Path,
    action: &Action,
) -> Result<PathBuf> {
    let source_rel = action.source.as_deref().ok_or_else(|| {
        LoaderError::Manifest(format!(
            "Action source eksik: {}",
            action.id
        ))
    })?;

    let exact = resolve_inside(archive, source_rel)?;

    if exact.is_file() {
        return Ok(exact);
    }

    let expected_path = Path::new(source_rel);

    if !is_playstation_disc_image(expected_path) {
        return Err(LoaderError::Manifest(format!(
            "Archive source file yok: {source_rel}"
        )));
    }

    let expected_extension = expected_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let mut candidates = WalkDir::new(archive)
        .follow_links(false)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| {
            path.extension()
                .and_then(|value| value.to_str())
                .map(|value| value.eq_ignore_ascii_case(&expected_extension))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    if candidates.is_empty() {
        return Err(LoaderError::Manifest(format!(
            "Archive source file yok: {source_rel}. Arşiv içinde .{expected_extension} oyun imajı da bulunamadı."
        )));
    }

    // Tek aday varsa büyük ISO'yu gereksiz yere ikinci kez hash'leme.
    // Action sonucu zaten aşağıdaki normal bütünlük kontrolünde doğrulanacak.
    if candidates.len() == 1 {
        let selected = candidates.remove(0);

        let _ = crate::logging::event(
            "warning",
            "install",
            &format!(
                "Manifest archive source adı eşleşmedi; tek .{expected_extension} PlayStation oyun imajı otomatik kullanılıyor. Beklenen: {source_rel}, bulunan: {}",
                selected.display()
            ),
        );

        return Ok(selected);
    }

    // Birden fazla aday varsa manifest hedef hash'ini kullanarak içerikle eşleştir.
    // Bu, çok parçalı BIN setlerinde veya birden fazla disk imajında yanlış
    // dosya seçilmesini önler.
    if let Some(expected_hash) = action.expected_sha256.as_deref() {
        let mut hash_matches = Vec::new();

        for candidate in &candidates {
            if hash_file(candidate)?
                .eq_ignore_ascii_case(expected_hash)
            {
                hash_matches.push(candidate.clone());
            }
        }

        if hash_matches.len() == 1 {
            let selected = hash_matches.remove(0);

            let _ = crate::logging::event(
                "warning",
                "install",
                &format!(
                    "Manifest archive source adı eşleşmedi; SHA-256 ile PlayStation oyun imajı bulundu. Beklenen: {source_rel}, bulunan: {}",
                    selected.display()
                ),
            );

            return Ok(selected);
        }

        if hash_matches.len() > 1 {
            return Err(LoaderError::Manifest(format!(
                "Archive source eşleşmesi belirsiz: {source_rel}. Aynı SHA-256 değerine sahip birden fazla .{expected_extension} dosyası bulundu."
            )));
        }
    }

    let names = candidates
        .iter()
        .filter_map(|path| {
            path.strip_prefix(archive)
                .ok()
                .map(|relative| relative.display().to_string())
        })
        .take(8)
        .collect::<Vec<_>>()
        .join(", ");

    Err(LoaderError::Manifest(format!(
        "Archive source eşleşmesi belirsiz: {source_rel}. Birden fazla .{expected_extension} dosyası bulundu: {names}"
    )))
}

struct CopyHandler;
impl PatchActionHandler for CopyHandler {
    fn supports(&self, a: &Action) -> bool {
        matches!(
            a.kind,
            ActionType::CopyFile | ActionType::ReplaceFile | ActionType::CopyDirectory
        )
    }
    fn apply(
        &self,
        a: &Action,
        archive: &Path,
        game: &Path,
        backup: &Path,
        changes: &mut Vec<ChangeRecord>,
    ) -> Result<()> {
        if a.kind == ActionType::CopyDirectory {
            let source =
                resolve_inside(archive, a.source.as_deref().unwrap())?;

            if !source.is_dir() {
                return Err(LoaderError::Manifest(format!(
                    "Archive source directory yok: {}",
                    a.source.as_deref().unwrap()
                )));
            }

            for entry in WalkDir::new(&source)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let relative = entry
                    .path()
                    .strip_prefix(&source)
                    .map_err(|_| LoaderError::UnsafePath(entry.path().display().to_string()))?;
                let destination_relative = Path::new(&a.destination).join(relative);
                let destination_string = destination_relative.to_string_lossy().replace('\\', "/");
                let destination = resolve_inside(game, &destination_string)?;
                copy_one(
                    entry.path(),
                    &destination,
                    &destination_string,
                    backup,
                    changes,
                )?;
            }
        } else {
            let source =
                resolve_archive_copy_source(archive, a)?;

            let destination =
                resolve_inside(game, &a.destination)?;

            copy_one(
                &source,
                &destination,
                &a.destination,
                backup,
                changes,
            )?;
        }

        Ok(())
    }
}
struct DeleteHandler;
impl PatchActionHandler for DeleteHandler {
    fn supports(&self, a: &Action) -> bool {
        matches!(a.kind, ActionType::DeleteFile | ActionType::DeleteDirectory)
    }
    fn apply(
        &self,
        a: &Action,
        _: &Path,
        game: &Path,
        backup: &Path,
        changes: &mut Vec<ChangeRecord>,
    ) -> Result<()> {
        let destination = resolve_inside(game, &a.destination)?;
        if !destination.exists() {
            return Ok(());
        }
        if a.kind == ActionType::DeleteDirectory {
            for entry in WalkDir::new(&destination)
                .contents_first(true)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let rel = entry
                    .path()
                    .strip_prefix(game)
                    .map_err(|_| LoaderError::UnsafePath(entry.path().display().to_string()))?
                    .to_string_lossy()
                    .replace('\\', "/");
                backup_deleted(entry.path(), &rel, backup, changes)?;
            }
            fs::remove_dir_all(destination)?;
        } else {
            backup_deleted(&destination, &a.destination, backup, changes)?;
            fs::remove_file(destination)?;
        }
        Ok(())
    }
}
struct DirectoryHandler;
impl PatchActionHandler for DirectoryHandler {
    fn supports(&self, a: &Action) -> bool {
        a.kind == ActionType::CreateDirectory
    }
    fn apply(
        &self,
        a: &Action,
        _: &Path,
        game: &Path,
        _: &Path,
        changes: &mut Vec<ChangeRecord>,
    ) -> Result<()> {
        let destination = resolve_inside(game, &a.destination)?;
        if !destination.exists() {
            fs::create_dir_all(&destination)?;
            changes.push(ChangeRecord {
                kind: ChangeKind::CreatedDirectory,
                path: a.destination.clone(),
                secondary_path: None,
                backup_path: None,
                original_sha256: None,
                installed_sha256: None,
            });
        }
        Ok(())
    }
}
struct MoveHandler;
impl PatchActionHandler for MoveHandler {
    fn supports(&self, a: &Action) -> bool {
        matches!(a.kind, ActionType::MoveFile | ActionType::RenameFile)
    }
    fn apply(
        &self,
        a: &Action,
        _: &Path,
        game: &Path,
        backup: &Path,
        changes: &mut Vec<ChangeRecord>,
    ) -> Result<()> {
        let source_rel = a.source.as_deref().unwrap();
        let source = resolve_inside(game, source_rel)?;
        if !source.is_file() {
            return Err(LoaderError::Manifest(format!(
                "Taşınacak dosya yok: {source_rel}"
            )));
        }
        let destination = resolve_inside(game, &a.destination)?;
        let (backup_path, original_sha256) = if destination.exists() {
            let (p, h) = make_backup(&destination, backup)?;
            (Some(p), Some(h))
        } else {
            (None, None)
        };
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(&source, &destination)?;
        let installed_sha256 = Some(hash_file(&destination)?);
        changes.push(ChangeRecord {
            kind: ChangeKind::MovedFile,
            path: a.destination.clone(),
            secondary_path: Some(source_rel.into()),
            backup_path,
            original_sha256,
            installed_sha256,
        });
        Ok(())
    }
}

fn copy_one(
    source: &Path,
    destination: &Path,
    destination_rel: &str,
    backup: &Path,
    changes: &mut Vec<ChangeRecord>,
) -> Result<()> {
    let (kind, backup_path, original_sha256) = if destination.exists() {
        let (p, h) = make_backup(destination, backup)?;
        (ChangeKind::ReplacedFile, Some(p), Some(h))
    } else {
        (ChangeKind::CreatedFile, None, None)
    };
    copy_file(source, destination)?;
    let installed_sha256 = Some(hash_file(destination)?);
    changes.push(ChangeRecord {
        kind,
        path: destination_rel.into(),
        secondary_path: None,
        backup_path,
        original_sha256,
        installed_sha256,
    });
    Ok(())
}
fn backup_deleted(
    path: &Path,
    relative: &str,
    backup: &Path,
    changes: &mut Vec<ChangeRecord>,
) -> Result<()> {
    let (p, h) = make_backup(path, backup)?;
    changes.push(ChangeRecord {
        kind: ChangeKind::DeletedFile,
        path: relative.into(),
        secondary_path: None,
        backup_path: Some(p),
        original_sha256: Some(h),
        installed_sha256: None,
    });
    Ok(())
}
fn make_backup(path: &Path, backup: &Path) -> Result<(String, String)> {
    let relative = format!("files/{}", uuid::Uuid::new_v4());
    let target = backup.join(&relative);
    copy_file(path, &target)?;
    Ok((relative, hash_file(path)?))
}
fn rollback_changes(game: &Path, backup: &Path, changes: &[ChangeRecord]) -> Result<()> {
    for change in changes.iter().rev() {
        let path = resolve_inside(game, &change.path)?;
        match change.kind {
            ChangeKind::PatchedArchive => crate::patch::fat_dat::restore(game, backup, change)?,
            ChangeKind::CreatedFile => {
                if path.is_file() {
                    fs::remove_file(path)?;
                }
            }
            ChangeKind::ReplacedFile | ChangeKind::DeletedFile => {
                if path.exists() && path.is_file() {
                    fs::remove_file(&path)?;
                }
                if let Some(saved) = &change.backup_path {
                    copy_file(&backup.join(saved), &path)?;
                }
            }
            ChangeKind::CreatedDirectory => {
                if path.is_dir() && fs::read_dir(&path)?.next().is_none() {
                    fs::remove_dir(path)?;
                }
            }
            ChangeKind::MovedFile => {
                let source = resolve_inside(
                    game,
                    change
                        .secondary_path
                        .as_deref()
                        .ok_or_else(|| LoaderError::Manifest("Move journal eksik".into()))?,
                )?;
                if path.exists() {
                    if let Some(parent) = source.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::rename(&path, &source)?;
                }
                if let Some(saved) = &change.backup_path {
                    copy_file(&backup.join(saved), &path)?;
                }
            }
        }
    }
    Ok(())
}
fn verify_records(game: &Path, changes: &[ChangeRecord]) -> Result<Verification> {
    let mut conflicts = vec![];
    let mut checked = 0;
    for change in changes {
        if let Some(expected) = &change.installed_sha256 {
            checked += 1;
            let path = resolve_inside(game, &change.path)?;
            if !path.is_file() || !hash_file(&path)?.eq_ignore_ascii_case(expected) {
                conflicts.push(change.path.clone());
            }
        }
    }
    Ok(Verification {
        valid: conflicts.is_empty(),
        checked,
        conflicts,
    })
}
pub fn verify_installation(game_id: u64, game_root: &Path) -> Result<Verification> {
    let install: Installation = read_json(&installation_path(game_id)?)?;
    verify_records_with_backup(game_root, &backup_root(&install.backup_id)?, &install.changes)
}
fn verify_records_with_backup(game: &Path, backup: &Path, changes: &[ChangeRecord]) -> Result<Verification> {
    let mut result = Verification { valid: true, checked: 0, conflicts: vec![] };
    for change in changes {
        if change.kind == ChangeKind::PatchedArchive {
            result.checked += 2;
            if !crate::patch::fat_dat::verify(game, backup, change)? {
                result.conflicts.push(change.path.clone());
                if let Some(fat) = &change.secondary_path { result.conflicts.push(fat.clone()); }
            }
        } else {
            let r = verify_records(game, std::slice::from_ref(change))?;
            result.checked += r.checked;
            result.conflicts.extend(r.conflicts);
        }
    }
    result.valid = result.conflicts.is_empty();
    Ok(result)
}
fn is_archive_install(changes: &[ChangeRecord]) -> bool {
    changes.len() == 1 && changes[0].kind == ChangeKind::PatchedArchive
}

fn backup_material_exists(id: &str) -> Result<bool> {
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(LoaderError::UnsafePath(id.into()));
    }
    let primary = crate::storage::data_root()?.join("backups").join(id);
    let recovery = crate::storage::recovery_backups_root()?.join(id);
    Ok(primary.join("metadata.json").is_file() || recovery.join("metadata.json").is_file())
}

fn forget_installation_record(install: &Installation) -> Result<()> {
    let path = installation_path(install.game_id)?;
    if path.is_file() {
        fs::remove_file(path)?;
    }
    Ok(())
}

/// Onceki kurulumu vanilla haline dondurur. `force`, kullanici dosyalari elle
/// degistirmis olsa bile yedekten geri yazmaya izin verir. Backup/recovery
/// tamamen kayipsa force yalnızca kullanıcının oyun dosyalarını mağazadan
/// doğruladığını kabul ederek eski Loader kaydını temizler; oyun dosyalarına
/// dokunmaz.
fn restore_previous(game_root: &Path, previous: &Installation, force: bool) -> Result<()> {
    let same_root = PathBuf::from(&previous.game_root)
        .canonicalize()
        .ok()
        .zip(game_root.canonicalize().ok())
        .map(|(a, b)| a == b)
        .unwrap_or(false);
    if !same_root {
        return Err(LoaderError::Conflict(format!(
            "Bu oyunun yaması başka bir klasörde kurulu: {}. Önce oradan kaldırın.",
            previous.game_root
        )));
    }

    if !backup_material_exists(&previous.backup_id)? {
        if !force {
            return Err(LoaderError::Conflict(
                "Önceki kurulumun yedeği bulunamadı ve kurtarma kopyası da yok. Oyun dosyalarını Steam/Ubisoft üzerinden doğruladıysanız zorla devam ederek yalnızca eski Loader kaydını temizleyebilirsiniz; oyun dosyaları Loader tarafından değiştirilmeyecek.".into(),
            ));
        }
        forget_installation_record(previous)?;
        let _ = crate::logging::event(
            "warning",
            "install",
            &format!(
                "Backup/recovery yok; doğrulama sonrası yalnızca eski kurulum kaydı temizlendi: game_id={}",
                previous.game_id
            ),
        );
        return Ok(());
    }

    let backup = backup_root(&previous.backup_id)?;
    if !backup.join("metadata.json").is_file() {
        if !force {
            return Err(LoaderError::Conflict(
                "Önceki kurulumun yedeği bulunamadı. Zorla devam etmek dosyaları kalıcı değiştirebilir.".into(),
            ));
        }
        crate::backup::close_installation(previous)?;
        return Ok(());
    }
    let check = verify_records_with_backup(game_root, &backup, &previous.changes)?;
    if !check.valid && !force && !is_archive_install(&previous.changes) {
        return Err(LoaderError::Conflict(format!(
            "Önceki yamanın dosyaları değişmiş: {}. Önce 'Yamayı Kaldır' çalıştırın veya güncellemeyi zorlayın.",
            check.conflicts.join(", ")
        )));
    }
    rollback_changes(game_root, &backup, &previous.changes)?;
    crate::backup::close_installation(previous)?;
    Ok(())
}

pub fn uninstall(game_id: u64, game_root: &Path, force: bool) -> Result<UninstallReport> {
    let path = installation_path(game_id)?;
    let install: Installation = read_json(&path)?;
    if PathBuf::from(&install.game_root).canonicalize()? != game_root.canonicalize()? {
        return Err(LoaderError::Conflict(
            "Kurulum manifestindeki oyun kökü farklı".into(),
        ));
    }

    // Eski kurulumlarda kullanıcı LocalAppData backup'ını recovery sistemi
    // eklenmeden önce silmiş olabilir. Normal kaldırma bu durumda kesinlikle
    // dosyalara dokunmaz. Force ise yalnızca kullanıcı mağaza doğrulamasını
    // yaptığını onayladıktan sonra stale installation kaydını temizler.
    if !backup_material_exists(&install.backup_id)? {
        if !force {
            return Err(LoaderError::Conflict(
                "Kurulumun yedeği bulunamadı ve kurtarma kopyası da yok. Güvenlik için oyun dosyalarına dokunulmadı. Oyun dosyalarını Steam/Ubisoft üzerinden doğruladıysanız tekrar deneyip zorla devam ederek yalnızca eski Loader kaydını temizleyebilirsiniz.".into(),
            ));
        }
        forget_installation_record(&install)?;
        let note = "Yedek/kurtarma olmadığı için oyun dosyaları geri yüklenmedi; yalnızca eski Loader kurulum kaydı temizlendi.".to_string();
        let _ = crate::logging::event(
            "warning",
            "uninstall",
            &format!("{note} game_id={game_id}"),
        );
        return Ok(UninstallReport {
            restored: 0,
            forced: true,
            conflicts: vec![note],
        });
    }

    let backup = backup_root(&install.backup_id)?;
    let check = verify_records_with_backup(game_root, &backup, &install.changes)?;
    // FAT/DAT restore itself validates the base prefix and known partial suffix;
    // this also permits recovery after a terminated append without forcing.
    if !check.valid && !force && !is_archive_install(&install.changes) {
        return Err(LoaderError::Conflict(format!(
            "Dosyalar kurulumdan sonra değişmiş: {}",
            check.conflicts.join(", ")
        )));
    }
    if !backup.join("metadata.json").is_file() && !force {
        return Err(LoaderError::Conflict(
            "Kurulumun yedeği bulunamadı; orijinal dosyalar geri yüklenemez.".into(),
        ));
    }
    rollback_changes(game_root, &backup, &install.changes)?;
    crate::backup::close_installation(&install)?;
    let _ = crate::logging::event(
        "info",
        "uninstall",
        &format!("Yama kaldırıldı ve orijinal dosyalar geri yüklendi: game_id={game_id}"),
    );
    Ok(UninstallReport {
        restored: install.changes.len() as u64,
        forced: force && !check.valid,
        conflicts: check.conflicts,
    })
}
fn assert_not_running(process_name: Option<&str>) -> Result<()> {
    let Some(expected) = process_name else {
        return Ok(());
    };
    let system = System::new_all();
    if system
        .processes()
        .values()
        .any(|p| p.name().to_string_lossy().eq_ignore_ascii_case(expected))
    {
        return Err(LoaderError::GameRunning(expected.into()));
    }
    Ok(())
}
fn ensure_disk_space(root: &Path, needed: u64) -> Result<()> {
    let canonical = root.canonicalize()?;
    let disks = Disks::new_with_refreshed_list();
    let available = disks
        .iter()
        .filter(|d| canonical.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().components().count())
        .map(|d| d.available_space())
        .unwrap_or(u64::MAX);
    if available < needed {
        return Err(LoaderError::Other(format!(
            "Yetersiz disk alanı. Gerekli: {} MB",
            needed / 1048576
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::*;
    fn manifest() -> Manifest {
        Manifest {
            schema_version: 1,
            game: Game {
                id: 1,
                slug: "demo".into(),
                name: "Demo".into(),
            },
            detection: Detection {
                steam_app_id: None,
                epic_catalog_id: None,
                executable: "Demo.exe".into(),
                process_name: None,
                required_files: vec!["Demo.exe".into()],
                optional_files: vec![],
            },
            patch: Patch {
                id: 1,
                version: "1.0.0".into(),
                game_version: None,
                minimum_loader_version: "0.1.0".into(),
                mandatory: false,
                channel: "stable".into(),
            },
            archive: Archive {
                download_token_url: "https://example.com".into(),
                sha256: "0".repeat(64),
                size: 1,
            },
            install_actions: vec![Action {
                id: uuid::Uuid::new_v4().to_string(),
                kind: ActionType::CopyFile,
                source: Some("files/a".into()),
                destination: "Data/a".into(),
                backup: true,
                expected_sha256: None,
                options: Default::default(),
            }],
            integrity: Integrity {
                verify_after_install: true,
                conflict_policy: "abort".into(),
            },
            backup: BackupRule {
                automatic: true,
                retain_until_uninstall: true,
            },
        }
    }
    #[test]
    fn version_gate_blocks_old_loader() {
        let mut m = manifest();
        m.patch.minimum_loader_version = "1.2.0".into();
        assert!(ensure_loader_version(&m, "1.1.9").is_err());
        assert!(ensure_loader_version(&m, "1.2.0").is_ok());
        assert!(ensure_loader_version(&m, "1.2.1").is_ok());
        assert!(ensure_loader_version(&m, "2.0.0").is_ok());
    }

    #[test]
    fn version_compare_handles_shapes() {
        assert!(version_at_least("1.0.0", "1.0"));
        assert!(version_at_least("0.2.0-beta.1", "0.2.0"));
        assert!(!version_at_least("0.9.9", "1.0.0"));
        assert!(version_at_least("10.0.0", "9.9.9"));
    }

    #[test]
    fn update_over_existing_install_restores_vanilla_first() {
        // Regresyon testi: v1 kurulu iken v2 kurulunca yeni backup "orijinal"
        // olarak yamali dosyayi kaydediyordu ve kaldirma vanilla'ya donmuyordu.
        let temp = tempfile::tempdir().expect("temp");
        let game = temp.path().join("Game");
        let backup_v1 = temp.path().join("backup-v1");
        let extracted = temp.path().join("extracted");
        fs::create_dir_all(game.join("Data")).unwrap();
        fs::create_dir_all(backup_v1.join("files")).unwrap();
        fs::create_dir_all(extracted.join("files")).unwrap();
        fs::write(game.join("Data/text.dat"), b"VANILLA").unwrap();
        fs::write(extracted.join("files/text.dat"), b"PATCH V1").unwrap();

        let action = Action {
            id: uuid::Uuid::new_v4().to_string(),
            kind: ActionType::ReplaceFile,
            source: Some("files/text.dat".into()),
            destination: "Data/text.dat".into(),
            backup: true,
            expected_sha256: None,
            options: Default::default(),
        };
        let mut changes = Vec::new();
        CopyHandler
            .apply(&action, &extracted, &game, &backup_v1, &mut changes)
            .expect("v1 apply");
        assert_eq!(fs::read(game.join("Data/text.dat")).unwrap(), b"PATCH V1");

        let previous = Installation {
            schema_version: 1,
            game_id: 1,
            game_name: "Demo".into(),
            patch_id: 1,
            patch_version: "1.0.0".into(),
            game_root: game.display().to_string(),
            backup_id: "backup-v1".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            active: true,
            changes: changes.clone(),
        };

        // restore_previous mantigi: v2 uygulanmadan once vanilla'ya donulur.
        assert!(verify_records(&game, &previous.changes).unwrap().valid);
        rollback_changes(&game, &backup_v1, &previous.changes).expect("restore");
        assert_eq!(
            fs::read(game.join("Data/text.dat")).unwrap(),
            b"VANILLA",
            "guncelleme oncesi oyun vanilla haline donmeli"
        );

        // v2 artik vanilla uzerine kurulur, dolayisiyla yedegi de vanilla olur.
        fs::write(extracted.join("files/text.dat"), b"PATCH V2").unwrap();
        let backup_v2 = temp.path().join("backup-v2");
        fs::create_dir_all(backup_v2.join("files")).unwrap();
        let mut changes_v2 = Vec::new();
        CopyHandler
            .apply(&action, &extracted, &game, &backup_v2, &mut changes_v2)
            .expect("v2 apply");
        assert_eq!(fs::read(game.join("Data/text.dat")).unwrap(), b"PATCH V2");
        rollback_changes(&game, &backup_v2, &changes_v2).expect("v2 uninstall");
        assert_eq!(
            fs::read(game.join("Data/text.dat")).unwrap(),
            b"VANILLA",
            "v2 kaldirilinca vanilla'ya donmeli, v1'e degil"
        );
    }

    #[test]
    fn ps_disc_source_falls_back_when_zip_filename_differs() {
        let temp = tempfile::tempdir().expect("temp");
        let extracted = temp.path().join("extracted");
        let game = temp.path().join("game");
        let backup = temp.path().join("backup");

        fs::create_dir_all(&extracted).unwrap();
        fs::create_dir_all(&game).unwrap();
        fs::create_dir_all(backup.join("files")).unwrap();

        let actual = extracted.join("Resident Evil CVX-TR - ANIMUS CEViRi.iso");
        fs::write(&actual, b"PS2 ISO TEST").unwrap();

        let action = Action {
            id: uuid::Uuid::new_v4().to_string(),
            kind: ActionType::CopyFile,
            source: Some("Resident Evil CVX-TR - ANIMUS ÇEVİRİ.iso".into()),
            destination: "Resident Evil Code Veronica.iso".into(),
            backup: true,
            expected_sha256: Some(hash_file(&actual).unwrap()),
            options: Default::default(),
        };

        let mut changes = Vec::new();

        CopyHandler
            .apply(
                &action,
                &extracted,
                &game,
                &backup,
                &mut changes,
            )
            .expect("PS disc fallback");

        assert_eq!(
            fs::read(game.join("Resident Evil Code Veronica.iso")).unwrap(),
            b"PS2 ISO TEST"
        );
    }

    #[test]
    fn ps_disc_source_does_not_guess_between_multiple_images_without_hash() {
        let temp = tempfile::tempdir().expect("temp");
        let extracted = temp.path().join("extracted");
        fs::create_dir_all(&extracted).unwrap();
        fs::write(extracted.join("disc-a.iso"), b"A").unwrap();
        fs::write(extracted.join("disc-b.iso"), b"B").unwrap();

        let action = Action {
            id: uuid::Uuid::new_v4().to_string(),
            kind: ActionType::CopyFile,
            source: Some("missing-name.iso".into()),
            destination: "game.iso".into(),
            backup: true,
            expected_sha256: None,
            options: Default::default(),
        };

        let error =
            resolve_archive_copy_source(&extracted, &action)
                .expect_err("ambiguous images must fail");

        assert!(
            error.to_string().contains("belirsiz"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn valid_manifest() {
        assert!(validate_manifest(&manifest()).is_ok())
    }
    #[test]
    fn unknown_schema_fails() {
        let mut m = manifest();
        m.schema_version = 9;
        assert!(validate_manifest(&m).is_err())
    }

    #[test]
    fn fixture_install_backup_verify_and_restore() {
        let temp = tempfile::tempdir().expect("fixture temp");
        let game = temp.path().join("Patch Engine Test Game");
        let extracted = temp.path().join("extracted");
        let backup = temp.path().join("backup");
        let archive = temp.path().join("patch.zip");
        fs::create_dir_all(game.join("Data")).expect("game data");
        fs::create_dir_all(backup.join("files")).expect("backup files");
        fs::write(game.join("Game.exe"), b"fixture executable").expect("game exe");
        fs::write(game.join("Data/original.txt"), b"ORIGINAL").expect("original");
        {
            use std::io::Write;
            let mut zip = zip::ZipWriter::new(fs::File::create(&archive).expect("archive"));
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            zip.start_file("files/original.txt", options).unwrap();
            zip.write_all(b"TURKISH PATCH").unwrap();
            zip.start_file("files/turkish.txt", options).unwrap();
            zip.write_all(b"TURKISH FILE").unwrap();
            zip.finish().unwrap();
        }
        assert_eq!(hash_file(&archive).unwrap().len(), 64);
        extract_safe(&archive, &extracted).expect("safe extraction");

        let actions = [
            Action {
                id: uuid::Uuid::new_v4().to_string(),
                kind: ActionType::ReplaceFile,
                source: Some("files/original.txt".into()),
                destination: "Data/original.txt".into(),
                backup: true,
                expected_sha256: Some(hash_file(&extracted.join("files/original.txt")).unwrap()),
                options: Default::default(),
            },
            Action {
                id: uuid::Uuid::new_v4().to_string(),
                kind: ActionType::CopyFile,
                source: Some("files/turkish.txt".into()),
                destination: "Data/turkish.txt".into(),
                backup: true,
                expected_sha256: Some(hash_file(&extracted.join("files/turkish.txt")).unwrap()),
                options: Default::default(),
            },
        ];
        let handler = CopyHandler;
        let mut changes = Vec::new();
        for action in &actions {
            handler
                .apply(action, &extracted, &game, &backup, &mut changes)
                .expect("fixture action");
            assert_eq!(
                hash_file(&game.join(&action.destination)).unwrap(),
                action.expected_sha256.clone().unwrap()
            );
        }

        assert_eq!(
            fs::read(game.join("Data/original.txt")).unwrap(),
            b"TURKISH PATCH"
        );
        assert_eq!(
            fs::read(game.join("Data/turkish.txt")).unwrap(),
            b"TURKISH FILE"
        );
        assert!(verify_records(&game, &changes).expect("verify").valid);
        assert_eq!(changes.len(), 2);
        assert!(changes[0].backup_path.is_some());

        fs::write(game.join("Data/original.txt"), b"USER MODIFICATION").unwrap();
        let conflict = verify_records(&game, &changes).expect("conflict check");
        assert!(!conflict.valid);
        assert_eq!(conflict.conflicts, vec!["Data/original.txt"]);

        fs::write(game.join("Data/original.txt"), b"TURKISH PATCH").unwrap();
        rollback_changes(&game, &backup, &changes).expect("restore");
        assert_eq!(
            fs::read(game.join("Data/original.txt")).unwrap(),
            b"ORIGINAL"
        );
        assert!(!game.join("Data/turkish.txt").exists());
    }
}


