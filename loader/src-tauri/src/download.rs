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

fn extract_href_from_area(
    area: &str,
) -> Option<String> {
    for prefix in ["href=\"", "href='"] {
        let Some(start) = area.find(prefix) else {
            continue;
        };

        let quote = if prefix.ends_with('"') {
            '"'
        } else {
            '\''
        };

        let value_start = start + prefix.len();
        let tail = &area[value_start..];

        let Some(end) = tail.find(quote) else {
            continue;
        };

        let value =
            decode_html_attribute(&tail[..end]);

        if value.starts_with("https://") {
            return Some(value);
        }
    }

    None
}

/// MediaFire /file/.../file sayfasındaki gerçek CDN indirme linkini bulur.
/// Sayfanın kendisi 100-400 KB HTML olduğu için bunu yapmadan arşiv diye
/// indirmek, ekrandaki "beklenen 3.3 GB / indirilen 332 KB" hatasına yol açar.
fn extract_mediafire_direct_url(
    html: &str,
) -> Option<String> {
    // MediaFire'ın klasik downloadButton öğesi.
    for marker in [
        "id=\"downloadButton\"",
        "id='downloadButton'",
    ] {
        if let Some(marker_pos) = html.find(marker) {
            let start =
                marker_pos.saturating_sub(8192);
            let end =
                (marker_pos + 8192).min(html.len());

            let area = &html[start..end];

            // href çoğu zaman id'den önce olduğu için son href'i tercih et.
            let before_len =
                marker_pos.saturating_sub(start);
            let before = &area[..before_len];

            for prefix in ["href=\"", "href='"] {
                if let Some(pos) =
                    before.rfind(prefix)
                {
                    let quote =
                        if prefix.ends_with('"') {
                            '"'
                        } else {
                            '\''
                        };

                    let value_start =
                        pos + prefix.len();
                    let tail =
                        &before[value_start..];

                    if let Some(value_end) =
                        tail.find(quote)
                    {
                        let value =
                            decode_html_attribute(
                                &tail[..value_end],
                            );

                        if value.starts_with(
                            "https://",
                        ) {
                            return Some(value);
                        }
                    }
                }
            }

            if let Some(value) =
                extract_href_from_area(area)
            {
                return Some(value);
            }
        }
    }

    // MediaFire zaman zaman downloadButton markup'ını değiştiriyor.
    // Doğrudan CDN hostunu son çare olarak HTML içinde ara.
    for marker in [
        "https://download",
        "https:\\/\\/download",
    ] {
        if let Some(start) = html.find(marker) {
            let tail = &html[start..];
            let mut end = tail.len();

            for separator in [
                '"', '\'', '<', ' ', '\\',
            ] {
                if let Some(pos) =
                    tail.find(separator)
                {
                    if pos > 8 {
                        end = end.min(pos);
                    }
                }
            }

            let mut value =
                tail[..end].replace("\\/", "/");

            value =
                decode_html_attribute(&value);

            if let Ok(parsed) =
                reqwest::Url::parse(&value)
            {
                let host = parsed
                    .host_str()
                    .unwrap_or("")
                    .to_ascii_lowercase();

                if host.ends_with(
                    ".mediafire.com",
                ) {
                    return Some(value);
                }
            }
        }
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

pub fn download(
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

    let mut output = if resumed {
        let mut file =
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(target)?;

        file.seek(
            SeekFrom::Start(existing),
        )?;

        file
    } else {
        File::create(target)?
    };

    let mut downloaded =
        if resumed { existing } else { 0 };

    if !resumed {
        if let Some(length) =
            response.content_length()
        {
            if length != expected_size {
                let _ =
                    std::fs::remove_file(
                        target,
                    );

                return Err(
                    LoaderError::Integrity(
                        format!(
                            "İndirilen dosya boyutu uyuşmuyor. Beklenen: {expected_size} bayt, sunucu: {length} bayt"
                        ),
                    ),
                );
            }
        }
    }

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
        let _ =
            std::fs::remove_file(target);

        return Err(LoaderError::Integrity(
            format!(
                "İndirilen dosya boyutu uyuşmuyor. Beklenen: {expected_size} bayt, indirilen: {downloaded} bayt"
            ),
        ));
    }

    let actual = hash_file(target)?;

    if !actual.eq_ignore_ascii_case(
        expected_hash,
    ) {
        let _ =
            std::fs::remove_file(target);

        return Err(
            LoaderError::Integrity(
                "İndirilen yama dosyasının bütünlüğü doğrulanamadı."
                    .into(),
            ),
        );
    }

    let _ = crate::logging::event(
        "info",
        "hash",
        "Yama SHA-256 doğrulaması başarılı",
    );

    Ok(())
}
