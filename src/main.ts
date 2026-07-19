import './styles/tokens.css'
import './styles/base.css'
import './styles/topbar.css'
import './styles/sidebar.css'
import './styles/overlay.css'
import './styles/palette.css'
import './styles/settings.css'
import './styles/findbar.css'

import { registerRunners } from './actions'
import { events, ipc } from './ipc'
import { rebuildKeymap, syncChordsToContent } from './shortcuts/keymap'
import { startShortcutEngine } from './shortcuts/engine'
import { activeTab, getState, setState } from './state'
import { openDownloads } from './ui/downloads-view'
import { closeFindbar, openFindbar } from './ui/findbar'
import { hideOverlay, overlayOpen } from './ui/overlay'
import { openPalette } from './ui/palette'
import { openSettings } from './ui/settings'
import { applySidebarCollapsed, mountSidebar, syncInsets, toggleSidebar } from './ui/sidebar'
import { applyAutoHide, mountTopbar, showTopbar } from './ui/topbar'
import { displayUrl } from './util/favicon'

function wireActions(): void {
  registerRunners({
    'tab-new': async () => {
      await ipc.tabNew()
      await openPalette()
    },
    'tab-close': () => void ipc.tabClose(),
    'tab-reopen': () => void ipc.tabUndoClose(),
    'tab-next': () => void ipc.tabNext(),
    'tab-prev': () => void ipc.tabPrev(),
    ...Object.fromEntries(
      Array.from({ length: 9 }, (_, i) => [
        `tab-${i + 1}`,
        () => void ipc.tabSelect(i),
      ]),
    ),
    palette: () => {
      showTopbar()
      void openPalette()
    },
    back: () => void ipc.goBack(),
    forward: () => void ipc.goForward(),
    reload: () => void ipc.reload(false),
    'hard-reload': () => void ipc.reload(true),
    find: () => openFindbar(),
    bookmark: async () => {
      const tab = activeTab()
      const name = tab.title || displayUrl(tab.url) || 'untitled'
      await ipc.bookmarkSet(name)
    },
    'copy-url': () => void ipc.yankUrl(),
    'sidebar-toggle': () => void toggleSidebar(),
    downloads: () => void openDownloads(),
    settings: () => void openSettings(),
    quit: () => void ipc.quit(),
  })
}

function wireEvents(): void {
  void events.onSnapshot((snapshot) => setState({ snapshot }))
  void events.onConfig((config) => {
    setState({ config })
    rebuildKeymap(config)
    syncChordsToContent()
    applySidebarCollapsed(config.ui.sidebar_collapsed)
    applyAutoHide()
  })
  void events.onEscape(() => void handleEscape())

  // Escape inside the shell (overlay backdrop, settings, sidebar focus).
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && overlayOpen()) {
      e.preventDefault()
      void hideOverlay()
    }
  })
}

async function handleEscape(): Promise<void> {
  if (getState().ui.findOpen) {
    await closeFindbar()
  } else if (overlayOpen()) {
    await hideOverlay()
  }
}

async function boot(): Promise<void> {
  wireActions()
  mountSidebar()
  mountTopbar()
  wireEvents()
  startShortcutEngine()

  const [config, snapshot] = await Promise.all([ipc.getConfig(), ipc.getSnapshot()])
  setState({ config, snapshot })
  rebuildKeymap(config)
  syncChordsToContent()
  applySidebarCollapsed(config.ui.sidebar_collapsed)
  applyAutoHide()
  await syncInsets()
}

void boot()
