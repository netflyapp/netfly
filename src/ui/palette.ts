import { activeTab, getState } from '../state'
import { el } from '../util/dom'
import { debounce } from '../util/debounce'
import { displayUrl } from '../util/favicon'
import { hideOverlay, showOverlay } from './overlay'
import { gatherSuggestions, type Suggestion } from './palette-sources'

const KIND_ICONS: Record<Suggestion['kind'], string> = {
  tab: '⧉',
  bookmark: '☆',
  history: '◷',
  url: '→',
  search: '⌕',
}

let suggestions: Suggestion[] = []
let selected = 0
let listEl: HTMLElement

export async function openPalette(): Promise<void> {
  await showOverlay('palette', (container) => {
    const input = el('input', {
      class: 'palette-input',
      type: 'text',
      placeholder: 'Search or enter address…',
      spellcheck: false,
      'aria-label': 'Command palette',
    }) as HTMLInputElement

    listEl = el('div', { class: 'palette-list', role: 'listbox' })

    const tab = activeTab()
    const context = displayUrl(tab.url)
    const card = el(
      'div',
      { class: 'palette-card' },
      context
        ? el('div', { class: 'palette-context', text: `${tab.title || context}` })
        : null,
      input,
      listEl,
      el(
        'div',
        { class: 'palette-hints' },
        el('span', {}, el('kbd', { text: '↩' }), ' open'),
        el('span', {}, el('kbd', { text: '⌘↩' }), ' new tab'),
        el('span', {}, el('kbd', { text: '⎋' }), ' close'),
      ),
    )
    container.append(card)

    const refresh = debounce((value: string) => void updateSuggestions(value), 80)
    input.addEventListener('input', () => refresh(input.value))
    input.addEventListener('keydown', onKeyDown)
    void updateSuggestions('')
    queueMicrotask(() => input.focus())
  })
}

async function updateSuggestions(query: string): Promise<void> {
  suggestions = await gatherSuggestions(query, getState().snapshot)
  selected = 0
  renderList()
}

function renderList(): void {
  if (!listEl.isConnected) return
  listEl.replaceChildren(
    ...suggestions.map((s, i) =>
      el(
        'div',
        {
          class: `palette-row${i === selected ? ' selected' : ''}`,
          role: 'option',
          'aria-selected': String(i === selected),
          onclick: () => void accept(i, false),
        },
        el('span', { class: `row-icon kind-${s.kind}`, text: KIND_ICONS[s.kind] }),
        el('span', { class: 'row-title', text: s.title }),
        el('span', { class: 'row-detail', text: s.detail }),
      ),
    ),
  )
}

function onKeyDown(e: KeyboardEvent): void {
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    selected = Math.min(selected + 1, suggestions.length - 1)
    renderList()
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    selected = Math.max(selected - 1, 0)
    renderList()
  } else if (e.key === 'Enter') {
    e.preventDefault()
    void accept(selected, e.metaKey)
  } else if (e.key === 'Escape') {
    e.preventDefault()
    void hideOverlay()
  }
}

async function accept(index: number, newTab: boolean): Promise<void> {
  const suggestion = suggestions[index]
  if (!suggestion) return
  await hideOverlay()
  await suggestion.run(newTab)
}
