//! Browser state: tabs, navigation helpers, URL normalization.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabInfo {
    pub id: String,
    pub url: String,
    pub title: String,
    pub loading: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserSnapshot {
    pub tabs: Vec<TabInfo>,
    pub active_tab: usize,
    pub status: String,
}

#[derive(Debug)]
pub struct BrowserState {
    pub tabs: Vec<TabInfo>,
    pub active_tab: usize,
    pub status: String,
    pub closed: VecDeque<TabInfo>,
    next_id: u64,
}

impl Default for BrowserState {
    fn default() -> Self {
        let mut s = Self::empty();
        s.push_tab("about:blank", "New Tab");
        s
    }
}

impl BrowserState {
    /// Empty browser with no tabs (caller must add at least one).
    pub fn empty() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab: 0,
            status: String::new(),
            closed: VecDeque::new(),
            next_id: 0,
        }
    }
}

impl BrowserState {
    pub fn snapshot(&self) -> BrowserSnapshot {
        BrowserSnapshot {
            tabs: self.tabs.clone(),
            active_tab: self.active_tab,
            status: self.status.clone(),
        }
    }

    pub fn active_mut(&mut self) -> &mut TabInfo {
        &mut self.tabs[self.active_tab]
    }

    pub fn active(&self) -> &TabInfo {
        &self.tabs[self.active_tab]
    }

    pub fn active_id(&self) -> String {
        self.active().id.clone()
    }

    pub fn alloc_id(&mut self) -> String {
        let id = format!("content-{}", self.next_id);
        self.next_id += 1;
        id
    }

    pub fn push_tab(&mut self, url: &str, title: &str) -> String {
        let id = self.alloc_id();
        self.tabs.push(TabInfo {
            id: id.clone(),
            url: url.into(),
            title: title.into(),
            loading: false,
        });
        self.active_tab = self.tabs.len() - 1;
        id
    }

    /// Close active tab. Returns (closed_id, optional new_active_id).
    /// If last tab, replaces with blank rather than empty.
    pub fn close_active(&mut self) -> (String, String) {
        let closed = self.tabs.remove(self.active_tab);
        let closed_id = closed.id.clone();
        self.closed.push_front(closed);
        while self.closed.len() > 20 {
            self.closed.pop_back();
        }

        if self.tabs.is_empty() {
            let id = self.push_tab("about:blank", "New Tab");
            return (closed_id, id);
        }

        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
        let active_id = self.active_id();
        (closed_id, active_id)
    }

    /// Restore last closed tab. Returns restored tab id if any.
    pub fn undo_close(&mut self) -> Option<TabInfo> {
        let tab = self.closed.pop_front()?;
        // re-allocate a fresh webview id (old webview was destroyed)
        let mut restored = tab;
        restored.id = self.alloc_id();
        self.tabs.push(restored.clone());
        self.active_tab = self.tabs.len() - 1;
        Some(restored)
    }

    pub fn switch_tab(&mut self, index: usize) -> Option<String> {
        if index >= self.tabs.len() {
            return None;
        }
        self.active_tab = index;
        Some(self.active_id())
    }

    pub fn next_tab(&mut self) -> String {
        if self.tabs.is_empty() {
            return self.push_tab("about:blank", "New Tab");
        }
        self.active_tab = (self.active_tab + 1) % self.tabs.len();
        self.active_id()
    }

    pub fn prev_tab(&mut self) -> String {
        if self.tabs.is_empty() {
            return self.push_tab("about:blank", "New Tab");
        }
        if self.active_tab == 0 {
            self.active_tab = self.tabs.len() - 1;
        } else {
            self.active_tab -= 1;
        }
        self.active_id()
    }

}

pub fn urlencoding_lite(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_lifecycle() {
        let mut b = BrowserState::default();
        assert_eq!(b.tabs.len(), 1);
        let id2 = b.push_tab("https://a.com", "A");
        assert_eq!(b.tabs.len(), 2);
        assert_eq!(b.active().id, id2);
        b.prev_tab();
        let (closed, _) = b.close_active();
        assert!(closed.starts_with("content-"));
        assert_eq!(b.tabs.len(), 1);
        let restored = b.undo_close().unwrap();
        assert_eq!(b.tabs.len(), 2);
        assert_ne!(restored.id, closed); // new webview id
    }

    #[test]
    fn urlencoding_space() {
        assert_eq!(urlencoding_lite("a b"), "a%20b");
    }
}
