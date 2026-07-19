type Child = Node | string | null | undefined

interface Attrs {
  class?: string
  text?: string
  title?: string
  html?: never
  [key: string]: unknown
}

/** Tiny hyperscript helper: el('button', { class: 'x', onclick }, 'label') */
export function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  attrs: Attrs = {},
  ...children: Child[]
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag)
  for (const [key, value] of Object.entries(attrs)) {
    if (value == null) continue
    if (key === 'class') {
      node.className = value as string
    } else if (key === 'text') {
      node.textContent = value as string
    } else if (key.startsWith('on') && typeof value === 'function') {
      node.addEventListener(key.slice(2), value as EventListener)
    } else if (key === 'dataset') {
      Object.assign(node.dataset, value)
    } else if (key in node) {
      ;(node as unknown as Record<string, unknown>)[key] = value
    } else {
      node.setAttribute(key, String(value))
    }
  }
  for (const child of children) {
    if (child == null) continue
    node.append(child)
  }
  return node
}

export function clear(node: HTMLElement): void {
  node.replaceChildren()
}
