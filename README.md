# Netfly

An ultra-light macOS browser with an Arc/Zen-style interface, built on
Tauri 2 and the system WebKit engine. The entire frontend weighs
**7.6 kB JS + 3 kB CSS (gzipped)** — vanilla TypeScript, zero UI frameworks.

## Features

- **Collapsible sidebar with vertical tabs** — favicons, active-tab accent,
  close-on-hover, status toasts. Toggle with `⌘S`.
- **Command palette** (`⌘L`) — one input for URLs and search, with ranked
  suggestions from open tabs, history, and bookmarks. `↩` opens in the
  current tab, `⌘↩` in a new one. `⌘T` opens a new tab straight into the
  palette, Arc-style.
- **Auto-hide address bar** (Settings → General) — Orion-inspired. The top
  bar stays hidden and the page gets the full window height; hover the top
  edge (or press `⌘L`) to slide it in, move away and it hides again after
  400 ms. Respects `prefers-reduced-motion`.
- **Ad & tracker blocking** — host-based blocklist with cosmetic CSS
  filtering, toggleable in settings.
- **Find in page** (`⌘F`), **bookmarks** (`⌘D`), **downloads overlay**
  (`⌘J`), **session restore**, **search-engine prefixes**.
- **Fully rebindable shortcuts** — click a shortcut pill in Settings, press
  the new chord, done. Conflict detection included; bindings persist to
  `config.toml` and hot-reload everywhere, even while a page holds focus.
- **Zen dark design system** — oklch-based tokens, native macOS typography,
  inset traffic lights, compositor-friendly motion.

## Keyboard shortcuts (defaults)

| Action | Shortcut |
|---|---|
| New tab (opens palette) | `⌘T` |
| Close / reopen tab | `⌘W` / `⇧⌘T` |
| Switch tabs | `⌃Tab` / `⌃⇧Tab`, `⌘⌥←` / `⌘⌥→` |
| Jump to tab 1–9 | `⌘1`–`⌘9` |
| Command palette | `⌘L` |
| Back / forward | `⌘[` / `⌘]` |
| Reload / hard reload | `⌘R` / `⇧⌘R` |
| Find in page | `⌘F` |
| Bookmark page | `⌘D` |
| Copy page URL | `⇧⌘C` |
| Toggle sidebar | `⌘S` |
| Downloads | `⌘J` |
| Settings | `⌘,` |
| Quit | `⌘Q` |

All of these can be rebound in Settings → Shortcuts.

## Search prefixes

Type a prefix followed by a query in the palette:

| Prefix | Engine |
|---|---|
| `g` | Google |
| `ddg` | DuckDuckGo |
| `w` | Wikipedia (EN) |
| `gh` | GitHub |

No prefix falls through to the default search engine (DuckDuckGo out of the
box). Add your own engines under `[search_engines]` in the config.

## Configuration

Config lives at `~/Library/Application Support/netfly/config.toml` and is
managed from the in-app settings (`⌘,`); manual edits are picked up via the
settings panel's "Reload config".

```toml
start_page = "about:blank"
download_dir = "~/Downloads"
default_search = "https://duckduckgo.com/?q={}"
restore_session = true
adblock = true

[ui]
sidebar_width = 240
sidebar_collapsed = false
auto_hide_topbar = false

[search_engines]
g = "https://www.google.com/search?q={}"

[bindings]
# tab-new = "cmd+t"
# tab-next = ["ctrl+tab", "cmd+alt+right"]
```

## Development

Requirements: macOS, Node.js, Rust (stable), and the Tauri 2 CLI
(installed as a dev dependency).

```bash
npm install
npm run start      # tauri dev — Vite dev server + debug build
npm run build      # type-check + bundle the frontend
npm run app:build  # production .app bundle
```

Rust tests:

```bash
cd src-tauri && cargo test
```

## Architecture

- `src-tauri/` — Rust backend: window and tab management (one native
  WebKit webview per tab), config, history, bookmarks, downloads, adblock,
  session persistence.
- `src/` — the shell UI (topbar, sidebar, palette, overlays) in vanilla
  TypeScript. Tab content renders in native child webviews layered above
  the shell; the backend repositions them via content insets as the UI
  changes.
- Design docs and implementation plans live in `docs/superpowers/`.

## License

Private project — no license granted.
