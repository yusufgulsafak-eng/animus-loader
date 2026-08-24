use crate::error::{LoaderError, Result};
use keyring::v1::Entry;

const SERVICE: &str = "com.animus.patchloader";
const ACCOUNT: &str = "api-access-token";

fn entry() -> Result<Entry> {
    Entry::new(SERVICE, ACCOUNT)
        .map_err(|_| LoaderError::Other("Windows Credential Manager açılamadı.".into()))
}

pub fn save(token: &str) -> Result<()> {
    if token.len() < 32 || token.len() > 512 {
        return Err(LoaderError::Other("Access token biçimi geçersiz.".into()));
    }
    entry()?
        .set_password(token)
        .map_err(|_| LoaderError::Other("Access token güvenli depoya kaydedilemedi.".into()))
}

pub fn load() -> Result<Option<String>> {
    match entry()?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(error) if format!("{error:?}").contains("NoEntry") => Ok(None),
        Err(_) => Err(LoaderError::Other(
            "Access token güvenli depodan okunamadı.".into(),
        )),
    }
}

pub fn clear() -> Result<()> {
    let _ = entry()?.delete_credential();
    Ok(())
}
