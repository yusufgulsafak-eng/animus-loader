use animus_fat_dat_tests::{fat_dat, models::{Action, Installation}, storage::read_json};
use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 6 { return Err("Usage: fat-dat-check install|restore|verify GAME ARCHIVE BACKUP ACTION_JSON".into()); }
    let game=Path::new(&a[2]); let archive=Path::new(&a[3]); let backup=Path::new(&a[4]);
    let registration=backup.join("installation.json");
    match a[1].as_str() {
        "install" => {
            let action: Action=read_json(Path::new(&a[5]))?;
            let mut i=Installation {schema_version:1,game_id:1,game_name:"Archive QA".into(),patch_id:1,
                patch_version:"1.1.0".into(),game_root:game.display().to_string(),backup_id:"qa".into(),
                created_at:"QA".into(),active:true,changes:vec![]};
            fat_dat::install(&action,archive,game,backup,&mut i,&registration,Some("FarCry5.exe"))?;
        }
        "restore" => { let i:Installation=read_json(&registration)?; fat_dat::restore(game,backup,&i.changes[0])?; }
        "verify" => { let i:Installation=read_json(&registration)?; if !fat_dat::verify(game,backup,&i.changes[0])? { return Err("Verification failed".into()); } }
        _ => return Err("Unknown operation".into()),
    }
    println!("PASS: {}",a[1]); Ok(())
}


