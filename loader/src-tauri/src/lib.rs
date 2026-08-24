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
use models::{BackupInfo, DryRun, Installation, Manifest, Verification};
use std::path::PathBuf;
use tauri::AppHandle;

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
    auth_token: String,
) -> Result<Installation, LoaderError> {
    let _ = auth_token;
    patch::engine::install(&app, &manifest, &PathBuf::from(game_root), &archive_url)
}

#[tauri::command(rename_all = "camelCase")]
fn uninstall_patch(game_id: u64, game_root: String) -> Result<(), LoaderError> {
    patch::engine::uninstall(game_id, &PathBuf::from(game_root))
}

#[tauri::command(rename_all = "camelCase")]
fn verify_installation(game_id: u64, game_root: String) -> Result<Verification, LoaderError> {
    patch::engine::verify_installation(game_id, &PathBuf::from(game_root))
}

#[tauri::command]
fn list_backups() -> Result<Vec<BackupInfo>, LoaderError> {
    backup::list()
}

#[tauri::command(rename_all = "camelCase")]
fn clean_backup(backup_id: String) -> Result<(), LoaderError> {
    backup::clean(&backup_id)
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
            list_backups,
            clean_backup,
            load_access_token,
            save_access_token,
            clear_access_token,
            write_client_log
        ])
        .run(tauri::generate_context!())
        .expect("Tauri uygulaması başlatılamadı");
}
