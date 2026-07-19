//! User config (`config.toml`).

use crate::paths;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;

pub const DEFAULT_CONFIG_TOML: &str = r#"# Netfly configuration
# Path: ~/Library/Application Support/netfly/config.toml
# Hot-reloaded when edited via the in-app settings; use the settings
# panel's "Reload config" after manual edits.

start_page = "about:blank"
download_dir = "~/Downloads"
default_search = "https://duckduckgo.com/?q={}"
# Restore last session on startup (if non-empty)
restore_session = true
# Host-based ad/tracker block + cosmetic CSS
adblock = true

[ui]
sidebar_width = 240
sidebar_collapsed = false

[search_engines]
g = "https://www.google.com/search?q={}"
ddg = "https://duckduckgo.com/?q={}"
w = "https://en.wikipedia.org/wiki/Special:Search?search={}"
gh = "https://github.com/search?q={}"

# Shortcut overrides (action = chord or list of chords).
# Unlisted actions keep built-in defaults. Rebind from Settings (Cmd+,)
# or edit here. Chord format: "cmd+shift+t", "ctrl+tab", "cmd+[".
#
# Available actions:
#   tab-new tab-close tab-reopen tab-next tab-prev tab-1 ... tab-9
#   palette sidebar-toggle reload hard-reload back forward
#   find bookmark settings downloads copy-url quit
#
# [bindings]
# tab-new = "cmd+t"
# tab-next = ["ctrl+tab", "cmd+alt+right"]
"#;

/// One chord or several for a single action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ChordSet {
    One(String),
    Many(Vec<String>),
}

impl ChordSet {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn chords(&self) -> Vec<String> {
        match self {
            ChordSet::One(c) => vec![c.clone()],
            ChordSet::Many(v) => v.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct UiConfig {
    pub sidebar_width: u32,
    pub sidebar_collapsed: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            sidebar_width: 240,
            sidebar_collapsed: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub start_page: String,
    pub download_dir: String,
    pub default_search: String,
    pub restore_session: bool,
    pub adblock: bool,
    pub ui: UiConfig,
    pub search_engines: BTreeMap<String, String>,
    pub bindings: BTreeMap<String, ChordSet>,
}

impl Default for Config {
    fn default() -> Self {
        let mut search_engines = BTreeMap::new();
        search_engines.insert("g".into(), "https://www.google.com/search?q={}".into());
        search_engines.insert("ddg".into(), "https://duckduckgo.com/?q={}".into());
        search_engines.insert(
            "w".into(),
            "https://en.wikipedia.org/wiki/Special:Search?search={}".into(),
        );
        search_engines.insert("gh".into(), "https://github.com/search?q={}".into());

        Self {
            start_page: "about:blank".into(),
            download_dir: "~/Downloads".into(),
            default_search: "https://duckduckgo.com/?q={}".into(),
            restore_session: true,
            adblock: true,
            ui: UiConfig::default(),
            search_engines,
            bindings: BTreeMap::new(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let path = paths::config_file()?;
        if !path.exists() {
            fs::write(&path, DEFAULT_CONFIG_TOML).map_err(|e| e.to_string())?;
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        toml::from_str(&raw).map_err(|e| format!("config.toml parse error: {e}"))
    }

    pub fn reload(&mut self) -> Result<(), String> {
        *self = Self::load()?;
        Ok(())
    }

    /// Persist current config to config.toml (comments are not preserved).
    pub fn save(&self) -> Result<(), String> {
        let path = paths::config_file()?;
        let toml = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, toml).map_err(|e| e.to_string())
    }

    /// Override an action's chords. An empty list is a valid override
    /// meaning "explicitly unbound"; use `reset_binding` to restore defaults.
    pub fn set_binding(&mut self, action: &str, mut chords: Vec<String>) {
        let set = if chords.len() == 1 {
            ChordSet::One(chords.remove(0))
        } else {
            ChordSet::Many(chords)
        };
        self.bindings.insert(action.into(), set);
    }

    /// Remove an action's override so built-in defaults apply again.
    pub fn reset_binding(&mut self, action: &str) {
        self.bindings.remove(action);
    }

    pub fn set_ui(&mut self, sidebar_width: u32, sidebar_collapsed: bool) {
        self.ui = UiConfig {
            sidebar_width,
            sidebar_collapsed,
        };
    }

    pub fn path() -> Result<String, String> {
        paths::config_file().map(|p| p.display().to_string())
    }

    /// Expand `~` in download_dir.
    pub fn expanded_download_dir(&self) -> String {
        if let Some(rest) = self.download_dir.strip_prefix("~/") {
            if let Some(home) = dirs::home_dir() {
                return home.join(rest).display().to_string();
            }
        }
        if self.download_dir == "~" {
            if let Some(home) = dirs::home_dir() {
                return home.display().to_string();
            }
        }
        self.download_dir.clone()
    }
}

/// Normalize user input using config search engines / default search.
pub fn normalize_url(input: &str, cfg: &Config) -> String {
    let raw = input.trim();
    if raw.is_empty() {
        return cfg.start_page.clone();
    }
    if raw == "about:blank" {
        return raw.into();
    }
    if raw.starts_with("http://") || raw.starts_with("https://") || raw.starts_with("file://") {
        return raw.into();
    }

    // engine shortcut: "g rust" or "gh foobar"
    if let Some((key, rest)) = raw.split_once(' ') {
        let key = key.trim();
        let rest = rest.trim();
        if !rest.is_empty() {
            if let Some(template) = cfg.search_engines.get(key) {
                return apply_search_template(template, rest);
            }
        }
    }

    let looks_like_url = raw.contains('.')
        && !raw.contains(' ')
        && !raw.starts_with('.')
        && raw.chars().all(|c| {
            c.is_ascii_alphanumeric()
                || matches!(c, '.' | '/' | ':' | '-' | '_' | '?' | '=' | '&' | '%')
        });

    if looks_like_url {
        format!("https://{raw}")
    } else {
        apply_search_template(&cfg.default_search, raw)
    }
}

fn apply_search_template(template: &str, query: &str) -> String {
    let encoded = crate::browser::urlencoding_lite(query);
    if template.contains("{}") {
        template.replace("{}", &encoded)
    } else {
        format!("{template}{encoded}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_engine_shortcut() {
        let cfg = Config::default();
        let u = normalize_url("g hello world", &cfg);
        assert!(u.contains("google.com"));
        assert!(u.contains("hello%20world"));
    }

    #[test]
    fn default_search() {
        let cfg = Config::default();
        let u = normalize_url("hello world", &cfg);
        assert!(u.contains("duckduckgo.com"));
    }

    #[test]
    fn domain() {
        let cfg = Config::default();
        assert_eq!(normalize_url("example.com", &cfg), "https://example.com");
    }

    #[test]
    fn parse_default_toml() {
        let cfg: Config = toml::from_str(DEFAULT_CONFIG_TOML).unwrap();
        assert!(cfg.restore_session);
        assert!(cfg.search_engines.contains_key("g"));
        assert_eq!(cfg.ui.sidebar_width, 240);
        assert!(!cfg.ui.sidebar_collapsed);
    }

    #[test]
    fn bindings_one_or_many_roundtrip() {
        let raw = r#"
            [bindings]
            tab-new = "cmd+t"
            tab-next = ["ctrl+tab", "cmd+alt+right"]
        "#;
        let cfg: Config = toml::from_str(raw).unwrap();
        assert_eq!(cfg.bindings["tab-new"].chords(), vec!["cmd+t"]);
        assert_eq!(
            cfg.bindings["tab-next"].chords(),
            vec!["ctrl+tab", "cmd+alt+right"]
        );
        let out = toml::to_string_pretty(&cfg).unwrap();
        let re: Config = toml::from_str(&out).unwrap();
        assert_eq!(re.bindings, cfg.bindings);
    }

    #[test]
    fn set_binding_add_unbind_reset() {
        let mut cfg = Config::default();
        cfg.set_binding("tab-new", vec!["cmd+shift+n".into()]);
        assert_eq!(cfg.bindings["tab-new"].chords(), vec!["cmd+shift+n"]);
        cfg.set_binding("tab-new", vec![]);
        assert_eq!(cfg.bindings["tab-new"].chords(), Vec::<String>::new());
        cfg.reset_binding("tab-new");
        assert!(!cfg.bindings.contains_key("tab-new"));
    }
}
