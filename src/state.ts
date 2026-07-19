import type { Config, OverlayView, Snapshot } from './types'

export interface UiState {
  sidebarCollapsed: boolean
  overlay: OverlayView
  findOpen: boolean
  topbarVisible: boolean
}

export interface AppState {
  snapshot: Snapshot
  config: Config
  ui: UiState
}

type Listener = (state: AppState) => void

const DEFAULT_SNAPSHOT: Snapshot = {
  tabs: [{ id: 'content-0', url: 'about:blank', title: 'New Tab', loading: false }],
  activeTab: 0,
  status: '',
}

const DEFAULT_CONFIG: Config = {
  start_page: 'about:blank',
  download_dir: '~/Downloads',
  default_search: 'https://duckduckgo.com/?q={}',
  restore_session: true,
  adblock: true,
  ui: { sidebar_width: 240, sidebar_collapsed: false, auto_hide_topbar: false },
  search_engines: {},
  bindings: {},
}

let state: AppState = {
  snapshot: DEFAULT_SNAPSHOT,
  config: DEFAULT_CONFIG,
  ui: { sidebarCollapsed: false, overlay: 'none', findOpen: false, topbarVisible: true },
}

const listeners = new Set<Listener>()

export function getState(): AppState {
  return state
}

export function setState(partial: Partial<AppState>): void {
  state = { ...state, ...partial }
  for (const fn of listeners) fn(state)
}

export function setUi(partial: Partial<UiState>): void {
  setState({ ui: { ...state.ui, ...partial } })
}

export function subscribe(fn: Listener): () => void {
  listeners.add(fn)
  return () => listeners.delete(fn)
}

export function activeTab() {
  const { tabs, activeTab: idx } = state.snapshot
  return tabs[Math.min(idx, tabs.length - 1)] ?? DEFAULT_SNAPSHOT.tabs[0]
}
