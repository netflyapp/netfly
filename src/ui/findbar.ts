import { ipc } from '../ipc'
import { getState, setUi } from '../state'
import { el } from '../util/dom'

let input: HTMLInputElement
let bar: HTMLElement

export function mountFindbar(topbar: HTMLElement): void {
  input = el('input', {
    class: 'find-input',
    type: 'text',
    placeholder: 'Find in page…',
    spellcheck: false,
  }) as HTMLInputElement

  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault()
      void ipc.findInPage(input.value, !e.shiftKey)
    } else if (e.key === 'Escape') {
      e.preventDefault()
      void closeFindbar()
    }
  })

  bar = el(
    'div',
    { class: 'findbar' },
    input,
    el('button', {
      class: 'nav-btn',
      title: 'Previous match (⇧↩)',
      text: '↑',
      onclick: () => void ipc.findInPage(input.value, false),
    }),
    el('button', {
      class: 'nav-btn',
      title: 'Next match (↩)',
      text: '↓',
      onclick: () => void ipc.findInPage(input.value, true),
    }),
    el('button', {
      class: 'nav-btn',
      title: 'Close (Esc)',
      text: '×',
      onclick: () => void closeFindbar(),
    }),
  )
  topbar.append(bar)
}

export function openFindbar(): void {
  setUi({ findOpen: true })
  input.focus()
  input.select()
}

export async function closeFindbar(): Promise<void> {
  if (!getState().ui.findOpen) return
  setUi({ findOpen: false })
  input.value = ''
  await ipc.focusPage()
}
