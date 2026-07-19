//! Session save / load (list of tab URLs).

use crate::paths;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub name: String,
    pub active: usize,
    pub urls: Vec<String>,
}

impl Session {
    pub fn save(name: &str, active: usize, urls: Vec<String>) -> Result<(), String> {
        let session = Session {
            name: name.to_string(),
            active,
            urls,
        };
        let path = paths::session_file(name)?;
        let raw = serde_json::to_string_pretty(&session).map_err(|e| e.to_string())?;
        fs::write(path, raw).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn load(name: &str) -> Result<Session, String> {
        let path = paths::session_file(name)?;
        let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&raw).map_err(|e| e.to_string())
    }

    pub fn save_last(active: usize, urls: Vec<String>) -> Result<(), String> {
        Self::save("_last", active, urls)
    }

    pub fn load_last() -> Result<Option<Session>, String> {
        let path = paths::last_session_file()?;
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(Self::load("_last")?))
    }
}
