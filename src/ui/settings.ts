import { ipc } from '../ipc'
import { getState } from '../state'
import { el } from '../util/dom'
import { showOverlay } from './overlay'
import { renderShortcutsSection } from './settings-shortcuts'

type SectionId = 'general' | 'shortcuts' | 'about'

const SECTIONS: { id: SectionId; label: string }[] = [
  { id: 'general', label: 'General' },
  { id: 'shortcuts', label: 'Shortcuts' },
  { id: 'about', label: 'About' },
]

let current: SectionId = 'general'

export async function openSettings(): Promise<void> {
  await showOverlay('settings', (container) => {
    const content = el('div', { class: 'settings-content' })
    const nav = el(
      'nav',
      { class: 'settings-nav', 'aria-label': 'Settings sections' },
      ...SECTIONS.map((s) =>
        el('button', {
          class: `settings-nav-btn${s.id === current ? ' active' : ''}`,
          text: s.label,
          dataset: { section: s.id },
          onclick: () => {
            current = s.id
            nav
              .querySelectorAll('.settings-nav-btn')
              .forEach((b) =>
                b.classList.toggle('active', (b as HTMLElement).dataset.section === s.id),
              )
            renderSection(content)
          },
        }),
      ),
    )

    container.append(
      el(
        'div',
        { class: 'settings-card' },
        el('h1', { class: 'settings-title', text: 'Settings' }),
        el('div', { class: 'settings-body' }, nav, content),
      ),
    )
    renderSection(content)
  })
}

function renderSection(content: HTMLElement): void {
  content.replaceChildren()
  if (current === 'general') renderGeneral(content)
  else if (current === 'shortcuts') renderShortcutsSection(content)
  else renderAbout(content)
}

function field(label: string, control: HTMLElement, hint?: string): HTMLElement {
  return el(
    'div',
    { class: 'settings-field' },
    el('label', { class: 'settings-label', text: label }),
    control,
    hint ? el('p', { class: 'settings-hint', text: hint }) : null,
  )
}

function textInput(value: string, onCommit: (v: string) => void): HTMLInputElement {
  const input = el('input', {
    class: 'settings-input',
    type: 'text',
    value,
    spellcheck: false,
  }) as HTMLInputElement
  input.addEventListener('change', () => onCommit(input.value.trim()))
  return input
}

function toggle(checked: boolean, onChange: (v: boolean) => void): HTMLElement {
  const input = el('input', { type: 'checkbox' }) as HTMLInputElement
  input.checked = checked
  input.addEventListener('change', () => onChange(input.checked))
  return el('label', { class: 'settings-toggle' }, input, el('span', { class: 'toggle-track' }))
}

function renderGeneral(content: HTMLElement): void {
  const { config } = getState()

  content.append(
    field(
      'Start page',
      textInput(config.start_page, (v) => void ipc.configSetGeneral({ startPage: v || 'about:blank' })),
      'Loaded in new tabs and on startup without a session.',
    ),
    field(
      'Default search',
      textInput(config.default_search, (v) => {
        if (v.includes('{}')) void ipc.configSetGeneral({ defaultSearch: v })
      }),
      'URL template with {} as the query placeholder.',
    ),
    field(
      'Restore session on launch',
      toggle(config.restore_session, (v) => void ipc.configSetGeneral({ restoreSession: v })),
    ),
    field(
      'Block ads and trackers',
      toggle(config.adblock, (v) => void ipc.adblockSet(v)),
      'Host-based blocklist with cosmetic filtering.',
    ),
    field(
      'Auto-hide address bar',
      toggle(config.ui.auto_hide_topbar, (v) =>
        void ipc.configSetUi(config.ui.sidebar_width, config.ui.sidebar_collapsed, v),
      ),
      'Hide the top bar when not in use. Hover the top edge to reveal.',
    ),
    field(
      'Downloads folder',
      el('code', { class: 'settings-code', text: config.download_dir }),
    ),
    el(
      'div',
      { class: 'settings-actions' },
      el('button', {
        class: 'ghost-btn',
        text: 'Open config.toml',
        onclick: () => void ipc.configEdit(),
      }),
      el('button', {
        class: 'ghost-btn',
        text: 'Reload config',
        onclick: () => void ipc.configReload(),
      }),
    ),
  )
}

function renderAbout(content: HTMLElement): void {
  const paths = el('div', { class: 'settings-field' })
  content.append(
    el('p', { class: 'about-name', text: 'Netfly' }),
    el('p', {
      class: 'settings-hint',
      text: 'Ultra-light macOS browser · Tauri + system WebKit',
    }),
    paths,
  )
  void (async () => {
    const [data, cfg] = await Promise.all([ipc.dataPath(), ipc.configPath()])
    paths.append(
      field('Data directory', el('code', { class: 'settings-code', text: data })),
      field('Config file', el('code', { class: 'settings-code', text: cfg })),
    )
  })()
}
