export interface Action {
  id: string
  label: string
  category: string
  defaultChords: string[]
}

const TAB_JUMPS: Action[] = Array.from({ length: 9 }, (_, i) => ({
  id: `tab-${i + 1}`,
  label: `Go to tab ${i + 1}`,
  category: 'Tabs',
  defaultChords: [`cmd+${i + 1}`],
}))

export const ACTIONS: Action[] = [
  { id: 'tab-new', label: 'New tab', category: 'Tabs', defaultChords: ['cmd+t'] },
  { id: 'tab-close', label: 'Close tab', category: 'Tabs', defaultChords: ['cmd+w'] },
  { id: 'tab-reopen', label: 'Reopen closed tab', category: 'Tabs', defaultChords: ['cmd+shift+t'] },
  { id: 'tab-next', label: 'Next tab', category: 'Tabs', defaultChords: ['ctrl+tab', 'cmd+alt+right'] },
  { id: 'tab-prev', label: 'Previous tab', category: 'Tabs', defaultChords: ['ctrl+shift+tab', 'cmd+alt+left'] },
  ...TAB_JUMPS,
  { id: 'palette', label: 'Open command palette', category: 'Navigation', defaultChords: ['cmd+l'] },
  { id: 'back', label: 'Back', category: 'Navigation', defaultChords: ['cmd+['] },
  { id: 'forward', label: 'Forward', category: 'Navigation', defaultChords: ['cmd+]'] },
  { id: 'reload', label: 'Reload page', category: 'Navigation', defaultChords: ['cmd+r'] },
  { id: 'hard-reload', label: 'Hard reload', category: 'Navigation', defaultChords: ['cmd+shift+r'] },
  { id: 'find', label: 'Find in page', category: 'Page', defaultChords: ['cmd+f'] },
  { id: 'bookmark', label: 'Bookmark page', category: 'Page', defaultChords: ['cmd+d'] },
  { id: 'copy-url', label: 'Copy page URL', category: 'Page', defaultChords: ['cmd+shift+c'] },
  { id: 'sidebar-toggle', label: 'Toggle sidebar', category: 'Window', defaultChords: ['cmd+s'] },
  { id: 'downloads', label: 'Downloads', category: 'Window', defaultChords: ['cmd+j'] },
  { id: 'settings', label: 'Settings', category: 'Window', defaultChords: ['cmd+,'] },
  { id: 'quit', label: 'Quit Netfly', category: 'Window', defaultChords: ['cmd+q'] },
]

export const ACTION_BY_ID = new Map(ACTIONS.map((a) => [a.id, a]))

type Runner = () => void | Promise<void>

const runners = new Map<string, Runner>()

/** Wire concrete handlers (done once at boot in main.ts). */
export function registerRunners(map: Record<string, Runner>): void {
  for (const [id, fn] of Object.entries(map)) runners.set(id, fn)
}

export function runAction(id: string): void {
  const fn = runners.get(id)
  if (fn) void fn()
}
