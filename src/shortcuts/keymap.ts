import { ACTIONS } from '../actions'
import { ipc } from '../ipc'
import type { ChordSet, Config } from '../types'

let chordToAction = new Map<string, string>()
let actionToChords = new Map<string, string[]>()

function overrideChords(set: ChordSet): string[] {
  return typeof set === 'string' ? [set] : set
}

/** Rebuild the merged keymap (defaults ⊕ config overrides). */
export function rebuildKeymap(config: Config): void {
  const nextByChord = new Map<string, string>()
  const nextByAction = new Map<string, string[]>()
  for (const action of ACTIONS) {
    const override = config.bindings[action.id]
    const chords = override !== undefined ? overrideChords(override) : action.defaultChords
    nextByAction.set(action.id, chords)
    for (const chord of chords) nextByChord.set(chord, action.id)
  }
  chordToAction = nextByChord
  actionToChords = nextByAction
}

/** Push the merged chord list into content webviews via Rust. */
export function syncChordsToContent(): void {
  void ipc.setActiveChords([...chordToAction.keys()])
}

export function actionForChord(chord: string): string | undefined {
  return chordToAction.get(chord)
}

export function chordsForAction(actionId: string): string[] {
  return actionToChords.get(actionId) ?? []
}
