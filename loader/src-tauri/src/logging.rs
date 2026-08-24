use crate::{error::Result, storage::data_root};
use chrono::Utc;
use serde_json::json;
use std::{
    fs::{self, OpenOptions},
    io::Write,
};

pub fn event(level: &str, category: &str, message: &str) -> Result<()> {
    let root = data_root()?.join("logs");
    fs::create_dir_all(&root)?;
    let clean = message
        .replace("Bearer ", "Bearer [REDACTED] ")
        .chars()
        .take(1200)
        .collect::<String>();
    let line =
        json!({"time":Utc::now().to_rfc3339(),"level":level,"category":category,"message":clean});
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join("loader.log"))?;
    serde_json::to_writer(&mut file, &line)?;
    file.write_all(b"\n")?;
    Ok(())
}
