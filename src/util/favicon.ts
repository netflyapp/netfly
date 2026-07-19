/** Favicon URL for a page URL, or null when the host is unusable. */
export function faviconFor(url: string): string | null {
  try {
    const host = new URL(url).hostname
    if (!host) return null
    return `https://www.google.com/s2/favicons?domain=${encodeURIComponent(host)}&sz=32`
  } catch {
    return null
  }
}

/** Single-letter fallback badge text for a URL. */
export function letterFor(url: string, title: string): string {
  try {
    const host = new URL(url).hostname.replace(/^www\./, '')
    if (host) return host[0].toUpperCase()
  } catch {
    /* not a URL */
  }
  return (title.trim()[0] ?? '•').toUpperCase()
}

/** Compact display form of a URL for pills and lists. */
export function displayUrl(url: string): string {
  if (url === 'about:blank' || url === '') return ''
  try {
    const u = new URL(url)
    const path = u.pathname === '/' ? '' : u.pathname
    return u.hostname.replace(/^www\./, '') + path
  } catch {
    return url
  }
}
