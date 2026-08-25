mod backup;
mod credential;
mod download;
mod error;
mod game_detection;
mod logging;
mod models;
mod patch;
mod security;
mod storage;

use error::LoaderError;
use models::{
    BackupInfo, DryRun, Installation, InstallationSummary, Manifest, PruneReport, UninstallReport,
    Verification,
};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Calisan loader surumu. Yamanin `minimum_loader_version` alani buna gore denetlenir.
fn loader_version_of(app: &AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command(rename_all = "camelCase")]
fn validate_game_root(game_root: String, required_files: Vec<String>) -> Result<(), LoaderError> {
    game_detection::validate_root(&PathBuf::from(game_root), &required_files)
}

#[tauri::command(rename_all = "camelCase")]
fn detect_game(
    steam_app_id: Option<String>,
    required_files: Vec<String>,
) -> Result<Option<String>, LoaderError> {
    match steam_app_id {
        Some(id) => Ok(
            game_detection::detect_steam(&id, &required_files)?.map(|p| p.display().to_string())
        ),
        None => Ok(None),
    }
}

#[tauri::command(rename_all = "camelCase")]
fn dry_run_patch(manifest: Manifest, game_root: String) -> Result<DryRun, LoaderError> {
    patch::engine::dry_run(&manifest, &PathBuf::from(game_root))
}

#[tauri::command(rename_all = "camelCase")]
fn install_patch(
    app: AppHandle,
    manifest: Manifest,
    game_root: String,
    archive_url: String,
    force: Option<bool>,
) -> Result<Installation, LoaderError> {
    let version = loader_version_of(&app);
    patch::engine::install(
        &app,
        &manifest,
        &PathBuf::from(game_root),
        &archive_url,
        &version,
        force.unwrap_or(false),
    )
}

#[tauri::command(rename_all = "camelCase")]
fn uninstall_patch(
    game_id: u64,
    game_root: String,
    force: Option<bool>,
) -> Result<UninstallReport, LoaderError> {
    patch::engine::uninstall(
        game_id,
        &PathBuf::from(game_root),
        force.unwrap_or(false),
    )
}

#[tauri::command(rename_all = "camelCase")]
fn verify_installation(game_id: u64, game_root: String) -> Result<Verification, LoaderError> {
    patch::engine::verify_installation(game_id, &PathBuf::from(game_root))
}

/// Diskteki kurulum kayitlari. Arayuz "kurulu mu / guncelleme var mi"
/// bilgisini artik tarayici depolamasindan degil buradan okur.
#[tauri::command]
fn list_installations() -> Result<Vec<InstallationSummary>, LoaderError> {
    backup::list_installations()
}

#[tauri::command(rename_all = "camelCase")]
fn installation_for_game(game_id: u64) -> Result<Option<Installation>, LoaderError> {
    backup::find_installation(game_id)
}

#[tauri::command]
fn list_backups() -> Result<Vec<BackupInfo>, LoaderError> {
    backup::list()
}

#[tauri::command(rename_all = "camelCase")]
fn clean_backup(backup_id: String) -> Result<(), LoaderError> {
    backup::clean(&backup_id)
}

/// Aktif kuruluma bagli olmayan yedekleri ve indirme onbellegini temizler.
#[tauri::command]
fn prune_storage() -> Result<PruneReport, LoaderError> {
    backup::prune()
}

#[tauri::command]
fn loader_version(app: AppHandle) -> Result<String, LoaderError> {
    Ok(loader_version_of(&app))
}

/// Destek/sosyal baglantilari sistem tarayicisinda acar.
/// Sadece https adreslerine izin verilir ve arguman shell'e verilmez.
#[tauri::command(rename_all = "camelCase")]
fn open_external(url: String) -> Result<(), LoaderError> {
    let parsed = reqwest::Url::parse(&url).map_err(|_| LoaderError::Other("Geçersiz bağlantı.".into()))?;
    if parsed.scheme() != "https" {
        return Err(LoaderError::Other("Yalnız HTTPS bağlantıları açılabilir.".into()));
    }
    let target = parsed.as_str().to_string();
    #[cfg(target_os = "windows")]
    let spawned = std::process::Command::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", &target])
        .spawn();
    #[cfg(target_os = "macos")]
    let spawned = std::process::Command::new("open").arg(&target).spawn();
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let spawned = std::process::Command::new("xdg-open").arg(&target).spawn();
    spawned.map_err(|_| LoaderError::Other("Bağlantı açılamadı.".into()))?;
    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
fn write_client_log(level: String, category: String, message: String) -> Result<(), LoaderError> {
    let allowed = ["debug", "info", "warning", "error"];
    let normalized = if allowed.contains(&level.as_str()) {
        level
    } else {
        "info".into()
    };
    logging::event(&normalized, &category, &message)
}

#[tauri::command]
fn load_access_token() -> Result<Option<String>, LoaderError> {
    credential::load()
}

#[tauri::command(rename_all = "camelCase")]
fn save_access_token(token: String) -> Result<(), LoaderError> {
    credential::save(&token)
}

#[tauri::command]
fn clear_access_token() -> Result<(), LoaderError> {
    credential::clear()
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            validate_game_root,
            detect_game,
            dry_run_patch,
            install_patch,
            uninstall_patch,
            verify_installation,
            list_installations,
            installation_for_game,
            list_backups,
            clean_backup,
            prune_storage,
            loader_version,
            open_external,
            load_access_token,
            save_access_token,
            clear_access_token,
            write_client_log
        ])
        .run(tauri::generate_context!())
        .expect("Tauri uygulaması başlatılamadı");
}
