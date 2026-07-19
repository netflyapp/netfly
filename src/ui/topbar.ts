import { runAction } from '../actions'
import { activeTab, getState, setUi, subscribe } from '../state'
import { el } from '../util/dom'
import { displayUrl } from '../util/favicon'
import { mountFindbar } from './findbar'
import { syncInsets } from './sidebar'

const HIDE_DELAY_MS = 400

let urlPill: HTMLElement
let backBtn: HTMLButtonElement
let fwdBtn: HTMLButtonElement
let hideTimer: ReturnType<typeof setTimeout> | null = null
let autoHideWasOn: boolean | null = null

function navButton(label: string, title: string, action: string): HTMLButtonElement {
  return el('button', {
    class: 'nav-btn',
    title,
    'aria-label': title,
    text: label,
    onclick: () => runAction(action),
  })
}

function setTopbarVisible(visible: boolean): void {
  const rootEl = document.getElementById('topbar') as HTMLElement
  rootEl.classList.toggle('topbar-hidden', !visible)
  setUi({ topbarVisible: visible })
  void syncInsets()
}

function cancelHide(): void {
  if (hideTimer) {
    clearTimeout(hideTimer)
    hideTimer = null
  }
}

/** Hide after a grace period unless something pins the topbar open. */
export function scheduleTopbarHide(): void {
  if (!getState().config.ui.auto_hide_topbar) return
  if (hideTimer) return
  hideTimer = setTimeout(() => {
    hideTimer = null
    const { config, ui } = getState()
    if (!config.ui.auto_hide_topbar) return
    if (ui.findOpen || ui.overlay !== 'none') return
    setTopbarVisible(false)
  }, HIDE_DELAY_MS)
}

function wireAutoHide(rootEl: HTMLElement): void {
  const strip = el('div', { class: 'topbar-trigger' })
  document.body.append(strip)

  strip.addEventListener('mouseenter', showTopbar)
  rootEl.addEventListener('mouseenter', showTopbar)
  rootEl.addEventListener('mouseleave', scheduleTopbarHide)
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

  wireAutoHide(rootEl)
  mountFindbar(rootEl)
  subscribe(render)
  render()
}

/** Apply auto-hide classes; reset visibility only when the mode toggles. */
export function applyAutoHide(): void {
  const on = getState().config.ui.auto_hide_topbar
  const app = document.getElementById('app') as HTMLElement
  const rootEl = document.getElementById('topbar') as HTMLElement
  const strip = document.querySelector('.topbar-trigger') as HTMLElement | null

  app.classList.toggle('auto-hide-topbar', on)
  rootEl.classList.toggle('auto-hide', on)
  strip?.classList.toggle('auto-hide', on)

  if (on === autoHideWasOn) return
  autoHideWasOn = on
  cancelHide()
  setTopbarVisible(!on)
}

/** Reveal now and cancel any pending hide (findbar, palette, hover). */
export function showTopbar(): void {
  if (!getState().config.ui.auto_hide_topbar) return
  cancelHide()
  setTopbarVisible(true)
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
