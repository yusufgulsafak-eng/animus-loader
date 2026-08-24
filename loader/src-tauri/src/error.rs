use thiserror::Error;

#[derive(Debug, Error)]
pub enum LoaderError {
    #[error("Güvenli olmayan yol: {0}")]
    UnsafePath(String),
    #[error("Manifest geçersiz: {0}")]
    Manifest(String),
    #[error("Dosya bütünlüğü uyuşmuyor: {0}")]
    Integrity(String),
    #[error("Dosya çakışması: {0}")]
    Conflict(String),
    #[error("Oyun çalışırken patch işlemi yapılamaz: {0}")]
    GameRunning(String),
    #[error("I/O hatası: {0}")]
    Io(#[from] std::io::Error),
    #[error("ZIP hatası: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("HTTP hatası: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON hatası: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Other(String),
}
pub type Result<T> = std::result::Result<T, LoaderError>;
impl serde::Serialize for LoaderError {
    fn serialize<S>(&self, s: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        s.serialize_str(&self.to_string())
    }
}
