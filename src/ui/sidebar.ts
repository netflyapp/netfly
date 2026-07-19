import { runAction } from '../actions'
import { ipc } from '../ipc'
import { getState, setUi, subscribe } from '../state'
import { el } from '../util/dom'
import { renderTabItem } from './tab-item'

const SIDEBAR_WIDTH = 240
const TOPBAR_HEIGHT = 44

let tabList: HTMLElement
let statusEl: HTMLElement

export function mountSidebar(): void {
  const rootEl = document.getElementById('sidebar') as HTMLElement

  tabList = el('div', { class: 'tab-list', role: 'list' })
  statusEl = el('div', { class: 'sidebar-status', 'aria-live': 'polite' })

  rootEl.append(
    el('div', { class: 'sidebar-drag' }),
    tabList,
    el(
      'button',
      { class: 'new-tab-btn', onclick: () => runAction('tab-new') },
      el('span', { class: 'new-tab-plus', text: '+' }),
      el('span', { text: 'New tab' }),
      el('kbd', { class: 'new-tab-kbd', text: '⌘T' }),
    ),
    statusEl,
  )

  subscribe(render)
  render()
}

function render(): void {
  const { snapshot } = getState()
  tabList.replaceChildren(
    ...snapshot.tabs.map((tab, i) => renderTabItem(tab, i, i === snapshot.activeTab)),
  )
  statusEl.textContent = snapshot.status
  statusEl.classList.toggle('visible', snapshot.status.length > 0)
}

/** Toggle collapse: CSS transition on shell + native re-layout + persist. */
export async function toggleSidebar(): Promise<void> {
  const { config, ui } = getState()
  const collapsed = !ui.sidebarCollapsed
  applySidebarCollapsed(collapsed)
  await syncInsets()
  await ipc.configSetUi(SIDEBAR_WIDTH, collapsed, config.ui.auto_hide_topbar)
}

/** Apply collapse state to shell DOM + store (no persistence). */
export function applySidebarCollapsed(collapsed: boolean): void {
  document.getElementById('app')?.classList.toggle('sidebar-collapsed', collapsed)
  setUi({ sidebarCollapsed: collapsed })
}

/** Push current insets to Rust (boot + after restore). */
export async function syncInsets(): Promise<void> {
  const { config, ui } = getState()
  const topInset = config.ui.auto_hide_topbar && !ui.topbarVisible ? 0 : TOPBAR_HEIGHT
  await ipc.setContentInsets(topInset, ui.sidebarCollapsed ? 0 : SIDEBAR_WIDTH, 0, 0)
}
