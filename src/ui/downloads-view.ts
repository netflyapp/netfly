import { ipc } from '../ipc'
import type { DownloadItem } from '../types'
import { el } from '../util/dom'
import { showOverlay } from './overlay'

function statusGlyph(item: DownloadItem): string {
  if (item.success === true) return '✓'
  if (item.success === false) return '✗'
  return '↓'
}

export async function openDownloads(): Promise<void> {
  await showOverlay('downloads', (container) => {
    const list = el('div', { class: 'downloads-list' })
    const card = el(
      'div',
      { class: 'palette-card downloads-card' },
      el(
        'div',
        { class: 'downloads-header' },
        el('h2', { text: 'Downloads' }),
        el('button', {
          class: 'ghost-btn',
          text: 'Open folder',
          onclick: () => void ipc.downloadsOpenDir(),
        }),
        el('button', {
          class: 'ghost-btn',
          text: 'Clear',
          onclick: async () => {
            await ipc.downloadsClear()
            await render()
          },
        }),
      ),
      list,
    )
    container.append(card)

    async function render(): Promise<void> {
      const items = await ipc.downloadsList()
      if (items.length === 0) {
        list.replaceChildren(el('div', { class: 'empty-note', text: 'No downloads yet' }))
        return
      }
      list.replaceChildren(
        ...items.map((item) =>
          el(
            'div',
            {
              class: `download-row status-${item.success === false ? 'failed' : item.success === true ? 'done' : 'active'}`,
              onclick: () => void ipc.downloadsOpenFile(item.id),
              title: item.path,
            },
            el('span', { class: 'download-glyph', text: statusGlyph(item) }),
            el('span', { class: 'row-title', text: item.filename }),
            el('span', { class: 'row-detail', text: item.path }),
          ),
        ),
      )
    }
    void render()
  })
}
