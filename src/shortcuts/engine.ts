import { runAction } from '../actions'
import { events } from '../ipc'
import { hasPrimaryModifier, normalizeEvent } from './chords'
import { actionForChord } from './keymap'

/**
 * When another module needs raw key capture (shortcut recording in
 * settings), it suspends dispatch to avoid firing actions mid-recording.
 */
let suspended = false

export function suspendShortcuts(value: boolean): void {
  suspended = value
}

function isTextTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false
  return (
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    target.isContentEditable
  )
}

function dispatch(chord: string, e?: KeyboardEvent): void {
  if (suspended) return
  const actionId = actionForChord(chord)
  if (!actionId) return
  // Plain keys keep working inside shell inputs (palette, settings).
  if (e && isTextTarget(e.target) && !hasPrimaryModifier(chord)) return
  e?.preventDefault()
  e?.stopPropagation()
  runAction(actionId)
}

/** Start both capture paths: shell keydown + content-forwarded chords. */
export function startShortcutEngine(): void {
  window.addEventListener(
    'keydown',
    (e) => {
      const chord = normalizeEvent(e)
      if (chord) dispatch(chord, e)
    },
    true,
  )
  void events.onAction((chord) => dispatch(chord))
}
