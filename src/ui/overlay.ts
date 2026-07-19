import { ipc } from '../ipc'
import { getState, setUi } from '../state'
import type { OverlayView } from '../types'

type Cleanup = (() => void) | void

let cleanup: Cleanup = undefined

function root(): HTMLElement {
  return document.getElementById('overlay') as HTMLElement
}

/**
 * Show a full-window overlay (native content webview is hidden while open,
 * because child webviews always render above the shell).
 */
export async function showOverlay(
  view: OverlayView,
  build: (container: HTMLElement) => Cleanup,
): Promise<void> {
  const container = root()
  if (cleanup) cleanup()
  container.replaceChildren()
  container.hidden = false
  container.dataset.view = view
  setUi({ overlay: view })
  await ipc.setOverlay(true)
  container.addEventListener('mousedown', onBackdropClick)
  cleanup = build(container)
}

export async function hideOverlay(): Promise<void> {
  if (getState().ui.overlay === 'none') return
  const container = root()
  if (cleanup) cleanup()
  cleanup = undefined
  container.removeEventListener('mousedown', onBackdropClick)
  container.hidden = true
  container.replaceChildren()
  setUi({ overlay: 'none' })
  await ipc.setOverlay(false)
}

export function overlayOpen(): boolean {
  return getState().ui.overlay !== 'none'
}

function onBackdropClick(e: MouseEvent): void {
  if (e.target === root()) void hideOverlay()
}
