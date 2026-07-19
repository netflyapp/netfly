# Changelog

All notable changes to Netfly are documented here.
Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [3.0.1] — 2026-07-19

### Added

- **Auto-hide address bar** (Settings → General) — inspired by Arc/Zen.
  When enabled, the top bar slides out of view after 600 ms of inactivity.
  Hover the top 4 px of the window to reveal. Content insets adjust in
  real time so the page fills the full height. Find bar (`⌘F`)
  automatically shows the bar.

## [3.0.0] — 2026-07-19

Complete UI rewrite: the vim-modal qutebrowser-style shell is replaced with an
Arc/Zen-style interface. The ultra-light Tauri 2 + system WebKit backend
carries over from v2. Frontend weighs **7.6 kB JS + 3 kB CSS (gzipped)** —
vanilla TypeScript, zero UI frameworks.

### Added

- **Collapsible left sidebar** with vertical tabs — favicon (with letter
  fallback), ellipsized title, close-on-hover, active-tab accent pill,
  new-tab row, transient status toasts at the bottom. Toggle with `⌘S`
  (200 ms animation, state persisted in `config.toml`).
- **Command palette** (`⌘L` or click the URL pill) — single input for
  addresses and search with ranked suggestions from open tabs, history,
  and bookmarks. `↩` opens in the current tab, `⌘↩` in a new tab.
  `⌘T` opens a new tab straight into the palette (Arc-style).
- **In-app settings** (`⌘,`) — General (start page, default search,
  session restore, adblock, downloads folder, config file access),
  Shortcuts, and About sections rendered as a full overlay.
- **Shortcut rebinding UI** — click a shortcut pill, press the new chord,
  done. Conflict detection with Replace/Cancel, per-action reset to
  default. Bindings persist to `config.toml` (`[bindings]`,
  action → chord or chord list) and hot-reload everywhere, including
  pages that currently hold keyboard focus.
- **Standard shortcut set** — `⌘T` new tab, `⌘W` close, `⇧⌘T` reopen,
  `⌃Tab`/`⌃⇧Tab` and `⌘⌥←`/`⌘⌥→` tab switching, `⌘1–9` jump to tab,
  `⌘L` palette, `⌘[`/`⌘]` back/forward, `⌘R`/`⇧⌘R` reload, `⌘F` find,
  `⌘D` bookmark, `⇧⌘C` copy URL, `⌘J` downloads, `⌘,` settings,
  `⌘Q` quit.
- **Find bar** (`⌘F`) docked in the top bar with next/previous match and
  `Esc` to close.
- **Downloads overlay** (`⌘J`) — list with status glyphs, open file,
  open folder, clear finished.
- **Zen dark design system** — oklch-based tokens (near-violet dark
  surfaces, muted violet accent), native macOS typography, inset traffic
  lights over the sidebar (`titleBarStyle: Overlay`), compositor-friendly
  motion with `prefers-reduced-motion` support.
- **Config additions** — `[ui] sidebar_width / sidebar_collapsed`,
  flat `[bindings]` table, programmatic config save with live
  `browser://config` propagation.

### Changed

- Shortcut delivery now works while a web page has keyboard focus: the
  content-webview init script swallows bound chords in the capture phase
  and forwards them to the shell via the `netfly://` bridge.
- Content webview layout switched from fixed top/bottom bars to dynamic
  insets (`set_content_insets`), so the native page area follows the
  sidebar in real time.
- Overlays (palette, settings, downloads) hide the native content webview
  while open (`set_overlay`) — child WKWebViews always render above the
  shell, so this guarantees overlays are never covered.
- Adblock toggle now persists to `config.toml` instead of being
  in-memory only.
- Status messages moved from a permanent bottom status bar to transient
  toasts in the sidebar footer.

### Removed

- Vim modal system (normal/insert/command/hint modes) and the `:`
  command line.
- Link-hints engine (`f`/`F`) and related `netfly://` hint IPC.
- Permanent bottom status bar (28 px) — content area reclaimed.
- `[bindings.normal]` chord → action config schema (replaced by flat
  action → chord `[bindings]`).
- `@tauri-apps/plugin-opener` JS dependency (opener is used Rust-side
  only).

### Carried over from v2

Tabs with undo-close stack, SQLite history, TOML bookmarks/quickmarks,
JSON session autosave/restore, download manager, host-based adblock with
cosmetic filtering, GreaseMonkey-style userscripts, search-engine
shortcuts (`g`, `ddg`, `w`, `gh`), macOS-native WKWebView per tab.
