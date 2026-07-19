//! Application data paths under Application Support.

use std::fs;
use std::path::PathBuf;

pub fn data_dir() -> Result<PathBuf, String> {
    let base = dirs::data_dir().ok_or_else(|| "cannot resolve data dir".to_string())?;
    let dir = base.join("netfly");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(dir.join("sessions")).map_err(|e| e.to_string())?;
    fs::create_dir_all(dir.join("userscripts")).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub fn history_db() -> Result<PathBuf, String> {
    Ok(data_dir()?.join("history.sqlite"))
}

pub fn bookmarks_file() -> Result<PathBuf, String> {
    Ok(data_dir()?.join("bookmarks.toml"))
}

pub fn session_file(name: &str) -> Result<PathBuf, String> {
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    Ok(data_dir()?.join("sessions").join(format!("{safe}.json")))
}

pub fn last_session_file() -> Result<PathBuf, String> {
    session_file("_last")
}

pub fn config_file() -> Result<PathBuf, String> {
    Ok(data_dir()?.join("config.toml"))
}

pub fn userscripts_dir() -> Result<PathBuf, String> {
    let dir = data_dir()?.join("userscripts");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub fn downloads_log() -> Result<PathBuf, String> {
    Ok(data_dir()?.join("downloads.json"))
}

pub fn blocklist_file() -> Result<PathBuf, String> {
    Ok(data_dir()?.join("blocklist-hosts.txt"))
}
