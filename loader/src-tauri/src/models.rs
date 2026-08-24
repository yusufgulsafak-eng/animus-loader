use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub schema_version: u16,
    pub game: Game,
    pub detection: Detection,
    pub patch: Patch,
    pub archive: Archive,
    pub install_actions: Vec<Action>,
    pub integrity: Integrity,
    pub backup: BackupRule,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub id: u64,
    pub slug: String,
    pub name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Detection {
    pub steam_app_id: Option<String>,
    pub epic_catalog_id: Option<String>,
    pub executable: String,
    pub process_name: Option<String>,
    #[serde(default)]
    pub required_files: Vec<String>,
    #[serde(default)]
    pub optional_files: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patch {
    pub id: u64,
    pub version: String,
    pub game_version: Option<String>,
    pub minimum_loader_version: String,
    pub mandatory: bool,
    pub channel: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Archive {
    pub download_token_url: String,
    pub sha256: String,
    pub size: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Integrity {
    pub verify_after_install: bool,
    pub conflict_policy: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupRule {
    pub automatic: bool,
    pub retain_until_uninstall: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: ActionType,
    pub source: Option<String>,
    pub destination: String,
    pub backup: bool,
    pub expected_sha256: Option<String>,
    #[serde(default)]
    pub options: HashMap<String, serde_json::Value>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActionType {
    CopyFile,
    CopyDirectory,
    ReplaceFile,
    DeleteFile,
    DeleteDirectory,
    CreateDirectory,
    MoveFile,
    RenameFile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRecord {
    pub kind: ChangeKind,
    pub path: String,
    pub secondary_path: Option<String>,
    pub backup_path: Option<String>,
    pub original_sha256: Option<String>,
    pub installed_sha256: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    CreatedFile,
    ReplacedFile,
    DeletedFile,
    CreatedDirectory,
    MovedFile,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Installation {
    pub schema_version: u16,
    pub game_id: u64,
    pub game_name: String,
    pub patch_id: u64,
    pub patch_version: String,
    pub game_root: String,
    pub backup_id: String,
    pub created_at: String,
    pub active: bool,
    pub changes: Vec<ChangeRecord>,
}
#[derive(Debug, Serialize)]
pub struct DryRun {
    pub created_files: u64,
    pub changed_files: u64,
    pub deleted_files: u64,
    pub backup_files: u64,
    pub download_bytes: u64,
    pub estimated_disk_bytes: u64,
    pub warnings: Vec<String>,
}
#[derive(Debug, Serialize)]
pub struct Verification {
    pub valid: bool,
    pub checked: u64,
    pub conflicts: Vec<String>,
}
#[derive(Debug, Serialize)]
pub struct BackupInfo {
    pub id: String,
    pub game_name: String,
    pub version: String,
    pub created_at: String,
    pub size_bytes: u64,
    pub active: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct Progress {
    pub stage: String,
    pub percent: u8,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub downloaded_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_per_second: Option<u64>,
}
