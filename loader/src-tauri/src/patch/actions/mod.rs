use crate::{
    error::Result,
    models::{Action, ChangeRecord},
};
use std::path::Path;
pub trait PatchActionHandler {
    fn supports(&self, action: &Action) -> bool;
    fn apply(
        &self,
        action: &Action,
        archive_root: &Path,
        game_root: &Path,
        backup_root: &Path,
        changes: &mut Vec<ChangeRecord>,
    ) -> Result<()>;
}
