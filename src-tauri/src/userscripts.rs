//! Greasemonkey-style userscripts from `userscripts/*.js`.

use crate::paths;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserScript {
    pub name: String,
    pub file: String,
    pub matches: Vec<String>,
    pub excludes: Vec<String>,
    pub run_at: String,
    pub code: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Default)]
pub struct UserScriptStore {
    pub scripts: Vec<UserScript>,
}

impl UserScriptStore {
    pub fn load() -> Result<Self, String> {
        let dir = paths::userscripts_dir()?;
        ensure_example(&dir)?;

        let mut scripts = Vec::new();
        let entries = fs::read_dir(&dir).map_err(|e| e.to_string())?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("js") {
                continue;
            }
            // skip example templates that start with underscore? no — load all .js
            let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
            if let Some(script) = parse_userscript(&path, &raw) {
                scripts.push(script);
            }
        }
        scripts.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(Self { scripts })
    }

    pub fn reload(&mut self) -> Result<(), String> {
        *self = Self::load()?;
        Ok(())
    }

    /// Scripts that match URL and should run at document-end / document-idle.
    pub fn matching_for(&self, url: &str) -> Vec<&UserScript> {
        self.scripts
            .iter()
            .filter(|s| s.enabled && matches_url(s, url))
            .collect()
    }

    /// Concatenate matching scripts into one eval payload.
    pub fn inject_js_for(&self, url: &str) -> Option<String> {
        let matched = self.matching_for(url);
        if matched.is_empty() {
            return None;
        }
        let mut out = String::from("(function(){\n");
        for s in matched {
            out.push_str("try {\n");
            out.push_str(&s.code);
            out.push_str("\n} catch (e) { console.error('[netfly userscript]', ");
            out.push_str(&serde_json::to_string(&s.name).unwrap_or_else(|_| "\"?\"".into()));
            out.push_str(", e); }\n");
        }
        out.push_str("})();");
        Some(out)
    }
}

fn ensure_example(dir: &PathBuf) -> Result<(), String> {
    let example = dir.join("example-dark-hint.user.js");
    if !example.exists() {
        fs::write(example, EXAMPLE_USERSCRIPT).map_err(|e| e.to_string())?;
    }
    Ok(())
}

const EXAMPLE_USERSCRIPT: &str = r#"// ==UserScript==
// @name         Example dark hint
// @match        *://example.com/*
// @run-at       document-end
// ==/UserScript==

(function () {
  if (document.getElementById('__netfly_example_banner')) return;
  var b = document.createElement('div');
  b.id = '__netfly_example_banner';
  b.textContent = 'Netfly userscript active on example.com';
  b.style.cssText =
    'position:fixed;bottom:8px;right:8px;z-index:2147483646;' +
    'background:#161b22;color:#3fb950;font:12px Menlo,monospace;' +
    'padding:6px 10px;border-radius:6px;border:1px solid #2a313c;';
  document.documentElement.appendChild(b);
})();
"#;

fn parse_userscript(path: &std::path::Path, raw: &str) -> Option<UserScript> {
    let file = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("script.js")
        .to_string();

    let mut name = file.clone();
    let mut matches = Vec::new();
    let mut excludes = Vec::new();
    let mut run_at = "document-end".to_string();
    let mut enabled = true;

    if let Some(header) = extract_header(raw) {
        for line in header.lines() {
            let line = line.trim().trim_start_matches("//").trim();
            if let Some(v) = meta_val(line, "@name") {
                name = v;
            } else if let Some(v) = meta_val(line, "@match") {
                matches.push(v);
            } else if let Some(v) = meta_val(line, "@include") {
                matches.push(v);
            } else if let Some(v) = meta_val(line, "@exclude") {
                excludes.push(v);
            } else if let Some(v) = meta_val(line, "@run-at") {
                run_at = v;
            } else if let Some(v) = meta_val(line, "@enabled") {
                enabled = v != "false" && v != "0";
            }
        }
    }

    // no match patterns → match nothing (safer); require explicit @match
    if matches.is_empty() {
        // allow * only if user puts //@match *://*/*
        return None;
    }

    // strip header from code? keep full file — headers are comments
    Some(UserScript {
        name,
        file,
        matches,
        excludes,
        run_at,
        code: raw.to_string(),
        enabled,
    })
}

fn extract_header(raw: &str) -> Option<String> {
    let start = raw.find("==UserScript==")?;
    let end = raw.find("==/UserScript==")?;
    if end <= start {
        return None;
    }
    Some(raw[start..end].to_string())
}

fn meta_val(line: &str, key: &str) -> Option<String> {
    let line = line.trim();
    if let Some(rest) = line.strip_prefix(key) {
        let v = rest.trim();
        if !v.is_empty() {
            return Some(v.to_string());
        }
    }
    None
}

fn matches_url(script: &UserScript, url: &str) -> bool {
    for ex in &script.excludes {
        if pattern_match(ex, url) {
            return false;
        }
    }
    script.matches.iter().any(|m| pattern_match(m, url))
}

/// Simple userscript glob: `*` any chars, `?` one char. Case-sensitive.
pub fn pattern_match(pattern: &str, url: &str) -> bool {
    glob_match(pattern, url)
}

fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0usize, 0usize);
    let mut star_p: Option<usize> = None;
    let mut star_t: usize = 0;

    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star_p = Some(pi);
            star_t = ti;
            pi += 1;
        } else if let Some(sp) = star_p {
            pi = sp + 1;
            star_t += 1;
            ti = star_t;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_basics() {
        assert!(pattern_match("*://example.com/*", "https://example.com/foo"));
        assert!(pattern_match("*://*.github.com/*", "https://gist.github.com/x"));
        assert!(!pattern_match("*://example.com/*", "https://evil.com/"));
        assert!(pattern_match("*", "anything"));
    }

    #[test]
    fn parse_header() {
        let raw = r#"// ==UserScript==
// @name  Test
// @match *://a.com/*
// @exclude *://a.com/admin*
// ==/UserScript==
console.log(1);
"#;
        let path = std::path::Path::new("test.user.js");
        let s = parse_userscript(path, raw).unwrap();
        assert_eq!(s.name, "Test");
        assert_eq!(s.matches.len(), 1);
        assert!(matches_url(&s, "https://a.com/x"));
        assert!(!matches_url(&s, "https://a.com/admin"));
    }
}
