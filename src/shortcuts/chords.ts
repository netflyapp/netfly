/** Chord format: "cmd+shift+t" — modifiers in cmd,ctrl,alt,shift order. */

const KEY_ALIASES: Record<string, string> = {
  arrowleft: 'left',
  arrowright: 'right',
  arrowup: 'up',
  arrowdown: 'down',
  ' ': 'space',
  escape: 'esc',
}

const MODIFIER_KEYS = new Set(['meta', 'control', 'alt', 'shift'])

/** Normalize a KeyboardEvent to a chord string, or null for bare modifiers. */
export function normalizeEvent(e: KeyboardEvent): string | null {
  let key = e.key.toLowerCase()
  if (MODIFIER_KEYS.has(key)) return null
  key = KEY_ALIASES[key] ?? key
  const parts: string[] = []
  if (e.metaKey) parts.push('cmd')
  if (e.ctrlKey) parts.push('ctrl')
  if (e.altKey) parts.push('alt')
  if (e.shiftKey) parts.push('shift')
  parts.push(key)
  return parts.join('+')
}

const MOD_SYMBOLS: Record<string, string> = {
  cmd: '⌘',
  ctrl: '⌃',
  alt: '⌥',
  shift: '⇧',
}

const KEY_SYMBOLS: Record<string, string> = {
  left: '←',
  right: '→',
  up: '↑',
  down: '↓',
  tab: '⇥',
  enter: '↩',
  backspace: '⌫',
  space: '␣',
  esc: '⎋',
}

/** Pretty display: "cmd+shift+t" → "⌘⇧T". */
export function formatChord(chord: string): string {
  return chord
    .split('+')
    .map((part) => MOD_SYMBOLS[part] ?? KEY_SYMBOLS[part] ?? part.toUpperCase())
    .join('')
}

/** True when the chord includes cmd or ctrl (safe to act on inside inputs). */
export function hasPrimaryModifier(chord: string): boolean {
  return chord.startsWith('cmd+') || chord.startsWith('ctrl+') || chord.includes('+ctrl+')
}
