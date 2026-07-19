import { ipc } from '../ipc'
import type { Snapshot } from '../types'
import { displayUrl } from '../util/favicon'

export interface Suggestion {
  kind: 'tab' | 'bookmark' | 'history' | 'url' | 'search'
  title: string
  detail: string
  /** Run the suggestion. `newTab` only applies to url-opening kinds. */
  run: (newTab: boolean) => Promise<unknown>
}

const MAX_TABS = 4
const MAX_BOOKMARKS = 4
const MAX_HISTORY = 8

function matches(query: string, ...haystacks: string[]): boolean {
  const q = query.toLowerCase()
  return haystacks.some((h) => h.toLowerCase().includes(q))
}

function looksLikeUrl(raw: string): boolean {
  if (/^https?:\/\//.test(raw) || raw.startsWith('file://')) return true
  return raw.includes('.') && !raw.includes(' ')
}

export async function gatherSuggestions(
  query: string,
  snapshot: Snapshot,
): Promise<Suggestion[]> {
  const q = query.trim()
  const out: Suggestion[] = []

  if (q) {
    const tabHits = snapshot.tabs
      .map((tab, index) => ({ tab, index }))
      .filter(({ tab }) => matches(q, tab.title, tab.url))
      .slice(0, MAX_TABS)
    for (const { tab, index } of tabHits) {
      out.push({
        kind: 'tab',
        title: tab.title || displayUrl(tab.url) || 'New Tab',
        detail: `Switch to tab · ${displayUrl(tab.url)}`,
        run: () => ipc.tabSelect(index),
      })
    }

    try {
      const store = await ipc.listBookmarks()
      const hits = Object.entries(store.bookmarks)
        .filter(([name, url]) => matches(q, name, url))
        .slice(0, MAX_BOOKMARKS)
      for (const [name, url] of hits) {
        out.push({
          kind: 'bookmark',
          title: name,
          detail: displayUrl(url),
          run: (newTab) => ipc.openUrl(url, newTab),
        })
      }
    } catch {
      /* bookmarks unavailable — skip source */
    }

    try {
      const entries = await ipc.historySearch(q, MAX_HISTORY)
      for (const entry of entries) {
        out.push({
          kind: 'history',
          title: entry.title || displayUrl(entry.url),
          detail: displayUrl(entry.url),
          run: (newTab) => ipc.openUrl(entry.url, newTab),
        })
      }
    } catch {
      /* history unavailable — skip source */
    }
  }

  // Fallback rows — always present so Enter has a target.
  if (q && looksLikeUrl(q)) {
    out.push({
      kind: 'url',
      title: `Go to ${q}`,
      detail: 'Open address',
      run: (newTab) => ipc.openUrl(q, newTab),
    })
  }
  if (q) {
    out.push({
      kind: 'search',
      title: `Search for “${q}”`,
      detail: 'Web search',
      run: (newTab) => ipc.openUrl(q, newTab),
    })
  }
  return out
}
