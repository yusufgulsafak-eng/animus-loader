use crate::{
    error::{LoaderError, Result},
    models::Progress,
    storage::hash_file,
};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    net::IpAddr,
    time::Instant,
};
use tauri::{AppHandle, Emitter};
fn validate_public_https_url(url: &reqwest::Url) -> std::result::Result<(), String> {
    if url.scheme() != "https" { return Err("Yama indirme adresi HTTPS olmalıdır".into()); }
    let host=url.host_str().ok_or_else(|| "İndirme adresinde host yok".to_string())?;
    if host.eq_ignore_ascii_case("localhost") || host.ends_with(".localhost") { return Err("Yerel ağ indirme adresi reddedildi".into()); }
    if let Ok(ip)=host.parse::<IpAddr>() {
        let blocked=match ip { IpAddr::V4(v)=>v.is_private()||v.is_loopback()||v.is_link_local()||v.is_unspecified(), IpAddr::V6(v)=>v.is_loopback()||v.is_unspecified() };
        if blocked { return Err("Private/yerel IP indirme adresi reddedildi".into()); }
    }
    Ok(())
}

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
    let initial=reqwest::Url::parse(url).map_err(|e| LoaderError::Other(e.to_string()))?;
    if https_only { validate_public_https_url(&initial).map_err(LoaderError::Other)?; }
    let redirect_https=https_only;
    let client = reqwest::blocking::Client::builder()
        .https_only(https_only)
        .redirect(reqwest::redirect::Policy::custom(move |attempt| {
            if redirect_https && validate_public_https_url(attempt.url()).is_err() { attempt.stop() } else if attempt.previous().len() >= 8 { attempt.stop() } else { attempt.follow() }
        }))
        .timeout(std::time::Duration::from_secs(1800))
        .build()?;
    let existing=std::fs::metadata(target).map(|m|m.len()).unwrap_or(0).min(expected_size);
    let mut request=client.get(url);
    if existing>0 && existing<expected_size { request=request.header(reqwest::header::RANGE,format!("bytes={existing}-")); }
    let mut response=request.send()?.error_for_status()?;
    if https_only { validate_public_https_url(response.url()).map_err(LoaderError::Other)?; }
    let resumed=existing>0 && response.status()==reqwest::StatusCode::PARTIAL_CONTENT;
    let mut output=if resumed { let mut f=std::fs::OpenOptions::new().read(true).write(true).open(target)?;f.seek(SeekFrom::Start(existing))?;f } else { File::create(target)? };
    let mut downloaded=if resumed { existing } else { 0 };
    if !resumed {
        if let Some(length)=response.content_length() { if length!=expected_size { return Err(LoaderError::Integrity(format!("Beklenen {expected_size}, sunucu {length} bayt"))); } }
    }
    let mut buffer = [0u8; 128 * 1024];
    let started = Instant::now();
    loop {
        let count = response.read(&mut buffer)?;
        if count == 0 { break; }
        output.write_all(&buffer[..count])?;
        downloaded += count as u64;
        let percent = ((downloaded.saturating_mul(50) / expected_size.max(1)).min(50)) as u8;
        let elapsed = started.elapsed().as_secs_f64().max(0.001);
        let speed = ((downloaded.saturating_sub(if resumed {existing}else{0})) as f64 / elapsed) as u64;
        let _ = app.emit("patch-progress", Progress { stage:"download".into(), percent, message:if resumed{"Yama indirmesi devam ettiriliyor".into()}else{"Yama indiriliyor".into()}, downloaded_bytes:Some(downloaded), total_bytes:Some(expected_size), bytes_per_second:Some(speed) });
    }
    output.flush()?;output.sync_all()?;
    if downloaded != expected_size { return Err(LoaderError::Integrity("İndirilen dosya boyutu uyuşmuyor".into())); }
    let actual=hash_file(target)?;
    if !actual.eq_ignore_ascii_case(expected_hash) { let _=std::fs::remove_file(target);return Err(LoaderError::Integrity("İndirilen yama dosyasının bütünlüğü doğrulanamadı.".into())); }
    let _=crate::logging::event("info","hash","Yama SHA-256 doğrulaması başarılı");
    Ok(())
}
