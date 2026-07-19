//! Download tracking + path helpers.

use crate::config::Config;
use crate::paths;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DownloadStatus {
    Requested,
    Finished,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadItem {
    pub id: u64,
    pub url: String,
    pub filename: String,
    pub path: String,
    pub status: DownloadStatus,
    pub success: Option<bool>,
    pub started_at: i64,
    pub finished_at: Option<i64>,
}

#[derive(Debug, Default)]
pub struct DownloadManager {
    next_id: u64,
    items: Vec<DownloadItem>,
}

impl DownloadManager {
    pub fn list(&self) -> Vec<DownloadItem> {
        self.items.clone()
    }

    pub fn clear_finished(&mut self) {
        self.items
            .retain(|i| matches!(i.status, DownloadStatus::Requested));
    }

    pub fn on_requested(&mut self, url: &str, dest: &Path) -> DownloadItem {
        let id = self.next_id;
        self.next_id += 1;
        let filename = dest
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("download")
            .to_string();
        let item = DownloadItem {
            id,
            url: url.to_string(),
            filename,
            path: dest.display().to_string(),
            status: DownloadStatus::Requested,
            success: None,
            started_at: now_secs(),
            finished_at: None,
        };
        self.items.insert(0, item.clone());
        // cap history
        if self.items.len() > 100 {
            self.items.truncate(100);
        }
        item
    }

    pub fn on_finished(&mut self, url: &str, path: Option<&Path>, success: bool) -> Option<DownloadItem> {
        // match most recent requested with same url
        if let Some(item) = self
            .items
            .iter_mut()
            .find(|i| i.url == url && matches!(i.status, DownloadStatus::Requested))
        {
            item.status = if success {
                DownloadStatus::Finished
            } else {
                DownloadStatus::Failed
            };
            item.success = Some(success);
            item.finished_at = Some(now_secs());
            if let Some(p) = path {
                item.path = p.display().to_string();
                if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                    item.filename = name.to_string();
                }
            }
            return Some(item.clone());
        }
        // create synthetic finished entry
        let id = self.next_id;
        self.next_id += 1;
        let path_s = path
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let filename = path
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .unwrap_or("download")
            .to_string();
        let item = DownloadItem {
            id,
            url: url.to_string(),
            filename,
            path: path_s,
            status: if success {
                DownloadStatus::Finished
            } else {
                DownloadStatus::Failed
            },
            success: Some(success),
            started_at: now_secs(),
            finished_at: Some(now_secs()),
        };
        self.items.insert(0, item.clone());
        Some(item)
    }

    pub fn persist(&self) -> Result<(), String> {
        let path = paths::downloads_log()?;
        let raw = serde_json::to_string_pretty(&self.items).map_err(|e| e.to_string())?;
        fs::write(path, raw).map_err(|e| e.to_string())
    }

    pub fn load() -> Self {
        let mut mgr = Self::default();
        if let Ok(path) = paths::downloads_log() {
            if let Ok(raw) = fs::read_to_string(path) {
                if let Ok(items) = serde_json::from_str::<Vec<DownloadItem>>(&raw) {
                    mgr.next_id = items.iter().map(|i| i.id).max().unwrap_or(0) + 1;
                    mgr.items = items;
                }
            }
        }
        mgr
    }
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Build absolute destination path under config download_dir.
pub fn destination_for(cfg: &Config, url: &str, suggested: Option<&str>) -> PathBuf {
    let dir = PathBuf::from(cfg.expanded_download_dir());
    let _ = fs::create_dir_all(&dir);

    let mut name = suggested
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| filename_from_url(url));

    // sanitize
    name = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ' ') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if name.is_empty() {
        name = "download".into();
    }

    let mut path = dir.join(&name);
    if !path.exists() {
        return path;
    }
    // unique suffix
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("download")
        .to_string();
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|e| format!(".{e}"))
        .unwrap_or_default();
    for n in 1..1000 {
        let candidate = dir.join(format!("{stem} ({n}){ext}"));
        if !candidate.exists() {
            path = candidate;
            break;
        }
    }
    path
}

fn filename_from_url(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| {
            u.path_segments()
                .and_then(|mut s| s.next_back())
                .map(|s| s.to_string())
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "download".into())
}

/// Fetch URL to disk (manual :download command).
pub fn fetch_to_file(url: &str, dest: &Path) -> Result<u64, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(120))
        .build();
    let resp = agent.get(url).call().map_err(|e| e.to_string())?;
    let mut reader = resp.into_reader();
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut file = fs::File::create(dest).map_err(|e| e.to_string())?;
    let n = std::io::copy(&mut reader, &mut file).map_err(|e| e.to_string())?;
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filename_from_path() {
        assert_eq!(
            filename_from_url("https://example.com/files/a.pdf?x=1"),
            "a.pdf"
        );
    }

    #[test]
    fn manager_lifecycle() {
        let mut m = DownloadManager::default();
        let p = PathBuf::from("/tmp/netfly-test-dl.bin");
        let item = m.on_requested("https://x/y.bin", &p);
        assert_eq!(item.status, DownloadStatus::Requested);
        let done = m.on_finished("https://x/y.bin", Some(&p), true).unwrap();
        assert_eq!(done.status, DownloadStatus::Finished);
    }
}
