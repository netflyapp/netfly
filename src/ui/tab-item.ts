import { ipc } from '../ipc'
import type { TabInfo } from '../types'
import { el } from '../util/dom'
import { displayUrl, faviconFor, letterFor } from '../util/favicon'

export function renderTabItem(tab: TabInfo, index: number, isActive: boolean): HTMLElement {
  const icon = el('span', { class: 'tab-icon', text: letterFor(tab.url, tab.title) })
  const src = faviconFor(tab.url)
  if (src) {
    const img = el('img', { class: 'tab-favicon', src, alt: '' }) as HTMLImageElement
    img.addEventListener('error', () => img.remove())
    img.addEventListener('load', () => icon.classList.add('has-favicon'))
    icon.append(img)
  }

  const title = tab.title || displayUrl(tab.url) || 'New Tab'
  const row = el(
    'div',
    {
      class: `tab-item${isActive ? ' active' : ''}${tab.loading ? ' loading' : ''}`,
      role: 'button',
      tabindex: '0',
      title: tab.url,
      onclick: () => void ipc.tabSelect(index),
    },
    icon,
    el('span', { class: 'tab-title', text: title }),
    el('button', {
      class: 'tab-close',
      title: 'Close tab',
      'aria-label': `Close ${title}`,
      onclick: (e: Event) => {
        e.stopPropagation()
        void closeTabAt(index)
      },
    }, '×'),
  )
  return row
}

async function closeTabAt(index: number): Promise<void> {
  await ipc.tabSelect(index)
  await ipc.tabClose()
}
