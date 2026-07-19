import { ACTION_BY_ID, ACTIONS } from '../actions'
import { ipc } from '../ipc'
import { formatChord, normalizeEvent } from '../shortcuts/chords'
import { actionForChord, chordsForAction } from '../shortcuts/keymap'
import { suspendShortcuts } from '../shortcuts/engine'
import { el } from '../util/dom'

export function renderShortcutsSection(content: HTMLElement): void {
  const table = el('div', { class: 'shortcuts-table' })
  content.append(
    el('p', {
      class: 'settings-hint',
      text: 'Click a shortcut to record a new one. Esc cancels. Saved to config.toml.',
    }),
    table,
  )
  renderRows(table)
}

function renderRows(table: HTMLElement): void {
  const categories = [...new Set(ACTIONS.map((a) => a.category))]
  table.replaceChildren(
    ...categories.flatMap((category) => [
      el('div', { class: 'shortcuts-category', text: category }),
      ...ACTIONS.filter((a) => a.category === category).map((action) =>
        renderRow(table, action.id),
      ),
    ]),
  )
}

function renderRow(table: HTMLElement, actionId: string): HTMLElement {
  const action = ACTION_BY_ID.get(actionId)
  if (!action) return el('div')
  const chords = chordsForAction(actionId)
  const isDefault =
    chords.length === action.defaultChords.length &&
    chords.every((c, i) => c === action.defaultChords[i])

  const pill = el('button', {
    class: `chord-pill${chords.length === 0 ? ' unbound' : ''}`,
    text: chords.length ? chords.map(formatChord).join('  ') : 'unbound',
    title: 'Click to record a new shortcut',
  })
  const row = el(
    'div',
    { class: 'shortcut-row' },
    el('span', { class: 'shortcut-label', text: action.label }),
    el('span', { class: 'shortcut-feedback' }),
    pill,
    el('button', {
      class: `ghost-btn reset-btn${isDefault ? ' hidden' : ''}`,
      text: 'Reset',
      title: 'Restore default shortcut',
      onclick: async () => {
        await ipc.configResetBinding(actionId)
        scheduleRerender(table)
      },
    }),
  )
  pill.addEventListener('click', () => startRecording(table, row, actionId, pill))
  return row
}

function startRecording(
  table: HTMLElement,
  row: HTMLElement,
  actionId: string,
  pill: HTMLElement,
): void {
  suspendShortcuts(true)
  pill.classList.add('recording')
  pill.textContent = 'Press keys…'
  const feedback = row.querySelector('.shortcut-feedback') as HTMLElement
  feedback.textContent = ''

  const stop = (): void => {
    suspendShortcuts(false)
    window.removeEventListener('keydown', onKey, true)
    pill.classList.remove('recording')
  }

  const onKey = (e: KeyboardEvent): void => {
    e.preventDefault()
    e.stopPropagation()
    if (e.key === 'Escape') {
      stop()
      scheduleRerender(table)
      return
    }
    const chord = normalizeEvent(e)
    if (!chord) return // bare modifier — keep recording

    const conflictAction = actionForChord(chord)
    if (conflictAction && conflictAction !== actionId) {
      stop()
      showConflict(table, row, actionId, chord, conflictAction, feedback)
      return
    }
    stop()
    void commitBinding(table, actionId, chord)
  }
  window.addEventListener('keydown', onKey, true)
}

function showConflict(
  table: HTMLElement,
  row: HTMLElement,
  actionId: string,
  chord: string,
  conflictActionId: string,
  feedback: HTMLElement,
): void {
  const other = ACTION_BY_ID.get(conflictActionId)
  feedback.replaceChildren(
    el('span', {
      class: 'conflict-note',
      text: `${formatChord(chord)} is used by “${other?.label ?? conflictActionId}”`,
    }),
    el('button', {
      class: 'ghost-btn danger',
      text: 'Replace',
      onclick: async () => {
        const remaining = chordsForAction(conflictActionId).filter((c) => c !== chord)
        await ipc.configSetBinding(conflictActionId, remaining)
        await commitBinding(table, actionId, chord)
      },
    }),
    el('button', {
      class: 'ghost-btn',
      text: 'Cancel',
      onclick: () => scheduleRerender(table),
    }),
  )
  void row
}

async function commitBinding(table: HTMLElement, actionId: string, chord: string): Promise<void> {
  await ipc.configSetBinding(actionId, [chord])
  scheduleRerender(table)
}

/** Re-render after the config event has refreshed the keymap. */
function scheduleRerender(table: HTMLElement): void {
  setTimeout(() => {
    if (table.isConnected) renderRows(table)
  }, 80)
}
