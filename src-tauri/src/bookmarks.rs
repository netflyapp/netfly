//! Bookmarks + quickmarks persisted as TOML.

use crate::paths;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BookmarkStore {
    /// named bookmarks: name → url
    #[serde(default)]
    pub bookmarks: BTreeMap<String, String>,
    /// single-key quickmarks: key → url
    #[serde(default)]
    pub quickmarks: BTreeMap<String, String>,
}

impl BookmarkStore {
    pub fn load() -> Result<Self, String> {
        let path = paths::bookmarks_file()?;
        if !path.exists() {
            let empty = Self::default();
            empty.save()?;
            return Ok(empty);
        }
        let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        toml::from_str(&raw).map_err(|e| e.to_string())
    }

    pub fn save(&self) -> Result<(), String> {
        let path = paths::bookmarks_file()?;
        let raw = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, raw).map_err(|e| e.to_string())
    }

    pub fn set_bookmark(&mut self, name: &str, url: &str) -> Result<(), String> {
        self.bookmarks.insert(name.to_string(), url.to_string());
        self.save()
    }

    pub fn set_quickmark(&mut self, key: &str, url: &str) -> Result<(), String> {
        let k = key.chars().next().unwrap_or('?').to_string();
        self.quickmarks.insert(k, url.to_string());
        self.save()
    }

    pub fn get_bookmark(&self, name: &str) -> Option<String> {
        self.bookmarks.get(name).cloned()
    }

    pub fn get_quickmark(&self, key: &str) -> Option<String> {
        let k = key.chars().next()?.to_string();
        self.quickmarks.get(&k).cloned()
    }
}
