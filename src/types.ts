export interface TabInfo {
  id: string
  url: string
  title: string
  loading: boolean
}

export interface Snapshot {
  tabs: TabInfo[]
  activeTab: number
  status: string
}

/** A binding override: one chord, or a list (empty = explicitly unbound). */
export type ChordSet = string | string[]

export interface UiConfig {
  sidebar_width: number
  sidebar_collapsed: boolean
  auto_hide_topbar: boolean
}

export interface Config {
  start_page: string
  download_dir: string
  default_search: string
  restore_session: boolean
  adblock: boolean
  ui: UiConfig
  search_engines: Record<string, string>
  bindings: Record<string, ChordSet>
}

export interface HistoryEntry {
  url: string
  title: string
  visitCount: number
  lastVisit: number
}

export interface BookmarkStore {
  bookmarks: Record<string, string>
  quickmarks: Record<string, string>
}

export interface DownloadItem {
  id: number
  url: string
  filename: string
  path: string
  status: string
  success: boolean | null
  startedAt: number
  finishedAt: number | null
}

export interface AdblockStatus {
  enabled: boolean
  hosts: number
  blocked: number
}

export type OverlayView = 'none' | 'palette' | 'downloads' | 'settings'
