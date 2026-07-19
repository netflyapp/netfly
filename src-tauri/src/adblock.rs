//! Lightweight host-based ad/tracker blocking + cosmetic CSS.

use crate::paths;
use serde::Serialize;
use std::collections::HashSet;
use std::fs;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Compact built-in list of common ad/tracker hosts (suffix match).
const BUILTIN_HOSTS: &str = include_str!("../resources/blocklist-hosts.txt");

const COSMETIC_SELECTORS: &str = r#"
.adsbygoogle, .adsbygoogle-noablate,
ins.adsbygoogle,
[id^="google_ads_"], [id^="div-gpt-ad"],
[class*="ad-container"], [class*="ad_container"],
[class*="ad-banner"], [class*="ad_banner"],
[class*="advertisement"],
iframe[src*="doubleclick"], iframe[src*="googlesyndication"],
iframe[id^="google_ads"],
#ad, #ads, #banner-ad, .ad-slot, .adslot,
.sponsored-content, .promo-ad,
div[data-ad], div[data-ad-slot],
.taboola, .OUTBRAIN, .outbrain, .rc-widget
"#;

#[derive(Debug)]
pub struct Adblock {
    enabled: AtomicBool,
    hosts: parking_lot::RwLock<HashSet<String>>,
    blocked_count: AtomicU64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdblockStatus {
    pub enabled: bool,
    pub host_count: usize,
    pub blocked_count: u64,
}

impl Adblock {
    pub fn load(enabled: bool) -> Self {
        let mut hosts = HashSet::new();
        load_hosts_into(&mut hosts, BUILTIN_HOSTS);
        if let Ok(path) = paths::blocklist_file() {
            if path.exists() {
                if let Ok(raw) = fs::read_to_string(path) {
                    load_hosts_into(&mut hosts, &raw);
                }
            } else {
                // seed user-editable file with comments
                let _ = fs::write(
                    &path,
                    "# Netfly extra blocklist (one host per line)\n\
                     # Matching is suffix-based: tracker.com blocks a.tracker.com\n\
                     # Lines starting with # are ignored\n",
                );
            }
        }
        Self {
            enabled: AtomicBool::new(enabled),
            hosts: parking_lot::RwLock::new(hosts),
            blocked_count: AtomicU64::new(0),
        }
    }

    pub fn reload(&self) -> Result<usize, String> {
        let mut hosts = HashSet::new();
        load_hosts_into(&mut hosts, BUILTIN_HOSTS);
        if let Ok(path) = paths::blocklist_file() {
            if path.exists() {
                let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
                load_hosts_into(&mut hosts, &raw);
            }
        }
        let n = hosts.len();
        *self.hosts.write() = hosts;
        Ok(n)
    }

    pub fn set_enabled(&self, on: bool) {
        self.enabled.store(on, Ordering::Relaxed);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn status(&self) -> AdblockStatus {
        AdblockStatus {
            enabled: self.is_enabled(),
            host_count: self.hosts.read().len(),
            blocked_count: self.blocked_count.load(Ordering::Relaxed),
        }
    }

    /// True if URL's host should be blocked.
    pub fn is_blocked_url(&self, url: &str) -> bool {
        if !self.is_enabled() {
            return false;
        }
        let Ok(u) = url::Url::parse(url) else {
            return false;
        };
        let Some(host) = u.host_str() else {
            return false;
        };
        self.is_blocked_host(host)
    }

    pub fn is_blocked_host(&self, host: &str) -> bool {
        if !self.is_enabled() {
            return false;
        }
        let host = host.trim_end_matches('.').to_ascii_lowercase();
        if host.is_empty() {
            return false;
        }
        let set = self.hosts.read();
        // exact or suffix match: ads.foo.com matches foo.com entry? we store full hosts
        // entry "doubleclick.net" matches "ad.doubleclick.net"
        if set.contains(&host) {
            return true;
        }
        for part in host.split('.').collect::<Vec<_>>().windows(2).rev() {
            // rebuild from each label
            let _ = part;
        }
        // check every suffix
        let labels: Vec<&str> = host.split('.').collect();
        for i in 0..labels.len() {
            let suffix = labels[i..].join(".");
            if set.contains(&suffix) {
                return true;
            }
        }
        false
    }

    pub fn record_block(&self) {
        self.blocked_count.fetch_add(1, Ordering::Relaxed);
    }

    /// JS/CSS inject for cosmetic filtering (runs on each page load).
    pub fn cosmetic_inject_js(&self) -> Option<String> {
        if !self.is_enabled() {
            return None;
        }
        let selectors = COSMETIC_SELECTORS
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join(",");
        let sel_json = serde_json::to_string(&selectors).ok()?;
        Some(format!(
            r#"(function(){{
  try {{
    var sel = {sel_json};
    var style = document.getElementById('__netfly_adblock_css');
    if (!style) {{
      style = document.createElement('style');
      style.id = '__netfly_adblock_css';
      style.textContent = sel + '{{display:none!important;visibility:hidden!important;height:0!important;max-height:0!important;overflow:hidden!important;}}';
      (document.documentElement||document.head).appendChild(style);
    }}
    function nuke() {{
      try {{
        document.querySelectorAll(sel).forEach(function(el){{
          el.remove();
        }});
      }} catch(e) {{}}
    }}
    nuke();
    if (!window.__netflyAdblockObs) {{
      window.__netflyAdblockObs = new MutationObserver(function(){{ nuke(); }});
      window.__netflyAdblockObs.observe(document.documentElement, {{childList:true,subtree:true}});
    }}
  }} catch(e) {{}}
}})();"#
        ))
    }
}

fn load_hosts_into(set: &mut HashSet<String>, raw: &str) {
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // support "0.0.0.0 host" / "127.0.0.1 host"
        let host = if line.starts_with("0.0.0.0 ") || line.starts_with("127.0.0.1 ") {
            line.split_whitespace().nth(1).unwrap_or("")
        } else {
            line.split_whitespace().next().unwrap_or("")
        };
        let host = host.trim().trim_start_matches('.').to_ascii_lowercase();
        if host.is_empty() || host == "localhost" {
            continue;
        }
        set.insert(host);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suffix_block() {
        let ab = Adblock::load(true);
        assert!(ab.is_blocked_host("doubleclick.net") || ab.is_blocked_host("ad.doubleclick.net"));
        // if doubleclick in list
        if ab.hosts.read().contains("doubleclick.net") {
            assert!(ab.is_blocked_host("ad.doubleclick.net"));
            assert!(ab.is_blocked_host("doubleclick.net"));
        }
        assert!(!ab.is_blocked_host("example.com"));
    }

    #[test]
    fn disabled_allows() {
        let ab = Adblock::load(false);
        assert!(!ab.is_blocked_host("doubleclick.net"));
    }
}
