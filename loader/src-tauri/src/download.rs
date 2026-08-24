use crate::{
    error::{LoaderError, Result},
    models::Progress,
    storage::hash_file,
};
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
    time::Instant,
};
use tauri::{AppHandle, Emitter};
pub fn download(
    app: &AppHandle,
    url: &str,
    target: &Path,
    expected_size: u64,
    expected_hash: &str,
) -> Result<()> {
    #[cfg(debug_assertions)]
    let https_only = !url.starts_with("http://127.0.0.1");
    #[cfg(not(debug_assertions))]
    let https_only = true;
    let client = reqwest::blocking::Client::builder()
        .https_only(https_only)
        .timeout(std::time::Duration::from_secs(1800))
        .build()?;
    let mut response = client.get(url).send()?.error_for_status()?;
    if let Some(length) = response.content_length() {
        if length != expected_size {
            return Err(LoaderError::Integrity(format!(
                "Beklenen {expected_size}, sunucu {length} bayt"
            )));
        }
    }
    let mut output = File::create(target)?;
    let mut downloaded = 0u64;
    let mut buffer = [0u8; 128 * 1024];
    let started = Instant::now();
    loop {
        let count = response.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        output.write_all(&buffer[..count])?;
        downloaded += count as u64;
        let percent = ((downloaded.saturating_mul(50) / expected_size.max(1)).min(50)) as u8;
        let elapsed = started.elapsed().as_secs_f64().max(0.001);
        let speed = (downloaded as f64 / elapsed) as u64;
        let _ = app.emit(
            "patch-progress",
            Progress {
                stage: "download".into(),
                percent,
                message: "Yama indiriliyor".into(),
                downloaded_bytes: Some(downloaded),
                total_bytes: Some(expected_size),
                bytes_per_second: Some(speed),
            },
        );
    }
    output.flush()?;
    output.sync_all()?;
    if downloaded != expected_size {
        let _ = crate::logging::event(
            "error",
            "download",
            &format!("Boyut uyuşmazlığı: expected={expected_size}, actual={downloaded}"),
        );
        return Err(LoaderError::Integrity(
            "İndirilen dosya boyutu uyuşmuyor".into(),
        ));
    }
    let actual = hash_file(target)?;
    if !actual.eq_ignore_ascii_case(expected_hash) {
        let _ = crate::logging::event(
            "error",
            "hash",
            &format!("SHA-256 uyuşmazlığı: expected={expected_hash}, actual={actual}"),
        );
        return Err(LoaderError::Integrity(
            "İndirilen yama dosyasının bütünlüğü doğrulanamadı.".into(),
        ));
    }
    let _ = crate::logging::event("info", "hash", "Yama SHA-256 doğrulaması başarılı");
    Ok(())
}
