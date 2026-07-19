import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type {
  AdblockStatus,
  BookmarkStore,
  Config,
  DownloadItem,
  HistoryEntry,
  Snapshot,
} from './types'

export const ipc = {
  getSnapshot: () => invoke<Snapshot>('get_snapshot'),
  getConfig: () => invoke<Config>('get_config'),

  openUrl: (input: string, newTab = false) => invoke<Snapshot>('open_url', { input, newTab }),
  tabNew: () => invoke<Snapshot>('tab_new'),
  tabClose: () => invoke<Snapshot>('tab_close'),
  tabUndoClose: () => invoke<Snapshot>('tab_undo_close'),
  tabNext: () => invoke<Snapshot>('tab_next'),
  tabPrev: () => invoke<Snapshot>('tab_prev'),
  tabSelect: (index: number) => invoke<Snapshot>('tab_select', { index }),

  goBack: () => invoke<void>('go_back'),
  goForward: () => invoke<void>('go_forward'),
  reload: (hard = false) => invoke<void>('reload', { hard }),
  findInPage: (query: string, forward = true) =>
    invoke<void>('find_in_page', { query, forward }),
  yankUrl: () => invoke<string>('yank_url'),
  focusPage: () => invoke<void>('focus_page'),

  setContentInsets: (top: number, left: number, right: number, bottom: number) =>
    invoke<void>('set_content_insets', { top, left, right, bottom }),
  setOverlay: (open: boolean) => invoke<void>('set_overlay', { open }),
  setActiveChords: (chords: string[]) => invoke<void>('set_active_chords', { chords }),

  configSetBinding: (action: string, chords: string[]) =>
    invoke<Config>('config_set_binding', { action, chords }),
  configResetBinding: (action: string) => invoke<Config>('config_reset_binding', { action }),
  configSetUi: (sidebarWidth: number, sidebarCollapsed: boolean, autoHideTopbar: boolean) =>
    invoke<Config>('config_set_ui', { sidebarWidth, sidebarCollapsed, autoHideTopbar }),
  configSetGeneral: (opts: {
    startPage?: string
    defaultSearch?: string
    restoreSession?: boolean
  }) => invoke<Config>('config_set_general', opts),
  configReload: () => invoke<Config>('config_reload'),
  configPath: () => invoke<string>('config_path'),
  configEdit: () => invoke<string>('config_edit'),
  dataPath: () => invoke<string>('data_path'),

  historySearch: (query: string, limit = 30) =>
    invoke<HistoryEntry[]>('history_search', { query, limit }),
  listBookmarks: () => invoke<BookmarkStore>('list_bookmarks'),
  bookmarkSet: (name: string, url?: string) => invoke<void>('bookmark_set', { name, url }),

  downloadsList: () => invoke<DownloadItem[]>('downloads_list'),
  downloadsClear: () => invoke<void>('downloads_clear'),
  downloadsOpenDir: () => invoke<string>('downloads_open_dir'),
  downloadsOpenFile: (id: number) => invoke<string>('downloads_open_file', { id }),

  adblockStatus: () => invoke<AdblockStatus>('adblock_status'),
  adblockSet: (enabled: boolean) => invoke<AdblockStatus>('adblock_set', { enabled }),

  quit: () => invoke<void>('quit_app'),
}

export const events = {
  onSnapshot: (cb: (snap: Snapshot) => void) =>
    listen<Snapshot>('browser://snapshot', (e) => cb(e.payload)),
  onConfig: (cb: (cfg: Config) => void) =>
    listen<Config>('browser://config', (e) => cb(e.payload)),
  onAction: (cb: (chord: string) => void) =>
    listen<string>('browser://action', (e) => cb(e.payload)),
  onEscape: (cb: () => void) => listen<void>('browser://escape', () => cb()),
  onDownload: (cb: (item: DownloadItem) => void) =>
    listen<DownloadItem>('browser://download', (e) => cb(e.payload)),
}
