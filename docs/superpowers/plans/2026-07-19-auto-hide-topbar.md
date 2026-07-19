# Auto-hide Topbar (Orion-style) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Smooth auto-hide/auto-reveal topbar: hidden by default when enabled, revealed instantly on top-edge hover or ⌘L, hidden again 400ms after the mouse leaves.

**Architecture:** "Polished push" — page content is a native child webview layered above the shell UI, so the topbar cannot overlay it. Content is full-height while the topbar is hidden; revealing slides the topbar in (180ms ease-out CSS transform) and resizes the content webview down 44px in the same frame via `setContentInsets`. All inset math is centralized in `syncInsets()` (src/ui/sidebar.ts).

**Tech Stack:** Tauri v2 (Rust backend, multi-webview), vanilla TypeScript + Vite frontend, CSS custom properties.

**Spec:** `docs/superpowers/specs/2026-07-19-auto-hide-topbar-design.md`

## Global Constraints

- Config key: `ui.auto_hide_topbar`, default `false` (already plumbed Rust↔TS in the uncommitted diff — this plan builds on that diff, do not revert it).
- Reveal animation: 180ms, ease `var(--ease)`; must respect `prefers-reduced-motion` (duration 0ms).
- Hide grace period: 400ms (`HIDE_DELAY_MS` in `src/ui/topbar.ts`).
- Topbar height source of truth: `TOPBAR_HEIGHT = 44` in `src/ui/sidebar.ts`; CSS token `--topbar-h`.
- Pinned (never auto-hides) while: findbar open, or any overlay open (`ui.overlay !== 'none'`).
- ⌘L (`palette` action) reveals the topbar before opening the palette.
- No new dependencies; no JS test framework exists in this repo — verification is `npm run build` (tsc + vite), `cargo test` in `src-tauri`, and the manual smoke checklist in Task 5.
- Backend emits `browser://config` after `config_set_ui` (src-tauri/src/lib.rs:535); the `onConfig` handler in `src/main.ts` is the single place that reapplies auto-hide state after config changes.

---

### Task 1: CSS polish — timing token, shadow, transition

**Files:**
- Modify: `src/styles/tokens.css`
- Modify: `src/styles/topbar.css:111-150`

**Interfaces:**
- Produces: CSS token `--t-topbar` (reveal duration); class contract used by Task 2: `#app.auto-hide-topbar`, `#topbar.auto-hide`, `#topbar.topbar-hidden`, `.topbar-trigger.auto-hide`.

- [ ] **Step 1: Add reveal-duration token with reduced-motion override**

In `src/styles/tokens.css`, the `/* motion */` block currently reads:

```css
  /* motion */
  --t-fast: 120ms;
  --t-med: 200ms;
  --ease: cubic-bezier(0.2, 0, 0, 1);
```

Change to:

```css
  /* motion */
  --t-fast: 120ms;
  --t-med: 200ms;
  --t-topbar: 180ms;
  --ease: cubic-bezier(0.2, 0, 0, 1);
```

And extend the existing reduced-motion block:

```css
@media (prefers-reduced-motion: reduce) {
  :root {
    --t-fast: 0ms;
    --t-med: 0ms;
    --t-topbar: 0ms;
  }
}
```

- [ ] **Step 2: Use the token and add a reveal shadow**

In `src/styles/topbar.css`, replace the whole auto-hide section (from `/* Auto-hide mode: … */` to end of file) with:

```css
/* Auto-hide mode: collapse grid row & fix topbar over the top edge */
#app.auto-hide-topbar {
  --topbar-h: 0px;
}

#topbar.auto-hide {
  position: fixed;
  z-index: 100;
  top: 0;
  left: var(--sidebar-current, var(--sidebar-w));
  right: 0;
  min-width: 0;
  transform: translateY(0);
  transition: transform var(--t-topbar) var(--ease);
  box-shadow: 0 8px 24px oklch(0 0 0 / 0.35);
}

#app.sidebar-collapsed #topbar.auto-hide {
  left: 0;
  padding-left: var(--traffic-lights-w);
}

#topbar.auto-hide.topbar-hidden {
  transform: translateY(-100%);
  pointer-events: none;
  box-shadow: none;
}

/* Thin invisible trigger strip at the top of the window */
.topbar-trigger {
  display: none;
  position: fixed;
  z-index: 99;
  top: 0;
  left: 0;
  right: 0;
  height: 4px;
}

.topbar-trigger.auto-hide {
  display: block;
}
```

- [ ] **Step 3: Verify build passes**

Run: `npm run build`
Expected: exits 0 (tsc + vite succeed).

- [ ] **Step 4: Commit**

```bash
git add src/styles/tokens.css src/styles/topbar.css
git commit -m "feat(topbar): auto-hide reveal timing token and shadow"
```

---

### Task 2: topbar.ts rework — guards, centralized insets, exported hide scheduler

**Files:**
- Modify: `src/ui/topbar.ts`

**Interfaces:**
- Consumes: `syncInsets(): Promise<void>` from `src/ui/sidebar.ts` (exists); `getState`/`setUi` from `src/state.ts`; CSS classes from Task 1.
- Produces (used by Tasks 3–4): `showTopbar(): void` (reveal + cancel pending hide; no-op when mode off), `scheduleTopbarHide(): void` (hide after 400ms unless pinned; no-op when mode off), `applyAutoHide(): void` (apply classes; on mode *transition* reset visibility).

Current bugs being fixed: hide scheduled even when mode off; 600ms delay; duplicated inset math (`ipc.setContentInsets` computed locally); `applyAutoHide` force-hides on *every* config event (any settings change would slam the topbar); redundant `mouseover` listener.

- [ ] **Step 1: Replace the auto-hide block in `src/ui/topbar.ts`**

Replace lines 1–12 (imports + module vars) with:

```ts
import { runAction } from '../actions'
import { activeTab, getState, setUi, subscribe } from '../state'
import { el } from '../util/dom'
import { displayUrl } from '../util/favicon'
import { mountFindbar } from './findbar'
import { syncInsets } from './sidebar'

const HIDE_DELAY_MS = 400

let urlPill: HTMLElement
let backBtn: HTMLButtonElement
let fwdBtn: HTMLButtonElement
let hideTimer: ReturnType<typeof setTimeout> | null = null
let autoHideWasOn: boolean | null = null
```

(Note: the `ipc` import and `TOPBAR_HEIGHT` const are removed — inset math now lives only in `syncInsets`.)

- [ ] **Step 2: Replace functions `setTopbarVisible` through `updateAutoHide` (currently lines 24–85) with:**

```ts
function setTopbarVisible(visible: boolean): void {
  const rootEl = document.getElementById('topbar') as HTMLElement
  rootEl.classList.toggle('topbar-hidden', !visible)
  setUi({ topbarVisible: visible })
  void syncInsets()
}

function cancelHide(): void {
  if (hideTimer) {
    clearTimeout(hideTimer)
    hideTimer = null
  }
}

/** Hide after a grace period unless something pins the topbar open. */
export function scheduleTopbarHide(): void {
  if (!getState().config.ui.auto_hide_topbar) return
  if (hideTimer) return
  hideTimer = setTimeout(() => {
    hideTimer = null
    const { config, ui } = getState()
    if (!config.ui.auto_hide_topbar) return
    if (ui.findOpen || ui.overlay !== 'none') return
    setTopbarVisible(false)
  }, HIDE_DELAY_MS)
}

function wireAutoHide(rootEl: HTMLElement): void {
  const strip = el('div', { class: 'topbar-trigger' })
  document.body.append(strip)

  strip.addEventListener('mouseenter', showTopbar)
  rootEl.addEventListener('mouseenter', showTopbar)
  rootEl.addEventListener('mouseleave', scheduleTopbarHide)
}
```

- [ ] **Step 3: Replace the exported `applyAutoHide` and `showTopbar` (currently lines 126–138) with:**

```ts
/** Apply auto-hide classes; reset visibility only when the mode toggles. */
export function applyAutoHide(): void {
  const on = getState().config.ui.auto_hide_topbar
  const app = document.getElementById('app') as HTMLElement
  const rootEl = document.getElementById('topbar') as HTMLElement
  const strip = document.querySelector('.topbar-trigger') as HTMLElement | null

  app.classList.toggle('auto-hide-topbar', on)
  rootEl.classList.toggle('auto-hide', on)
  strip?.classList.toggle('auto-hide', on)

  if (on === autoHideWasOn) return
  autoHideWasOn = on
  cancelHide()
  setTopbarVisible(!on)
}

/** Reveal now and cancel any pending hide (findbar, palette, hover). */
export function showTopbar(): void {
  if (!getState().config.ui.auto_hide_topbar) return
  cancelHide()
  setTopbarVisible(true)
}
```

`mountTopbar` keeps calling `wireAutoHide(rootEl)` before `mountFindbar(rootEl)` — unchanged.

- [ ] **Step 4: Verify build passes**

Run: `npm run build`
Expected: exits 0. If tsc reports an unused-import error for `ipc`, the old import was left behind — remove it.

- [ ] **Step 5: Commit**

```bash
git add src/ui/topbar.ts
git commit -m "feat(topbar): smooth auto-hide with guards and centralized insets"
```

---

### Task 3: sidebar.ts — deduplicate inset math in toggleSidebar

**Files:**
- Modify: `src/ui/sidebar.ts:46-53`

**Interfaces:**
- Consumes: nothing new.
- Produces: `toggleSidebar` now routes through `syncInsets()`; `syncInsets` remains the single inset authority (`topInset = auto_hide on && topbar hidden ? 0 : 44`).

- [ ] **Step 1: Replace `toggleSidebar` with:**

```ts
/** Toggle collapse: CSS transition on shell + native re-layout + persist. */
export async function toggleSidebar(): Promise<void> {
  const { config, ui } = getState()
  const collapsed = !ui.sidebarCollapsed
  applySidebarCollapsed(collapsed)
  await syncInsets()
  await ipc.configSetUi(SIDEBAR_WIDTH, collapsed, config.ui.auto_hide_topbar)
}
```

(`applySidebarCollapsed` updates `ui.sidebarCollapsed` via `setUi` before `syncInsets` reads it, so the computed left inset is correct.)

- [ ] **Step 2: Verify build passes**

Run: `npm run build`
Expected: exits 0.

- [ ] **Step 3: Commit**

```bash
git add src/ui/sidebar.ts
git commit -m "refactor(sidebar): route toggleSidebar insets through syncInsets"
```

---

### Task 4: ⌘L reveal + overlay-close re-hide + drop redundant settings call

**Files:**
- Modify: `src/main.ts:40` (palette runner)
- Modify: `src/ui/overlay.ts:32-42` (`hideOverlay`)
- Modify: `src/ui/settings.ts` (remove direct `applyAutoHide` call)

**Interfaces:**
- Consumes: `showTopbar()`, `scheduleTopbarHide()` from Task 2.
- Produces: nothing new.

- [ ] **Step 1: Reveal topbar when the palette opens**

In `src/main.ts`, change the import (line 21) to:

```ts
import { applyAutoHide, mountTopbar, showTopbar } from './ui/topbar'
```

and the palette runner (line 40) to:

```ts
    palette: () => {
      showTopbar()
      void openPalette()
    },
```

- [ ] **Step 2: Schedule a hide when any overlay closes**

In `src/ui/overlay.ts`, add the import:

```ts
import { scheduleTopbarHide } from './topbar'
```

and append one line at the end of `hideOverlay`:

```ts
export async function hideOverlay(): Promise<void> {
  if (getState().ui.overlay === 'none') return
  const container = root()
  if (cleanup) cleanup()
  cleanup = undefined
  container.removeEventListener('mousedown', onBackdropClick)
  container.hidden = true
  container.replaceChildren()
  setUi({ overlay: 'none' })
  await ipc.setOverlay(false)
  scheduleTopbarHide()
}
```

(No import cycle: `overlay.ts → topbar.ts → sidebar.ts`; sidebar does not import overlay.)

- [ ] **Step 3: Remove redundant `applyAutoHide` from settings**

In `src/ui/settings.ts`, the auto-hide field currently reads:

```ts
    field(
      'Auto-hide address bar',
      toggle(config.ui.auto_hide_topbar, async (v) => {
        await ipc.configSetUi(config.ui.sidebar_width, config.ui.sidebar_collapsed, v)
        applyAutoHide()
      }),
      'Hide the top bar when not in use. Hover the top edge to reveal.',
    ),
```

Change to (the backend emits `browser://config`, and `onConfig` in main.ts calls `applyAutoHide`):

```ts
    field(
      'Auto-hide address bar',
      toggle(config.ui.auto_hide_topbar, (v) =>
        void ipc.configSetUi(config.ui.sidebar_width, config.ui.sidebar_collapsed, v),
      ),
      'Hide the top bar when not in use. Hover the top edge to reveal.',
    ),
```

Also remove the now-unused import line `import { applyAutoHide } from './topbar'` at the top of `settings.ts`.

- [ ] **Step 4: Verify build passes**

Run: `npm run build`
Expected: exits 0 (tsc will catch any leftover unused import).

- [ ] **Step 5: Commit**

```bash
git add src/main.ts src/ui/overlay.ts src/ui/settings.ts
git commit -m "feat(topbar): reveal on palette open, re-hide after overlay close"
```

---

### Task 5: Backend tests, smoke test, final commit

**Files:**
- Test: `src-tauri/src/config.rs` (test already updated in the pre-existing diff)
- Modify: `CHANGELOG.md` (entry already present in the pre-existing diff — verify it describes auto-hide)

**Interfaces:**
- Consumes: everything above.

- [ ] **Step 1: Run Rust tests**

Run: `cd src-tauri && cargo test`
Expected: all tests pass, including `config` defaults test asserting `!cfg.ui.auto_hide_topbar`.

- [ ] **Step 2: Manual smoke test**

Run: `npm run start` (tauri dev), then verify:

1. Settings → toggle "Auto-hide address bar" ON → topbar slides up, page grows to full height, no restart needed.
2. Move mouse to top edge (4px strip) → topbar slides down in ~180ms, page content pushes down 44px simultaneously; shadow visible under topbar.
3. Move mouse away → topbar stays ~400ms, then slides up; page returns to full height.
4. Re-enter topbar during the grace period → hide cancelled.
5. ⌘L → topbar reveals + palette opens; close palette (Esc) → topbar hides after grace period.
6. ⌘F → findbar opens, topbar stays pinned while findbar open; Esc closes findbar.
7. ⌘S sidebar collapse/expand with auto-hide ON → topbar spans correct width (traffic-light padding when collapsed), insets correct.
8. Toggle setting OFF → topbar returns to static layout, page inset restored to 44px.
9. Unrelated settings change (e.g. toggle adblock) with auto-hide ON and topbar revealed → topbar does NOT flash/hide.

- [ ] **Step 3: Verify CHANGELOG entry, commit any remaining pre-existing plumbing**

```bash
git add -A
git commit -m "feat: auto-hide topbar config plumbing and changelog"
```

(This commits the pre-existing uncommitted diff: config.rs, lib.rs, ipc.ts, state.ts, types.ts, findbar.ts, CHANGELOG.md.)
