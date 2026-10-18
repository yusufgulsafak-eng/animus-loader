use crate::{
    error::{LoaderError, Result},
    models::Progress,
    storage::hash_file,
};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    net::IpAddr,
    path::Path,
    time::Instant,
};
use tauri::{AppHandle, Emitter};

const DOWNLOAD_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
     (KHTML, like Gecko) Chrome/140.0 Safari/537.36 AnimusSync/1.0";

fn validate_public_https_url(
    url: &reqwest::Url,
) -> std::result::Result<(), String> {
    if url.scheme() != "https" {
        return Err(
            "Yama indirme adresi HTTPS olmalıdır".into(),
        );
    }

    let host = url
        .host_str()
        .ok_or_else(|| {
            "İndirme adresinde host yok".to_string()
        })?;

    if host.eq_ignore_ascii_case("localhost")
        || host.ends_with(".localhost")
    {
        return Err(
            "Yerel ağ indirme adresi reddedildi".into(),
        );
    }

    if let Ok(ip) = host.parse::<IpAddr>() {
        let blocked = match ip {
            IpAddr::V4(v) => {
                v.is_private()
                    || v.is_loopback()
                    || v.is_link_local()
                    || v.is_unspecified()
            }
            IpAddr::V6(v) => {
                v.is_loopback() || v.is_unspecified()
            }
        };

        if blocked {
            return Err(
                "Private/yerel IP indirme adresi reddedildi"
                    .into(),
            );
        }
    }

    Ok(())
}

fn is_mediafire_landing_url(
    url: &reqwest::Url,
) -> bool {
    let host = url
        .host_str()
        .unwrap_or("")
        .to_ascii_lowercase();

    let mediafire_host =
        host == "mediafire.com"
            || host == "www.mediafire.com";

    mediafire_host
        && url.path().contains("/file/")
        && url.path().ends_with("/file")
}

fn decode_html_attribute(
    value: &str,
) -> String {
    value
        .replace("&amp;", "&")
        .replace("&#38;", "&")
        .replace("&#x26;", "&")
        .replace("&#x2F;", "/")
        .replace("&#47;", "/")
}

// MediaFire file pages must resolve to the real ZIP CDN, not an HTML
// landing page, advertisement, or an unrelated HTTPS link.
fn mediafire_zip_url(value: &str) -> Option<String> {
    let decoded = decode_html_attribute(value).replace("\\/", "/");
    let url = reqwest::Url::parse(&decoded).ok()?;
    let host = url.host_str()?.to_ascii_lowercase();
    let subdomain = host.split('.').next().unwrap_or("");

    if url.scheme() != "https"
        || !host.ends_with(".mediafire.com")
        || !subdomain.starts_with("download")
        || !url.path().to_ascii_lowercase().ends_with(".zip")
    {
        return None;
    }

    Some(url.to_string())
}

/// Resolve MediaFire's download button first, then look for a validated
/// CDN URL in JavaScript as a fallback. Do not use a nearby anchor's href.
fn extract_mediafire_direct_url(html: &str) -> Option<String> {
    let mut remaining = html;
    while let Some(start) = remaining.find("<a") {
        let tail = &remaining[start..];
        let Some(end) = tail.find('>') else { break };
        let tag = &tail[..end];

        if tag.contains("id=\"downloadButton\"")
            || tag.contains("id='downloadButton'")
        {
            for marker in ["href=\"", "href='"] {
                if let Some(pos) = tag.find(marker) {
                    let value = &tag[pos + marker.len()..];
                    let quote = if marker.ends_with('"') { '"' } else { '\'' };
                    if let Some(end) = value.find(quote) {
                        if let Some(url) = mediafire_zip_url(&value[..end]) {
                            return Some(url);
                        }
                    }
                }
            }
        }
        remaining = &tail[end + 1..];
    }

    // MediaFire may escape / in an embedded JSON/JS URL.
    let normalized = html.replace("\\/", "/");
    let mut remaining = normalized.as_str();
    while let Some(start) = remaining.find("https://download") {
        let tail = &remaining[start..];
        let end = tail.find(|c: char| {
            c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | '\\' | ')' | ',')
        }).unwrap_or(tail.len());

        if let Some(url) = mediafire_zip_url(&tail[..end]) {
            return Some(url);
        }
        remaining = &tail["https://download".len()..];
    }

    None
}

fn resolve_external_download_url(
    client: &reqwest::blocking::Client,
    url: &reqwest::Url,
    https_only: bool,
) -> Result<reqwest::Url> {
    if !is_mediafire_landing_url(url) {
        return Ok(url.clone());
    }

    let response = client
        .get(url.clone())
        .header(
            reqwest::header::ACCEPT,
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .send()?
        .error_for_status()?;

    if https_only {
        validate_public_https_url(
            response.url(),
        )
        .map_err(LoaderError::Other)?;
    }

    let html = response.text()?;

    let direct =
        extract_mediafire_direct_url(&html)
            .ok_or_else(|| {
                LoaderError::Other(
                    "MediaFire doğrudan indirme bağlantısı çözülemedi. MediaFire sayfa yapısı değişmiş olabilir."
                        .into(),
                )
            })?;

    let parsed =
        reqwest::Url::parse(&direct).map_err(
            |error| {
                LoaderError::Other(format!(
                    "MediaFire indirme bağlantısı geçersiz: {error}"
                ))
            },
        )?;

    if https_only {
        validate_public_https_url(&parsed)
            .map_err(LoaderError::Other)?;
    }

    Ok(parsed)
}

/// Indirme onbellegi SHA-256 ile anahtarlanir: ayni icerik ayni dosyaya yazilir,
/// bu yuzden yarim kalan indirme guvenle devam ettirilebilir. Farkli bir yama
/// asla ayni dosyaya denk gelmez.
pub fn cache_path(
    sha256: &str,
) -> Result<std::path::PathBuf> {
    let normalized =
        sha256.to_ascii_lowercase();

    if normalized.len() != 64
        || !normalized
            .chars()
            .all(|c| c.is_ascii_hexdigit())
    {
        return Err(LoaderError::Integrity(
            "Archive SHA-256 bicimi gecersiz"
                .into(),
        ));
    }

    let dir =
        crate::storage::data_root()?
            .join("cache");

    std::fs::create_dir_all(&dir)?;

    Ok(dir.join(format!(
        "{normalized}.zip"
    )))
}

/// Tamamlanmis kurulumdan sonra veya bakim sirasinda onbellegi bosaltir.
fn partial_cache_looks_like_zip(
    target: &Path,
) -> bool {
    let Ok(mut file) = File::open(target) else {
        return false;
    };

    let mut signature = [0u8; 4];

    if file.read_exact(&mut signature).is_err() {
        return false;
    }

    matches!(
        signature,
        [b'P', b'K', 3, 4]
            | [b'P', b'K', 5, 6]
            | [b'P', b'K', 7, 8]
    )
}

pub fn clear_cache() -> Result<u64> {
    let dir =
        crate::storage::data_root()?
            .join("cache");

    if !dir.is_dir() {
        return Ok(0);
    }

    let mut freed = 0u64;

    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();

        if !path.is_file() {
            continue;
        }

        let size =
            std::fs::metadata(&path)
                .map(|m| m.len())
                .unwrap_or(0);

        if std::fs::remove_file(&path)
            .is_ok()
        {
            freed += size;
        }
    }

    Ok(freed)
}

const DOWNLOAD_MAX_ATTEMPTS: usize = 5;

fn is_retryable_download_error(
    error: &LoaderError,
) -> bool {
    match error {
        LoaderError::Http(error) => {
            if let Some(status) = error.status() {
                return status.is_server_error()
                    || status
                        == reqwest::StatusCode::TOO_MANY_REQUESTS
                    || status
                        == reqwest::StatusCode::REQUEST_TIMEOUT;
            }

            error.is_timeout()
                || error.is_connect()
                || error.is_body()
                || error.is_request()
        }
        LoaderError::Io(error) => {
            let message = error
                .to_string()
                .to_ascii_lowercase();

            matches!(
                error.kind(),
                std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::BrokenPipe
                    | std::io::ErrorKind::TimedOut
                    | std::io::ErrorKind::UnexpectedEof
                    | std::io::ErrorKind::WouldBlock
                    | std::io::ErrorKind::Interrupted
            ) || message.contains(
                "request or response body",
            ) || message.contains(
                "connection closed",
            ) || message.contains(
                "connection reset",
            ) || message.contains(
                "unexpected eof",
            )
        }
        LoaderError::Other(message) => {
            let message =
                message.to_ascii_lowercase();

            message.contains(
                "beklenenden erken kapandı",
            ) || message.contains(
                "geçici indirme hatası",
            )
        }
        _ => false,
    }
}

fn retry_delay_seconds(
    completed_attempt: usize,
) -> u64 {
    match completed_attempt {
        0 | 1 => 2,
        2 => 4,
        3 => 8,
        _ => 12,
    }
}

pub fn download(
    app: &AppHandle,
    url: &str,
    target: &Path,
    expected_size: u64,
    expected_hash: &str,
) -> Result<()> {
    for attempt in 1..=DOWNLOAD_MAX_ATTEMPTS {
        match download_once(
            app,
            url,
            target,
            expected_size,
            expected_hash,
        ) {
            Ok(()) => return Ok(()),
            Err(error) => {
                if !is_retryable_download_error(
                    &error,
                ) {
                    return Err(error);
                }

                if attempt >= DOWNLOAD_MAX_ATTEMPTS {
                    return Err(
                        LoaderError::Other(
                            format!(
                                "Yama indirmesi {DOWNLOAD_MAX_ATTEMPTS} denemeden sonra tamamlanamadı. Bağlantı veya MediaFire CDN aktarımı kesiliyor. Son hata: {error}"
                            ),
                        ),
                    );
                }

                let downloaded =
                    std::fs::metadata(target)
                        .map(|metadata| {
                            metadata
                                .len()
                                .min(expected_size)
                        })
                        .unwrap_or(0);

                let percent = (
                    downloaded
                        .saturating_mul(50)
                        / expected_size.max(1)
                )
                    .min(50)
                    as u8;

                let next_attempt = attempt + 1;

                let _ = crate::logging::event(
                    "warn",
                    "download",
                    &format!(
                        "İndirme bağlantısı kesildi; otomatik yeniden deneme {next_attempt}/{DOWNLOAD_MAX_ATTEMPTS}: {error}"
                    ),
                );

                let _ = app.emit(
                    "patch-progress",
                    Progress {
                        stage: "download".into(),
                        percent,
                        message: format!(
                            "İndirme bağlantısı yenileniyor ({next_attempt}/{DOWNLOAD_MAX_ATTEMPTS})"
                        ),
                        downloaded_bytes:
                            Some(downloaded),
                        total_bytes:
                            Some(expected_size),
                        bytes_per_second: None,
                    },
                );

                std::thread::sleep(
                    std::time::Duration::from_secs(
                        retry_delay_seconds(
                            attempt,
                        ),
                    ),
                );
            }
        }
    }

    Err(LoaderError::Other(
        "İndirme yeniden deneme sınırı aşıldı."
            .into(),
    ))
}

fn download_once(
    app: &AppHandle,
    url: &str,
    target: &Path,
    expected_size: u64,
    expected_hash: &str,
) -> Result<()> {
    #[cfg(debug_assertions)]
    let https_only =
        !url.starts_with(
            "http://127.0.0.1",
        );

    #[cfg(not(debug_assertions))]
    let https_only = true;

    let initial =
        reqwest::Url::parse(url).map_err(
            |error| {
                LoaderError::Other(
                    error.to_string(),
                )
            },
        )?;

    if https_only {
        validate_public_https_url(&initial)
            .map_err(LoaderError::Other)?;
    }

    let redirect_https = https_only;

    let client =
        reqwest::blocking::Client::builder()
            .https_only(https_only)
            .user_agent(DOWNLOAD_USER_AGENT)
            .redirect(
                reqwest::redirect::Policy::custom(
                    move |attempt| {
                        if redirect_https
                            && validate_public_https_url(
                                attempt.url(),
                            )
                            .is_err()
                        {
                            attempt.stop()
                        } else if attempt
                            .previous()
                            .len()
                            >= 8
                        {
                            attempt.stop()
                        } else {
                            attempt.follow()
                        }
                    },
                ),
            )
            .timeout(
                std::time::Duration::from_secs(
                    1800,
                ),
            )
            .build()?;

    // MediaFire /file/.../file URL'leri dosya değildir, HTML indirme sayfasıdır.
    // Gerçek CDN URL'si her indirmede dinamik çözümlenir.
    let resolved_url =
        resolve_external_download_url(
            &client,
            &initial,
            https_only,
        )?;

    // Onbellekte tam dosya varsa yeniden indirme;
    // hash tutuyorsa dogrudan kullan.
    if let Ok(metadata) =
        std::fs::metadata(target)
    {
        if metadata.len() == expected_size {
            if hash_file(target)?
                .eq_ignore_ascii_case(
                    expected_hash,
                )
            {
                let _ = app.emit(
                    "patch-progress",
                    Progress {
                        stage:
                            "download".into(),
                        percent: 50,
                        message:
                            "Yama onbellekten kullaniliyor"
                                .into(),
                        downloaded_bytes:
                            Some(expected_size),
                        total_bytes:
                            Some(expected_size),
                        bytes_per_second: None,
                    },
                );

                return Ok(());
            }

            let _ =
                std::fs::remove_file(
                    target,
                );
        } else if metadata.len()
            > expected_size
        {
            let _ =
                std::fs::remove_file(
                    target,
                );
        }
    }

    let mut existing =
        std::fs::metadata(target)
            .map(|m| m.len())
            .unwrap_or(0)
            .min(expected_size);

    // Eski sürüm MediaFire HTML sayfasını yarım ZIP sanıp cache'e bırakmışsa
    // Range ile devam etmeye çalışma; baştan gerçek arşivi indir.
    if existing > 0
        && existing < expected_size
        && !partial_cache_looks_like_zip(target)
    {
        let _ =
            std::fs::remove_file(target);
        existing = 0;
    }

    let mut request =
        client.get(resolved_url.clone());

    if existing > 0
        && existing < expected_size
    {
        request = request.header(
            reqwest::header::RANGE,
            format!("bytes={existing}-"),
        );
    }

    let mut response =
        request.send()?.error_for_status()?;

    if https_only {
        validate_public_https_url(
            response.url(),
        )
        .map_err(LoaderError::Other)?;
    }

    // HTML geliyorsa artık bunu ZIP diye diske yazma.
    // Bu, MediaFire landing page veya koruma sayfasını anında yakalar.
    if let Some(content_type) =
        response.headers()
            .get(
                reqwest::header::CONTENT_TYPE,
            )
            .and_then(
                |value| value.to_str().ok(),
            )
    {
        if content_type
            .to_ascii_lowercase()
            .contains("text/html")
        {
            let _ =
                std::fs::remove_file(
                    target,
                );

            return Err(
                LoaderError::Integrity(
                    "İndirme sunucusu arşiv yerine HTML sayfası döndürdü. MediaFire doğrudan dosya bağlantısı alınamadı."
                        .into(),
                ),
            );
        }
    }

    let resumed =
        existing > 0
            && response.status()
                == reqwest::StatusCode::PARTIAL_CONTENT;

    // A resumable CDN response must start at precisely our cached byte.
    // Otherwise appending would corrupt the archive and fail SHA-256.
    if resumed {
        let range = response.headers()
            .get(reqwest::header::CONTENT_RANGE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("");
        let valid_range = range.strip_prefix("bytes ")
            .and_then(|s| s.split_once('/'))
            .and_then(|(range, total)| {
                let (start, _) = range.split_once('-')?;
                Some((start.parse::<u64>().ok()?, total.parse::<u64>().ok()?))
            })
            .map(|(start, total)| start == existing && total == expected_size)
            .unwrap_or(false);
        if !valid_range {
            let _ = std::fs::remove_file(target);
            return Err(LoaderError::Other(
                "Geçici indirme hatası: CDN Content-Range uyuşmuyor, indirme baştan denenecek.".into()
            ));
        }
    }

    // Reject known-wrong content lengths before opening the cache file.
    if !resumed {
        if let Some(length) = response.content_length() {
            if length != expected_size {
                let _ = std::fs::remove_file(target);
                return Err(LoaderError::Integrity(format!(
                    "İndirilen dosya boyutu uyuşmuyor. Beklenen: {expected_size} bayt, sunucu: {length} bayt"
                )));
            }
        }
    }

    // Some HTML pages are labelled application/octet-stream. Validate the
    // ZIP signature before writing the response to the cache.
    let mut zip_header = [0u8; 4];
    if !resumed {
        response.read_exact(&mut zip_header)?;
        if !matches!(
            zip_header,
            [b'P', b'K', 3, 4] | [b'P', b'K', 5, 6] | [b'P', b'K', 7, 8]
        ) {
            let _ = std::fs::remove_file(target);
            return Err(LoaderError::Integrity(
                "MediaFire ZIP yerine HTML veya ZIP olmayan içerik döndürdü.".into()
            ));
        }
    }

    let mut output = if resumed {
        let mut file = std::fs::OpenOptions::new()
            .read(true).write(true).open(target)?;
        file.seek(SeekFrom::Start(existing))?;
        file
    } else {
        File::create(target)?
    };

    if !resumed {
        output.write_all(&zip_header)?;
    }
    let mut downloaded = if resumed { existing } else { 4 };

    let mut buffer =
        [0u8; 128 * 1024];

    let started = Instant::now();

    loop {
        let count =
            response.read(&mut buffer)?;

        if count == 0 {
            break;
        }

        output.write_all(
            &buffer[..count],
        )?;

        downloaded += count as u64;

        let percent = (
            (
                downloaded
                    .saturating_mul(50)
                    / expected_size.max(1)
            )
            .min(50)
        ) as u8;

        let elapsed =
            started
                .elapsed()
                .as_secs_f64()
                .max(0.001);

        let speed = (
            (
                downloaded.saturating_sub(
                    if resumed {
                        existing
                    } else {
                        0
                    },
                )
            ) as f64
                / elapsed
        ) as u64;

        let _ = app.emit(
            "patch-progress",
            Progress {
                stage: "download".into(),
                percent,
                message: if resumed {
                    "Yama indirmesi devam ettiriliyor"
                        .into()
                } else {
                    "Yama indiriliyor".into()
                },
                downloaded_bytes:
                    Some(downloaded),
                total_bytes:
                    Some(expected_size),
                bytes_per_second:
                    Some(speed),
            },
        );
    }

    output.flush()?;
    output.sync_all()?;

    if downloaded != expected_size {
        if downloaded > expected_size {
            let _ =
                std::fs::remove_file(target);

            return Err(
                LoaderError::Integrity(
                    format!(
                        "İndirilen dosya boyutu uyuşmuyor. Beklenen: {expected_size} bayt, indirilen: {downloaded} bayt"
                    ),
                ),
            );
        }

        // Sunucu temiz EOF döndürerek bağlantıyı erken kapatırsa kısmi ZIP'i
        // silme. Üstteki retry katmanı yeni bir MediaFire CDN URL'si çözüp
        // Range isteğiyle kaldığı bayttan devam eder.
        return Err(LoaderError::Other(
            format!(
                "İndirme bağlantısı beklenenden erken kapandı. Beklenen: {expected_size} bayt, indirilen: {downloaded} bayt"
            ),
        ));
    }

    let actual = hash_file(target)?;

    if !actual.eq_ignore_ascii_case(
        expected_hash,
    ) {
        // Keep verification strict, but report enough information to
        // distinguish a stale EXE, CDN mismatch, and a wrong manifest.
        // Never log a signed CDN query string or token.
        let cdn_host = response
            .url()
            .host_str()
            .unwrap_or("(bilinmiyor)");
        let _ = crate::logging::event(
            "error",
            "download_integrity",
            &format!(
                "SHA256 uyuşmazlığı: beklenen={expected_hash}; gelen={actual}; boyut={downloaded}; cdn_host={cdn_host}"
            ),
        );
        let _ = std::fs::remove_file(target);

        return Err(LoaderError::Integrity(
            format!(
                "İndirilen yama dosyasının bütünlüğü doğrulanamadı. Beklenen SHA256: {expected_hash}; indirilen SHA256: {actual}; boyut: {downloaded} bayt. Ayrıntılar: Loader günlüğü."
            ),
        ));
    }

    let _ = crate::logging::event(
        "info",
        "hash",
        "Yama SHA-256 doğrulaması başarılı",
    );

    Ok(())
}

#[cfg(test)]
mod mediafire_resolution_tests {
    use super::*;
    const CDN: &str =
        "https://download1501.mediafire.com/abcd1234/ANIMUS_AC_SHADOWS.zip";

    #[test]
    fn selects_only_download_button_href() {
        let html = format!(
            "<a href='https://ads.example.net/fake.zip'>Ad</a>\
             <a href='{CDN}' id='downloadButton'>Download</a>"
        );
        assert_eq!(extract_mediafire_direct_url(&html), Some(CDN.into()));
    }

    #[test]
    fn rejects_page_and_unrelated_host() {
        assert!(extract_mediafire_direct_url(
            "<a id='downloadButton' href='https://www.mediafire.com/file/foo/file'>x</a>"
        ).is_none());
        assert!(extract_mediafire_direct_url(
            "<a id='downloadButton' href='https://download.bad.example.org/file.zip'>x</a>"
        ).is_none());
    }

    #[test]
    fn accepts_escaped_js_cdn_url() {
        let escaped = CDN.replace("/", "\\/");
        assert_eq!(extract_mediafire_direct_url(
            &format!("var link='{escaped}';")), Some(CDN.into()));
    }

    #[test]
    fn rejects_non_zip_cdn_url() {
        assert!(mediafire_zip_url(
            "https://download1501.mediafire.com/abc/payload.exe"
        ).is_none());
    }
}
