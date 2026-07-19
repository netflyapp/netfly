import { runAction } from '../actions'
import { activeTab, getState, subscribe } from '../state'
import { el } from '../util/dom'
import { displayUrl } from '../util/favicon'
import { mountFindbar } from './findbar'

let urlPill: HTMLElement
let backBtn: HTMLButtonElement
let fwdBtn: HTMLButtonElement

function navButton(label: string, title: string, action: string): HTMLButtonElement {
  return el('button', {
    class: 'nav-btn',
    title,
    'aria-label': title,
    text: label,
    onclick: () => runAction(action),
  })
}

export function mountTopbar(): void {
  const rootEl = document.getElementById('topbar') as HTMLElement

  backBtn = navButton('←', 'Back (⌘[)', 'back')
  fwdBtn = navButton('→', 'Forward (⌘])', 'forward')

  urlPill = el(
    'button',
    {
      class: 'url-pill',
      title: 'Open command palette (⌘L)',
      onclick: () => runAction('palette'),
    },
    el('span', { class: 'url-pill-icon', text: '⌘L' }),
    el('span', { class: 'url-pill-text' }),
  )

  rootEl.append(
    el('button', {
      class: 'nav-btn sidebar-btn',
      title: 'Toggle sidebar (⌘S)',
      'aria-label': 'Toggle sidebar',
      text: '☰',
      onclick: () => runAction('sidebar-toggle'),
    }),
    backBtn,
    fwdBtn,
    navButton('↻', 'Reload (⌘R)', 'reload'),
    urlPill,
    navButton('↓', 'Downloads (⌘J)', 'downloads'),
    navButton('⚙', 'Settings (⌘,)', 'settings'),
  )

  mountFindbar(rootEl)
  subscribe(render)
  render()
}

function render(): void {
  const tab = activeTab()
  const { findOpen } = getState().ui
  const text = urlPill.querySelector('.url-pill-text') as HTMLElement
  const display = displayUrl(tab.url)
  text.textContent = display || 'Search or enter address'
  urlPill.classList.toggle('placeholder', !display)
  urlPill.classList.toggle('loading', tab.loading)
  document.getElementById('topbar')?.classList.toggle('find-open', findOpen)
}
