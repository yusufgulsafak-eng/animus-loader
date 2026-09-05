// Compile and execute the real archive module without the Tauri UI runtime.
pub mod error {
    #[derive(Debug, thiserror::Error)]
    pub enum LoaderError {
        #[error("{0}")] Integrity(String),
        #[error("{0}")] UnsafePath(String),
        #[error("{0}")] Other(String),
        #[error("{0}")] Io(#[from] std::io::Error),
        #[error("{0}")] Json(#[from] serde_json::Error),
    }
    pub type Result<T> = std::result::Result<T,LoaderError>;
}
#[path="../../../loader/src-tauri/src/models.rs"]
pub mod models;
#[path="../../../loader/src-tauri/src/storage.rs"]
pub mod storage;
#[path="../../../loader/src-tauri/src/security/path.rs"]
pub mod path;
pub mod security { pub use crate::path; }
#[path="../../../loader/src-tauri/src/patch/fat_dat.rs"]
pub mod fat_dat;


