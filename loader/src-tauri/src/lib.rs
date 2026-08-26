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

use std::{
    env,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;
use walkdir::WalkDir;


// ============================================================
// LOADER VERSION
// ============================================================

/// Calisan loader surumu.
/// Yamanin `minimum_loader_version` alani buna gore denetlenir.
fn loader_version_of(app: &AppHandle) -> String {
    app.package_info().version.to_string()
}


// ============================================================
// PLAYSTATION EMULATOR
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PsPlatform {
    Ps1,
    Ps2,
}

impl PsPlatform {
    fn parse(value: &str) -> Result<Self, LoaderError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "ps1" | "psx" | "playstation1" => Ok(Self::Ps1),
            "ps2" | "playstation2" => Ok(Self::Ps2),
            _ => Err(LoaderError::Other(
                "Desteklenmeyen PlayStation platformu.".into(),
            )),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Ps1 => "PlayStation 1",
            Self::Ps2 => "PlayStation 2",
        }
    }

    fn emulator_name(self) -> &'static str {
        match self {
            Self::Ps1 => "DuckStation",
            Self::Ps2 => "PCSX2",
        }
    }

    fn env_variable(self) -> &'static str {
        match self {
            Self::Ps1 => "ANIMUS_DUCKSTATION_PATH",
            Self::Ps2 => "ANIMUS_PCSX2_PATH",
        }
    }

    fn config_file(self) -> &'static str {
        match self {
            Self::Ps1 => "duckstation.path",
            Self::Ps2 => "pcsx2.path",
        }
    }

    fn allowed_extensions(self) -> &'static [&'static str] {
        match self {
            Self::Ps1 => &[
                "cue",
                "bin",
                "chd",
                "iso",
                "img",
                "mdf",
                "pbp",
            ],

            Self::Ps2 => &[
                "iso",
                "chd",
                "cso",
                "bin",
                "cue",
            ],
        }
    }
}


// ============================================================
// PS GAME FILE VALIDATION
// ============================================================

fn validate_ps_game_file(
    platform: PsPlatform,
    path: &Path,
) -> Result<PathBuf, LoaderError> {
    if !path.exists() {
        return Err(LoaderError::Other(
            "Seçilen oyun dosyası bulunamadı.".into(),
        ));
    }

    if !path.is_file() {
        return Err(LoaderError::Other(
            "Seçilen yol bir oyun dosyası değil.".into(),
        ));
    }

    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if !platform
        .allowed_extensions()
        .iter()
        .any(|allowed| extension == *allowed)
    {
        return Err(LoaderError::Other(format!(
            "{} için desteklenmeyen oyun dosyası: .{}\n\nDesteklenen formatlar: {}",
            platform.label(),
            extension,
            platform.allowed_extensions().join(", ")
        )));
    }

    path.canonicalize().map_err(|error| {
        LoaderError::Other(format!(
            "Oyun dosyası yolu çözümlenemedi: {error}"
        ))
    })
}


// ============================================================
// SELECT PS GAME FILE
// ============================================================

#[tauri::command(rename_all = "camelCase")]
async fn select_ps_game_file(
    app: AppHandle,
    platform: String,
) -> Result<Option<String>, LoaderError> {
    let platform = PsPlatform::parse(&platform)?;

    let dialog = match platform {
        PsPlatform::Ps1 => app
            .dialog()
            .file()
            .add_filter(
                "PlayStation 1 Oyunları",
                &[
                    "cue",
                    "bin",
                    "chd",
                    "iso",
                    "img",
                    "mdf",
                    "pbp",
                ],
            ),

        PsPlatform::Ps2 => app
            .dialog()
            .file()
            .add_filter(
                "PlayStation 2 Oyunları",
                &[
                    "iso",
                    "chd",
                    "cso",
                    "bin",
                    "cue",
                ],
            ),
    };

    let selected = dialog.blocking_pick_file();

    let Some(selected) = selected else {
        return Ok(None);
    };

    let path = selected.into_path().map_err(|error| {
        LoaderError::Other(format!(
            "Seçilen oyun dosyasının yolu okunamadı: {error}"
        ))
    })?;

    let path = validate_ps_game_file(platform, &path)?;

    Ok(Some(path.display().to_string()))
}


// ============================================================
// EMULATOR CONFIG DIRECTORY
// ============================================================

fn emulator_config_directory() -> Result<PathBuf, LoaderError> {
    let root = dirs::data_local_dir()
        .ok_or_else(|| {
            LoaderError::Other(
                "Windows LocalAppData klasörü bulunamadı.".into(),
            )
        })?
        .join("AnimusPatchLoader")
        .join("emulators");

    fs::create_dir_all(&root).map_err(|error| {
        LoaderError::Other(format!(
            "Emülatör ayar klasörü oluşturulamadı: {error}"
        ))
    })?;

    Ok(root)
}


// ============================================================
// SAVED EMULATOR PATH
// ============================================================

fn saved_emulator_path(
    platform: PsPlatform,
) -> Option<PathBuf> {
    let config = emulator_config_directory().ok()?;
    let path_file = config.join(platform.config_file());

    let content = fs::read_to_string(path_file).ok()?;
    let path = PathBuf::from(content.trim());

    if emulator_executable_valid(platform, &path) {
        Some(path)
    } else {
        None
    }
}


fn save_emulator_path(
    platform: PsPlatform,
    emulator: &Path,
) -> Result<(), LoaderError> {
    let config = emulator_config_directory()?;

    fs::write(
        config.join(platform.config_file()),
        emulator.display().to_string(),
    )
    .map_err(|error| {
        LoaderError::Other(format!(
            "Emülatör yolu kaydedilemedi: {error}"
        ))
    })
}


// ============================================================
// EMULATOR EXECUTABLE VALIDATION
// ============================================================

fn emulator_executable_valid(
    platform: PsPlatform,
    path: &Path,
) -> bool {
    if !path.is_file() {
        return false;
    }

    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match platform {
        PsPlatform::Ps1 => {
            file_name.contains("duckstation")
        }

        PsPlatform::Ps2 => {
            file_name.contains("pcsx2")
        }
    }
}


// ============================================================
// ADD EMULATOR CANDIDATE
// ============================================================

fn push_candidate(
    candidates: &mut Vec<PathBuf>,
    base: Option<PathBuf>,
    relative: &str,
) {
    if let Some(base) = base {
        candidates.push(base.join(relative));
    }
}


// ============================================================
// COMMON EMULATOR LOCATIONS
// ============================================================

fn emulator_candidates(
    app: &AppHandle,
    platform: PsPlatform,
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    // --------------------------------------------------------
    // 0. Animus installer ile birlikte gelen emulator dosyalari
    // --------------------------------------------------------
    //
    // Tauri resource_dir Windows'ta ana .exe'nin bulundugu dizindir.
    // tauri.conf.json icinde emulators klasoru resource olarak paketlenince
    // OYNA tusu hicbir dosya secme penceresi acmadan buradaki emulatoru kullanir.
    if let Ok(resource_dir) = app.path().resource_dir() {
        match platform {
            PsPlatform::Ps1 => {
                candidates.push(
                    resource_dir
                        .join("emulators")
                        .join("duckstation")
                        .join("duckstation-qt-x64-ReleaseLTCG.exe"),
                );

                candidates.push(
                    resource_dir
                        .join("emulators")
                        .join("duckstation")
                        .join("duckstation-qt.exe"),
                );

                candidates.push(
                    resource_dir
                        .join("emulators")
                        .join("duckstation")
                        .join("duckstation.exe"),
                );
            }

            PsPlatform::Ps2 => {
                candidates.push(
                    resource_dir
                        .join("emulators")
                        .join("pcsx2")
                        .join("pcsx2-qt.exe"),
                );

                candidates.push(
                    resource_dir
                        .join("emulators")
                        .join("pcsx2")
                        .join("pcsx2-qtx64-avx2.exe"),
                );

                candidates.push(
                    resource_dir
                        .join("emulators")
                        .join("pcsx2")
                        .join("pcsx2.exe"),
                );
            }
        }
    }

    // --------------------------------------------------------
    // 1. Environment override
    // --------------------------------------------------------

    if let Some(value) = env::var_os(platform.env_variable()) {
        candidates.push(PathBuf::from(value));
    }

    // --------------------------------------------------------
    // 2. Animus ile beraber gelen emulator klasoru
    // --------------------------------------------------------

    if let Ok(current_exe) = env::current_exe() {
        if let Some(app_dir) = current_exe.parent() {
            match platform {
                PsPlatform::Ps1 => {
                    candidates.push(
                        app_dir
                            .join("emulators")
                            .join("duckstation")
                            .join("duckstation-qt-x64-ReleaseLTCG.exe"),
                    );

                    candidates.push(
                        app_dir
                            .join("emulators")
                            .join("duckstation")
                            .join("duckstation-qt.exe"),
                    );

                    candidates.push(
                        app_dir
                            .join("emulators")
                            .join("duckstation")
                            .join("duckstation.exe"),
                    );

                    candidates.push(
                        app_dir
                            .join("emulators")
                            .join("ps1")
                            .join("duckstation-qt-x64-ReleaseLTCG.exe"),
                    );

                    candidates.push(
                        app_dir
                            .join("DuckStation")
                            .join("duckstation-qt-x64-ReleaseLTCG.exe"),
                    );
                }

                PsPlatform::Ps2 => {
                    candidates.push(
                        app_dir
                            .join("emulators")
                            .join("pcsx2")
                            .join("pcsx2-qt.exe"),
                    );

                    candidates.push(
                        app_dir
                            .join("emulators")
                            .join("pcsx2")
                            .join("pcsx2-qtx64-avx2.exe"),
                    );

                    candidates.push(
                        app_dir
                            .join("emulators")
                            .join("pcsx2")
                            .join("pcsx2.exe"),
                    );

                    candidates.push(
                        app_dir
                            .join("emulators")
                            .join("ps2")
                            .join("pcsx2-qt.exe"),
                    );

                    candidates.push(
                        app_dir
                            .join("PCSX2")
                            .join("pcsx2-qt.exe"),
                    );
                }
            }
        }
    }

    // --------------------------------------------------------
    // 3. LocalAppData
    // --------------------------------------------------------

    let local = dirs::data_local_dir();

    match platform {
        PsPlatform::Ps1 => {
            push_candidate(
                &mut candidates,
                local.clone(),
                r"Programs\DuckStation\duckstation-qt-x64-ReleaseLTCG.exe",
            );

            push_candidate(
                &mut candidates,
                local.clone(),
                r"Programs\DuckStation\duckstation-qt.exe",
            );

            push_candidate(
                &mut candidates,
                local.clone(),
                r"DuckStation\duckstation-qt-x64-ReleaseLTCG.exe",
            );

            push_candidate(
                &mut candidates,
                local,
                r"DuckStation\duckstation-qt.exe",
            );
        }

        PsPlatform::Ps2 => {
            push_candidate(
                &mut candidates,
                local.clone(),
                r"Programs\PCSX2\pcsx2-qt.exe",
            );

            push_candidate(
                &mut candidates,
                local.clone(),
                r"Programs\PCSX2\pcsx2-qtx64-avx2.exe",
            );

            push_candidate(
                &mut candidates,
                local,
                r"PCSX2\pcsx2-qt.exe",
            );
        }
    }

    // --------------------------------------------------------
    // 4. Program Files
    // --------------------------------------------------------

    if let Some(program_files) = env::var_os("ProgramFiles") {
        let base = PathBuf::from(program_files);

        match platform {
            PsPlatform::Ps1 => {
                candidates.push(
                    base.join("DuckStation")
                        .join("duckstation-qt-x64-ReleaseLTCG.exe"),
                );

                candidates.push(
                    base.join("DuckStation")
                        .join("duckstation-qt.exe"),
                );
            }

            PsPlatform::Ps2 => {
                candidates.push(
                    base.join("PCSX2")
                        .join("pcsx2-qt.exe"),
                );

                candidates.push(
                    base.join("PCSX2")
                        .join("pcsx2-qtx64-avx2.exe"),
                );

                candidates.push(
                    base.join("PCSX2")
                        .join("pcsx2.exe"),
                );
            }
        }
    }

    // --------------------------------------------------------
    // 5. Program Files (x86)
    // --------------------------------------------------------

    if let Some(program_files) = env::var_os("ProgramFiles(x86)") {
        let base = PathBuf::from(program_files);

        match platform {
            PsPlatform::Ps1 => {
                candidates.push(
                    base.join("DuckStation")
                        .join("duckstation-qt-x64-ReleaseLTCG.exe"),
                );

                candidates.push(
                    base.join("DuckStation")
                        .join("duckstation-qt.exe"),
                );
            }

            PsPlatform::Ps2 => {
                candidates.push(
                    base.join("PCSX2")
                        .join("pcsx2-qt.exe"),
                );

                candidates.push(
                    base.join("PCSX2")
                        .join("pcsx2.exe"),
                );
            }
        }
    }

    candidates
}


// ============================================================
// DUCKSTATION AUTOMATIC INSTALL
// ============================================================

const DUCKSTATION_RELEASE_API: &str =
    "https://api.github.com/repos/stenzek/duckstation/releases/tags/latest";
const DUCKSTATION_WINDOWS_ASSET: &str =
    "duckstation-windows-x64-release.zip";

fn emulator_managed_directory(
    platform: PsPlatform,
) -> Result<PathBuf, LoaderError> {
    let root = dirs::data_local_dir()
        .ok_or_else(|| {
            LoaderError::Other(
                "Windows LocalAppData klasörü bulunamadı.".into(),
            )
        })?
        .join("AnimusPatchLoader")
        .join("emulators");

    fs::create_dir_all(&root).map_err(|error| {
        LoaderError::Other(format!(
            "Animus emülatör klasörü oluşturulamadı: {error}"
        ))
    })?;

    Ok(match platform {
        PsPlatform::Ps1 => root.join("duckstation"),
        PsPlatform::Ps2 => root.join("pcsx2"),
    })
}

fn github_client() -> Result<reqwest::blocking::Client, LoaderError> {
    reqwest::blocking::Client::builder()
        .user_agent("Animus-Patch-Loader/0.1")
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|error| {
            LoaderError::Other(format!(
                "Emülatör indirme istemcisi hazırlanamadı: {error}"
            ))
        })
}

fn duckstation_latest_asset(
    client: &reqwest::blocking::Client,
) -> Result<(String, Option<String>), LoaderError> {
    let response = client
        .get(DUCKSTATION_RELEASE_API)
        .header("Accept", "application/vnd.github+json")
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| {
            LoaderError::Other(format!(
                "DuckStation sürüm bilgisi alınamadı: {error}"
            ))
        })?;

    let body = response.text().map_err(|error| {
        LoaderError::Other(format!(
            "DuckStation sürüm yanıtı okunamadı: {error}"
        ))
    })?;

    let release: serde_json::Value =
        serde_json::from_str(&body).map_err(|error| {
            LoaderError::Other(format!(
                "DuckStation sürüm bilgisi JSON olarak çözümlenemedi: {error}"
            ))
        })?;

    let assets = release
        .get("assets")
        .and_then(|value| value.as_array())
        .ok_or_else(|| {
            LoaderError::Other(
                "DuckStation yayın dosyaları bulunamadı.".into(),
            )
        })?;

    let asset = assets
        .iter()
        .find(|asset| {
            asset
                .get("name")
                .and_then(|value| value.as_str())
                .map(|name| name.eq_ignore_ascii_case(DUCKSTATION_WINDOWS_ASSET))
                .unwrap_or(false)
        })
        .ok_or_else(|| {
            LoaderError::Other(format!(
                "Resmî DuckStation Windows paketi bulunamadı: {DUCKSTATION_WINDOWS_ASSET}"
            ))
        })?;

    let url = asset
        .get("browser_download_url")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            LoaderError::Other(
                "DuckStation indirme adresi bulunamadı.".into(),
            )
        })?
        .to_string();

    // GitHub yeni release API'lerinde asset digest alanini sha256:<hash>
    // biciminde dondurur. Alan yoksa indirme yine resmî GitHub URL'sinden gelir.
    let expected_sha256 = asset
        .get("digest")
        .and_then(|value| value.as_str())
        .and_then(|value| value.strip_prefix("sha256:"))
        .map(str::to_string);

    Ok((url, expected_sha256))
}

fn extract_emulator_zip(
    archive_path: &Path,
    destination: &Path,
) -> Result<(), LoaderError> {
    let file = fs::File::open(archive_path).map_err(|error| {
        LoaderError::Other(format!(
            "Emülatör arşivi açılamadı: {error}"
        ))
    })?;

    let mut archive = zip::ZipArchive::new(file).map_err(|error| {
        LoaderError::Other(format!(
            "Emülatör ZIP arşivi okunamadı: {error}"
        ))
    })?;

    fs::create_dir_all(destination).map_err(|error| {
        LoaderError::Other(format!(
            "Emülatör geçici klasörü oluşturulamadı: {error}"
        ))
    })?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| {
            LoaderError::Other(format!(
                "Emülatör ZIP girdisi okunamadı: {error}"
            ))
        })?;

        let relative = entry
            .enclosed_name()
            .ok_or_else(|| {
                LoaderError::Other(
                    "Emülatör arşivinde güvensiz dosya yolu bulundu.".into(),
                )
            })?;

        let output = destination.join(relative);

        if entry.is_dir() {
            fs::create_dir_all(&output).map_err(|error| {
                LoaderError::Other(format!(
                    "Emülatör klasörü oluşturulamadı: {error}"
                ))
            })?;
            continue;
        }

        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                LoaderError::Other(format!(
                    "Emülatör alt klasörü oluşturulamadı: {error}"
                ))
            })?;
        }

        let mut target = fs::File::create(&output).map_err(|error| {
            LoaderError::Other(format!(
                "Emülatör dosyası oluşturulamadı: {error}"
            ))
        })?;

        std::io::copy(&mut entry, &mut target).map_err(|error| {
            LoaderError::Other(format!(
                "Emülatör dosyası çıkarılamadı: {error}"
            ))
        })?;
    }

    Ok(())
}

fn duckstation_executable_in(
    root: &Path,
) -> Option<PathBuf> {
    [
        "duckstation-qt-x64-ReleaseLTCG.exe",
        "duckstation-qt.exe",
        "duckstation.exe",
    ]
    .into_iter()
    .map(|name| root.join(name))
    .find(|path| emulator_executable_valid(PsPlatform::Ps1, path))
}

fn install_duckstation_automatically(
) -> Result<PathBuf, LoaderError> {
    let client = github_client()?;
    let (download_url, expected_sha256) =
        duckstation_latest_asset(&client)?;

    let target = emulator_managed_directory(PsPlatform::Ps1)?;
    let parent = target.parent().ok_or_else(|| {
        LoaderError::Other(
            "Animus emülatör üst klasörü bulunamadı.".into(),
        )
    })?;

    let archive_path = parent.join("duckstation-download.zip");
    let staging = parent.join("duckstation-installing");

    let mut response = client
        .get(&download_url)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| {
            LoaderError::Other(format!(
                "DuckStation otomatik indirilemedi: {error}"
            ))
        })?;

    let mut archive_file = fs::File::create(&archive_path).map_err(|error| {
        LoaderError::Other(format!(
            "DuckStation indirme dosyası oluşturulamadı: {error}"
        ))
    })?;

    std::io::copy(&mut response, &mut archive_file).map_err(|error| {
        LoaderError::Other(format!(
            "DuckStation indirilemedi: {error}"
        ))
    })?;

    drop(archive_file);

    if let Some(expected) = expected_sha256 {
        let actual = crate::storage::hash_file(&archive_path)?;

        if !actual.eq_ignore_ascii_case(&expected) {
            let _ = fs::remove_file(&archive_path);

            return Err(LoaderError::Other(format!(
                "DuckStation indirmesi bütünlük kontrolünden geçemedi. Beklenen SHA-256: {expected}, bulunan: {actual}"
            )));
        }
    }

    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(|error| {
            LoaderError::Other(format!(
                "Eski DuckStation geçici klasörü temizlenemedi: {error}"
            ))
        })?;
    }

    extract_emulator_zip(
        &archive_path,
        &staging,
    )?;

    // DuckStation portable mod: ayarlari Animus'un yonettigi emulator
    // klasorunde tutar; Windows genel kurulumuna ihtiyac duymaz.
    fs::write(
        staging.join("portable.txt"),
        b"",
    )
    .map_err(|error| {
        LoaderError::Other(format!(
            "DuckStation portable ayarı oluşturulamadı: {error}"
        ))
    })?;

    let staged_executable =
        duckstation_executable_in(&staging)
            .ok_or_else(|| {
                LoaderError::Other(
                    "İndirilen DuckStation paketinde çalıştırılabilir dosya bulunamadı.".into(),
                )
            })?;

    let executable_name = staged_executable
        .file_name()
        .ok_or_else(|| {
            LoaderError::Other(
                "DuckStation çalıştırılabilir dosya adı okunamadı.".into(),
            )
        })?
        .to_os_string();

    if target.exists() {
        fs::remove_dir_all(&target).map_err(|error| {
            LoaderError::Other(format!(
                "Eski DuckStation kurulumu temizlenemedi: {error}"
            ))
        })?;
    }

    fs::rename(
        &staging,
        &target,
    )
    .map_err(|error| {
        LoaderError::Other(format!(
            "DuckStation kurulumu tamamlanamadı: {error}"
        ))
    })?;

    let _ = fs::remove_file(&archive_path);

    let executable = target
        .join(executable_name)
        .canonicalize()
        .unwrap_or_else(|_| target.join("duckstation-qt-x64-ReleaseLTCG.exe"));

    if !emulator_executable_valid(
        PsPlatform::Ps1,
        &executable,
    ) {
        return Err(LoaderError::Other(
            "DuckStation indirildi ancak çalıştırılabilir dosya doğrulanamadı.".into(),
        ));
    }

    save_emulator_path(
        PsPlatform::Ps1,
        &executable,
    )?;

    let _ = logging::event(
        "info",
        "emulator",
        &format!(
            "DuckStation resmî GitHub paketinden otomatik kuruldu: {}",
            executable.display()
        ),
    );

    Ok(executable)
}


// ============================================================
// FIND EMULATOR
// ============================================================

fn find_or_select_emulator(
    app: &AppHandle,
    platform: PsPlatform,
) -> Result<PathBuf, LoaderError> {
    // Daha once bulunan ve halen gecerli olan yolu kullan.
    if let Some(saved) = saved_emulator_path(platform) {
        return Ok(saved);
    }

    // Animus resource klasoru ve sistemde kurulu yaygin konumlari ara.
    for candidate in emulator_candidates(app, platform) {
        if emulator_executable_valid(platform, &candidate) {
            let canonical = candidate
                .canonicalize()
                .unwrap_or(candidate);

            let _ = save_emulator_path(
                platform,
                &canonical,
            );

            return Ok(canonical);
        }
    }

    match platform {
        // PS1 icin artik kullaniciya EXE sectirmiyoruz.
        // DuckStation resmî GitHub release paketinden otomatik indirilir.
        PsPlatform::Ps1 => install_duckstation_automatically(),

        // PS2 otomatik kurulumunu bir sonraki adimda PCSX2'nin 7z paketine
        // guvenli extraction ekleyerek tamamlayacagiz.
        PsPlatform::Ps2 => Err(LoaderError::Other(
            "PCSX2 bulunamadı. Animus otomatik PCSX2 kurulumu henüz etkin değil.".into(),
        )),
    }
}


// ============================================================
// LAUNCH PS GAME
// ============================================================

#[tauri::command(rename_all = "camelCase")]
async fn launch_ps_game(
    app: AppHandle,
    platform: String,
    game_path: String,
) -> Result<(), LoaderError> {
    let platform = PsPlatform::parse(&platform)?;

    let game_path = validate_ps_game_file(
        platform,
        &PathBuf::from(game_path),
    )?;

    let emulator = find_or_select_emulator(
        &app,
        platform,
    )?;

    let mut command = Command::new(&emulator);

    if let Some(parent) = emulator.parent() {
        command.current_dir(parent);
    }

    match platform {
        PsPlatform::Ps1 => {
            command
                .arg("-batch")
                .arg("-fullscreen")
                .arg("--")
                .arg(&game_path);
        }

        PsPlatform::Ps2 => {
            command
                .arg("-batch")
                .arg("-fullscreen")
                .arg("--")
                .arg(&game_path);
        }
    }

    command.spawn().map_err(|error| {
        LoaderError::Other(format!(
            "{} başlatılamadı: {error}",
            platform.emulator_name()
        ))
    })?;

    let game_name = game_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("bilinmeyen oyun");

    let _ = logging::event(
        "info",
        "emulator",
        &format!(
            "{} oyunu başlatıldı: {}",
            platform.label(),
            game_name
        ),
    );

    Ok(())
}


// ============================================================
// MANAGED PLAYSTATION GAME ROOT
// ============================================================

/// PS1/PS2 paketleri normal bir PC oyun klasörüne kurulmaz.
/// Animus kendi yönetilen klasörünü oluşturur ve kurulum motoruna bu yolu verir.
/// Kullanıcıdan klasör seçmesi istenmez.
#[tauri::command(rename_all = "camelCase")]
fn prepare_ps_game_root(
    game_id: u64,
) -> Result<String, LoaderError> {
    if game_id == 0 {
        return Err(LoaderError::Other(
            "Geçersiz PlayStation oyun kimliği.".into(),
        ));
    }

    let root = dirs::data_local_dir()
        .ok_or_else(|| {
            LoaderError::Other(
                "Windows LocalAppData klasörü bulunamadı.".into(),
            )
        })?
        .join("AnimusPatchLoader")
        .join("emulated-games")
        .join(format!("game-{game_id}"));

    fs::create_dir_all(&root).map_err(|error| {
        LoaderError::Other(format!(
            "PlayStation oyun klasörü oluşturulamadı: {error}"
        ))
    })?;

    let resolved = root
        .canonicalize()
        .unwrap_or(root);

    Ok(resolved.display().to_string())
}


// ============================================================
// FIND PS IMAGE INSIDE INSTALLED ANIMUS PACKAGE
// ============================================================

fn find_installed_ps_game_image(
    game_root: &Path,
    platform: PsPlatform,
) -> Result<PathBuf, LoaderError> {
    if !game_root.is_dir() {
        return Err(LoaderError::Other(
            "Kurulu PlayStation oyun klasörü bulunamadı.".into(),
        ));
    }

    let root = game_root.canonicalize().map_err(|error| {
        LoaderError::Other(format!(
            "PlayStation oyun klasörü okunamadı: {error}"
        ))
    })?;

    // Sıra önemli:
    // PS1'de CUE varsa BIN yerine CUE açılır.
    // PS2'de ISO/CHD önceliklidir.
    let preferred: &[&str] = match platform {
        PsPlatform::Ps1 => &[
            "cue",
            "chd",
            "pbp",
            "iso",
            "img",
            "mdf",
            "bin",
        ],
        PsPlatform::Ps2 => &[
            "iso",
            "chd",
            "cso",
            "cue",
            "bin",
        ],
    };

    for extension in preferred {
        for entry in WalkDir::new(&root)
            .follow_links(false)
            .max_depth(10)
            .into_iter()
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();

            let current_extension = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();

            if current_extension != *extension {
                continue;
            }

            let canonical = path.canonicalize().map_err(|error| {
                LoaderError::Other(format!(
                    "PlayStation oyun imajı okunamadı: {error}"
                ))
            })?;

            // Symlink/junction ile kurulum klasörünün dışına kaçılmasına izin verme.
            if !canonical.starts_with(&root) {
                continue;
            }

            return Ok(canonical);
        }
    }

    Err(LoaderError::Other(match platform {
        PsPlatform::Ps1 => {
            "Kurulan paketin içinde PS1 oyun dosyası bulunamadı. CUE/CHD/PBP/ISO/BIN aranmıştır."
                .into()
        }
        PsPlatform::Ps2 => {
            "Kurulan paketin içinde PS2 oyun dosyası bulunamadı. ISO/CHD/CSO/CUE/BIN aranmıştır."
                .into()
        }
    }))
}


/// Kullanıcıdan ISO seçmez.
/// MediaFire paketinin Animus tarafından kurulduğu klasörü tarar ve
/// bulunan oyun imajını uygun emülatör ile çalıştırır.
#[tauri::command(rename_all = "camelCase")]
async fn launch_installed_ps_game(
    app: AppHandle,
    platform: String,
    game_root: String,
) -> Result<String, LoaderError> {
    let platform = PsPlatform::parse(&platform)?;

    let image = find_installed_ps_game_image(
        &PathBuf::from(game_root),
        platform,
    )?;

    let emulator = find_or_select_emulator(
        &app,
        platform,
    )?;

    let mut command = Command::new(&emulator);

    if let Some(parent) = emulator.parent() {
        command.current_dir(parent);
    }

    command
        .arg("-batch")
        .arg("-fullscreen")
        .arg(&image);

    command.spawn().map_err(|error| {
        LoaderError::Other(format!(
            "{} başlatılamadı: {error}",
            platform.emulator_name()
        ))
    })?;

    let game_name = image
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("PlayStation oyunu")
        .to_string();

    let _ = logging::event(
        "info",
        "emulator",
        &format!(
            "{} kurulu Animus paketinden başlatıldı: {}",
            platform.label(),
            game_name
        ),
    );

    Ok(game_name)
}


// ============================================================
// GAME ROOT
// ============================================================

#[tauri::command(rename_all = "camelCase")]
fn validate_game_root(
    game_root: String,
    required_files: Vec<String>,
) -> Result<(), LoaderError> {
    game_detection::validate_root(
        &PathBuf::from(game_root),
        &required_files,
    )
}


#[tauri::command(rename_all = "camelCase")]
fn detect_game(
    steam_app_id: Option<String>,
    required_files: Vec<String>,
) -> Result<Option<String>, LoaderError> {
    match steam_app_id {
        Some(id) => Ok(
            game_detection::detect_steam(
                &id,
                &required_files,
            )?
            .map(|p| p.display().to_string()),
        ),

        None => Ok(None),
    }
}


// ============================================================
// PATCH DRY RUN
// ============================================================

#[tauri::command(rename_all = "camelCase")]
fn dry_run_patch(
    manifest: Manifest,
    game_root: String,
) -> Result<DryRun, LoaderError> {
    patch::engine::dry_run(
        &manifest,
        &PathBuf::from(game_root),
    )
}


// ============================================================
// INSTALL PATCH
// ============================================================

/// Indirme, ZIP cikarma, hash ve kopyalama blocking islemlerdir.
/// Tauri ana thread'ini kilitlememek icin worker thread'de calisir.
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

    let result = tauri::async_runtime::spawn_blocking(move || {
        patch::engine::install(
            &app,
            &manifest,
            &PathBuf::from(game_root),
            &archive_url,
            &version,
            force,
        )
    })
    .await
    .map_err(|error| {
        LoaderError::Other(format!(
            "Kurulum worker thread'i sonlandi: {error}"
        ))
    })?;

    if let Err(ref error) = result {
        let _ = logging::event(
            "error",
            "install",
            &format!(
                "Kurulum basarisiz: {error}"
            ),
        );
    }

    result
}


// ============================================================
// UNINSTALL PATCH
// ============================================================

#[tauri::command(rename_all = "camelCase")]
async fn uninstall_patch(
    game_id: u64,
    game_root: String,
    force: Option<bool>,
) -> Result<UninstallReport, LoaderError> {
    let force = force.unwrap_or(false);

    let result = tauri::async_runtime::spawn_blocking(move || {
        patch::engine::uninstall(
            game_id,
            &PathBuf::from(game_root),
            force,
        )
    })
    .await
    .map_err(|error| {
        LoaderError::Other(format!(
            "Yama kaldirma worker thread'i sonlandi: {error}"
        ))
    })?;

    if let Err(ref error) = result {
        let _ = logging::event(
            "error",
            "uninstall",
            &format!(
                "Yama kaldirma basarisiz: {error}"
            ),
        );
    }

    result
}


// ============================================================
// VERIFY PATCH
// ============================================================

#[tauri::command(rename_all = "camelCase")]
async fn verify_installation(
    game_id: u64,
    game_root: String,
) -> Result<Verification, LoaderError> {
    let result = tauri::async_runtime::spawn_blocking(move || {
        patch::engine::verify_installation(
            game_id,
            &PathBuf::from(game_root),
        )
    })
    .await
    .map_err(|error| {
        LoaderError::Other(format!(
            "Dosya dogrulama worker thread'i sonlandi: {error}"
        ))
    })?;

    if let Err(ref error) = result {
        let _ = logging::event(
            "error",
            "verify",
            &format!(
                "Dosya dogrulama basarisiz: {error}"
            ),
        );
    }

    result
}


// ============================================================
// INSTALLATIONS
// ============================================================

/// Diskteki kurulum kayitlari.
/// Arayuz "kurulu mu / guncelleme var mi"
/// bilgisini artik tarayici depolamasindan degil buradan okur.
#[tauri::command]
fn list_installations(
) -> Result<Vec<InstallationSummary>, LoaderError> {
    backup::list_installations()
}


#[tauri::command(rename_all = "camelCase")]
fn installation_for_game(
    game_id: u64,
) -> Result<Option<Installation>, LoaderError> {
    backup::find_installation(game_id)
}


// ============================================================
// BACKUPS
// ============================================================

#[tauri::command]
fn list_backups(
) -> Result<Vec<BackupInfo>, LoaderError> {
    backup::list()
}


#[tauri::command(rename_all = "camelCase")]
fn clean_backup(
    backup_id: String,
) -> Result<(), LoaderError> {
    backup::clean(&backup_id)
}


/// Aktif kuruluma bagli olmayan yedekleri
/// ve indirme onbellegini temizler.
#[tauri::command]
fn prune_storage(
) -> Result<PruneReport, LoaderError> {
    backup::prune()
}


// ============================================================
// VERSION
// ============================================================

#[tauri::command]
fn loader_version(
    app: AppHandle,
) -> Result<String, LoaderError> {
    Ok(loader_version_of(&app))
}


// ============================================================
// EXTERNAL URL
// ============================================================

/// Destek/sosyal baglantilari sistem tarayicisinda acar.
/// Sadece https adreslerine izin verilir ve arguman shell'e verilmez.
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

    let target =
        parsed.as_str().to_string();

    #[cfg(target_os = "windows")]
    let spawned =
        std::process::Command::new(
            "rundll32.exe"
        )
        .args([
            "url.dll,FileProtocolHandler",
            &target,
        ])
        .spawn();

    #[cfg(target_os = "macos")]
    let spawned =
        std::process::Command::new("open")
            .arg(&target)
            .spawn();

    #[cfg(all(
        not(target_os = "windows"),
        not(target_os = "macos")
    ))]
    let spawned =
        std::process::Command::new(
            "xdg-open"
        )
        .arg(&target)
        .spawn();

    spawned.map_err(|_| {
        LoaderError::Other(
            "Bağlantı açılamadı.".into()
        )
    })?;

    Ok(())
}


// ============================================================
// LOG
// ============================================================

#[tauri::command(rename_all = "camelCase")]
fn write_client_log(
    level: String,
    category: String,
    message: String,
) -> Result<(), LoaderError> {
    let allowed = [
        "debug",
        "info",
        "warning",
        "error",
    ];

    let normalized =
        if allowed.contains(
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
}


// ============================================================
// ACCESS TOKEN
// ============================================================

#[tauri::command]
fn load_access_token(
) -> Result<Option<String>, LoaderError> {
    credential::load()
}


#[tauri::command(rename_all = "camelCase")]
fn save_access_token(
    token: String,
) -> Result<(), LoaderError> {
    credential::save(&token)
}


#[tauri::command]
fn clear_access_token(
) -> Result<(), LoaderError> {
    credential::clear()
}


// ============================================================
// APP
// ============================================================

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
                write_client_log,

                // Animus Emu
                select_ps_game_file,
                launch_ps_game,
                prepare_ps_game_root,
                launch_installed_ps_game
            ]
        )
        .run(
            tauri::generate_context!()
        )
        .expect(
            "Tauri uygulaması başlatılamadı"
        );
}
