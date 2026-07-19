# Auto-hide Topbar (Orion-style) — Design

Date: 2026-07-19
Status: Approved

## Goal

Smooth auto-hide/auto-reveal of the topbar, inspired by Orion browser. When enabled, the
topbar stays hidden, giving the page full height. Hovering the top edge (or focusing the
URL) slides it in quickly and smoothly.

## Constraint that shapes the design

Page content lives in a **native child webview** that is layered *above* the UI shell
webview (`show_only` in `src-tauri/src/lib.rs`; the palette overlay works by hiding the
content webview entirely). The shell UI therefore cannot visually overlay the page.
A true Orion-style overlay is not achievable without native z-order hacks or a separate
child window — both rejected as high-risk/high-effort.

**Chosen approach: polished push.** Content is full-height while the topbar is hidden;
revealing the topbar resizes the content webview down by the topbar height, synchronized
with the slide animation. Smoothness comes from fast timing, easing, and a shadow —
not from overlaying.

## Behavior

### Mode toggle
- Config: `ui.auto_hide_topbar` (bool, default `false`) — Rust `UiConfig` + TS `UiConfig`.
- Settings UI checkbox toggles it; change applies live (no restart).

### Hidden state (mode on, idle)
- Topbar is `position: fixed`, translated `translateY(-100%)`, `pointer-events: none`.
- Content insets: `top = 0` (page gets full height).
- A 4px invisible trigger strip spans the top of the window (`z-index` under topbar).

### Reveal
- Trigger: `mouseenter` on the trigger strip or on the topbar itself. **No delay.**
- Animation: `translateY(-100%) → translateY(0)`, 180ms, ease-out.
- Same frame: `setContentInsets(44, left, 0, 0)` pushes the content webview down.
- Visual: shadow under the topbar, only in auto-hide mode while revealed.

### Hide
- Trigger: mouse leaves the topbar → 400ms grace period → hide.
- Animation: slide up 180ms; insets `top → 0`.
- Re-entering the topbar during the grace period cancels the hide.

### Pinning (no auto-hide while…)
- Findbar is open.
- URL pill is focused (via ⌘L / `focus-url` action). ⌘L also reveals the topbar first.
- Unpin on blur/close → normal hide flow resumes.

### Interplay
- Sidebar collapsed: topbar spans full width with traffic-light padding (existing CSS).
- Sidebar left offset: fixed topbar uses `left: sidebar width` when expanded.
- Overlay (palette/settings) open: content webview is hidden anyway; no special handling.

## Implementation notes

- Builds on the existing uncommitted diff (config plumbing, CSS skeleton, `topbar.ts`
  handlers). Fixes to that code:
  - Hide delay 600ms → 400ms; reveal becomes immediate.
  - Auto-hide listeners active only when the mode is on.
  - New: ⌘L wiring (reveal + focus pill + pin), shadow style, pin-on-url-focus.
- `TOPBAR_HEIGHT = 44` stays the single source of truth in `topbar.ts`; CSS uses the
  existing `--topbar-h` token.

## Out of scope

- Tab-switch/navigation "peek".
- Native z-order overlay experiments.
- Animating the webview resize (single-step resize, synchronized with the CSS slide).

## Testing

- Rust: config default test updated (`auto_hide_topbar = false`) — already in diff.
- Manual smoke: toggle setting live; hover reveal/hide; grace-period cancel; ⌘L reveal +
  pin; findbar pin; sidebar collapsed/expanded; mode off → topbar static as before.
