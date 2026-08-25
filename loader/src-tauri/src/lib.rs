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
    BackupInfo,
    DryRun,
    Installation,
    InstallationSummary,
    Manifest,
    PruneReport,
    UninstallReport,
    Verification,
};

use std::{
    any::Any,
    panic::{catch_unwind, AssertUnwindSafe},
    path::PathBuf,
};

use tauri::{
    AppHandle,
    Manager,
};

/// Çalışan loader sürümü.
///
/// Manifest içindeki minimum_loader_version kontrolü
/// bu değere göre yapılır.
fn loader_version_of(
    app: &AppHandle,
) -> String {
    app.package_info()
        .version
        .to_string()
}

/// Worker thread içinde oluşan panic mesajını okunabilir hale getirir.
fn panic_message(
    payload: Box<dyn Any + Send>,
) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }

    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_string();
    }

    "Bilinmeyen Rust panic".into()
}

/// Blocking dosya / disk / network işlemlerini Tauri ana thread'inden ayırır.
///
/// Ayrıca normal Rust error yanında panic durumunu da LoaderError'a çevirir.
/// Böylece mümkün olan durumlarda loader tamamen kapanmak yerine
/// kullanıcıya hata döndürebilir.
async fn run_blocking<T, F>(
    category: &'static str,
    action: F,
) -> Result<T, LoaderError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, LoaderError> + Send + 'static,
{
    let joined = tauri::async_runtime::spawn_blocking(move || {
        catch_unwind(
            AssertUnwindSafe(|| action())
        )
    })
    .await
    .map_err(|error| {
        let message = format!(
            "{category} worker thread'i beklenmedik şekilde sonlandı: {error}"
        );

        let _ = logging::event(
            "error",
            category,
            &message,
        );

        LoaderError::Other(message)
    })?;

    match joined {
        Ok(result) => {
            if let Err(ref error) = result {
                let _ = logging::event(
                    "error",
                    category,
                    &error.to_string(),
                );
            }

            result
        }

        Err(payload) => {
            let panic = panic_message(payload);

            let message = format!(
                "{category} işlemi sırasında beklenmeyen hata oluştu: {panic}"
            );

            let _ = logging::event(
                "error",
                category,
                &message,
            );

            Err(
                LoaderError::Other(message)
            )
        }
    }
}

#[tauri::command(rename_all = "camelCase")]
async fn validate_game_root(
    game_root: String,
    required_files: Vec<String>,
) -> Result<(), LoaderError> {
    run_blocking(
        "validate_game_root",
        move || {
            game_detection::validate_root(
                &PathBuf::from(game_root),
                &required_files,
            )
        },
    )
    .await
}

#[tauri::command(rename_all = "camelCase")]
async fn detect_game(
    steam_app_id: Option<String>,
    required_files: Vec<String>,
) -> Result<Option<String>, LoaderError> {
    run_blocking(
        "detect_game",
        move || {
            match steam_app_id {
                Some(id) => {
                    Ok(
                        game_detection::detect_steam(
                            &id,
                            &required_files,
                        )?
                        .map(|path| {
                            path.display().to_string()
                        })
                    )
                }

                None => Ok(None),
            }
        },
    )
    .await
}

#[tauri::command(rename_all = "camelCase")]
async fn dry_run_patch(
    manifest: Manifest,
    game_root: String,
) -> Result<DryRun, LoaderError> {
    run_blocking(
        "dry_run",
        move || {
            patch::engine::dry_run(
                &manifest,
                &PathBuf::from(game_root),
            )
        },
    )
    .await
}

/// Yama kurulumu.
///
/// İndirme, SHA-256, ZIP çıkarma, backup ve dosya kopyalama
/// worker thread üzerinde çalışır.
#[tauri::command(rename_all = "camelCase")]
async fn install_patch(
    app: AppHandle,
    manifest: Manifest,
    game_root: String,
    archive_url: String,
    force: Option<bool>,
) -> Result<Installation, LoaderError> {
    let version = loader_version_of(&app);
    let force = force.unwrap_or(false);

    let game_id = manifest.game.id;
    let patch_version = manifest.patch.version.clone();

    let _ = logging::event(
        "info",
        "install",
        &format!(
            "Kurulum command başladı: game_id={game_id}, patch={patch_version}"
        ),
    );

    let result = run_blocking(
        "install",
        move || {
            patch::engine::install(
                &app,
                &manifest,
                &PathBuf::from(game_root),
                &archive_url,
                &version,
                force,
            )
        },
    )
    .await;

    match &result {
        Ok(installation) => {
            let _ = logging::event(
                "info",
                "install",
                &format!(
                    "Kurulum command tamamlandı: game_id={}, patch={}",
                    installation.game_id,
                    installation.patch_version
                ),
            );
        }

        Err(error) => {
            let _ = logging::event(
                "error",
                "install",
                &format!(
                    "Kurulum command başarısız: {error}"
                ),
            );
        }
    }

    result
}

/// Yamayı kaldırır.
///
/// verify + hash + backup restore işlemleri worker thread'de çalışır.
#[tauri::command(rename_all = "camelCase")]
async fn uninstall_patch(
    game_id: u64,
    game_root: String,
    force: Option<bool>,
) -> Result<UninstallReport, LoaderError> {
    let force = force.unwrap_or(false);

    let _ = logging::event(
        "info",
        "uninstall",
        &format!(
            "Yama kaldırma başladı: game_id={game_id}, force={force}"
        ),
    );

    let result = run_blocking(
        "uninstall",
        move || {
            patch::engine::uninstall(
                game_id,
                &PathBuf::from(game_root),
                force,
            )
        },
    )
    .await;

    match &result {
        Ok(report) => {
            let _ = logging::event(
                "info",
                "uninstall",
                &format!(
                    "Yama kaldırma tamamlandı: game_id={game_id}, restored={}",
                    report.restored
                ),
            );
        }

        Err(error) => {
            let _ = logging::event(
                "error",
                "uninstall",
                &format!(
                    "Yama kaldırma başarısız: game_id={game_id}, error={error}"
                ),
            );
        }
    }

    result
}

/// Kurulu yamanın dosyalarını doğrular.
///
/// Büyük dosyaların SHA-256 işlemi worker thread'de gerçekleştirilir.
#[tauri::command(rename_all = "camelCase")]
async fn verify_installation(
    game_id: u64,
    game_root: String,
) -> Result<Verification, LoaderError> {
    let _ = logging::event(
        "info",
        "verify",
        &format!(
            "Dosya doğrulama başladı: game_id={game_id}"
        ),
    );

    let result = run_blocking(
        "verify",
        move || {
            patch::engine::verify_installation(
                game_id,
                &PathBuf::from(game_root),
            )
        },
    )
    .await;

    match &result {
        Ok(verification) => {
            let _ = logging::event(
                "info",
                "verify",
                &format!(
                    "Dosya doğrulama tamamlandı: game_id={game_id}, valid={}, checked={}, conflicts={}",
                    verification.valid,
                    verification.checked,
                    verification.conflicts.len()
                ),
            );
        }

        Err(error) => {
            let _ = logging::event(
                "error",
                "verify",
                &format!(
                    "Dosya doğrulama başarısız: game_id={game_id}, error={error}"
                ),
            );
        }
    }

    result
}

/// Diskteki aktif kurulum kayıtlarını döndürür.
#[tauri::command]
async fn list_installations(
) -> Result<Vec<InstallationSummary>, LoaderError> {
    run_blocking(
        "list_installations",
        backup::list_installations,
    )
    .await
}

#[tauri::command(rename_all = "camelCase")]
async fn installation_for_game(
    game_id: u64,
) -> Result<Option<Installation>, LoaderError> {
    run_blocking(
        "installation_for_game",
        move || {
            backup::find_installation(game_id)
        },
    )
    .await
}

#[tauri::command]
async fn list_backups(
) -> Result<Vec<BackupInfo>, LoaderError> {
    run_blocking(
        "list_backups",
        backup::list,
    )
    .await
}

#[tauri::command(rename_all = "camelCase")]
async fn clean_backup(
    backup_id: String,
) -> Result<(), LoaderError> {
    run_blocking(
        "clean_backup",
        move || {
            backup::clean(&backup_id)
        },
    )
    .await
}

/// Sahipsiz yedekleri ve indirme cache'ini temizler.
#[tauri::command]
async fn prune_storage(
) -> Result<PruneReport, LoaderError> {
    run_blocking(
        "prune_storage",
        backup::prune,
    )
    .await
}

#[tauri::command]
fn loader_version(
    app: AppHandle,
) -> Result<String, LoaderError> {
    Ok(
        loader_version_of(&app)
    )
}

/// Destek / sosyal bağlantılarını sistem tarayıcısında açar.
///
/// Sadece HTTPS kabul edilir.
#[tauri::command(rename_all = "camelCase")]
fn open_external(
    url: String,
) -> Result<(), LoaderError> {
    let parsed = reqwest::Url::parse(&url)
        .map_err(|_| {
            LoaderError::Other(
                "Geçersiz bağlantı.".into()
            )
        })?;

    if parsed.scheme() != "https" {
        return Err(
            LoaderError::Other(
                "Yalnız HTTPS bağlantıları açılabilir.".into()
            )
        );
    }

    let target = parsed.as_str().to_string();

    #[cfg(target_os = "windows")]
    let spawned = std::process::Command::new(
        "rundll32.exe"
    )
    .args([
        "url.dll,FileProtocolHandler",
        &target,
    ])
    .spawn();

    #[cfg(target_os = "macos")]
    let spawned = std::process::Command::new(
        "open"
    )
    .arg(&target)
    .spawn();

    #[cfg(all(
        not(target_os = "windows"),
        not(target_os = "macos")
    ))]
    let spawned = std::process::Command::new(
        "xdg-open"
    )
    .arg(&target)
    .spawn();

    spawned.map_err(|error| {
        LoaderError::Other(
            format!(
                "Bağlantı açılamadı: {error}"
            )
        )
    })?;

    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
async fn write_client_log(
    level: String,
    category: String,
    message: String,
) -> Result<(), LoaderError> {
    run_blocking(
        "client_log",
        move || {
            let allowed = [
                "debug",
                "info",
                "warning",
                "error",
            ];

            let normalized = if allowed.contains(
                &level.as_str()
            ) {
                level
            } else {
                "info".into()
            };

            logging::event(
                &normalized,
                &category,
                &message,
            )
        },
    )
    .await
}

#[tauri::command]
async fn load_access_token(
) -> Result<Option<String>, LoaderError> {
    run_blocking(
        "load_access_token",
        credential::load,
    )
    .await
}

#[tauri::command(rename_all = "camelCase")]
async fn save_access_token(
    token: String,
) -> Result<(), LoaderError> {
    run_blocking(
        "save_access_token",
        move || {
            credential::save(&token)
        },
    )
    .await
}

#[tauri::command]
async fn clear_access_token(
) -> Result<(), LoaderError> {
    run_blocking(
        "clear_access_token",
        credential::clear,
    )
    .await
}

pub fn run() {
    tauri::Builder::default()

        .plugin(
            tauri_plugin_dialog::init()
        )

        .plugin(
            tauri_plugin_updater::Builder::new()
                .build()
        )

        .invoke_handler(
            tauri::generate_handler![
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
            ]
        )

        .run(
            tauri::generate_context!()
        )

        .expect(
            "Tauri uygulaması başlatılamadı"
        );
}
