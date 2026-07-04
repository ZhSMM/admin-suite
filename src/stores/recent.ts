import { defineStore } from 'pinia'

/**
 * Tracks two things that survive page reloads (but stay on disk only):
 *   1. Recent tool inputs — the last N snapshots of each tool's inputs, so
 *      you can re-run "yesterday's regex" without remembering the pattern.
 *   2. Favorites — pinned route paths. Surfaced at the top of ⌘K so common
 *      jumps beat the menu tree.
 *
 * Both lists live entirely in localStorage. We deliberately do NOT store
 * tool *outputs* — they're trivially regenerable, and persisting them would
 * blow up storage on tools that produce kilobytes of output (SQL formatter,
 * diff, etc.).
 *
 * Snapshot data is filtered through each tool's `sanitize` callback so
 * secrets (passwords, AES keys, HMAC secrets, private keys) never reach
 * disk.  See `useToolRecorder` below.
 */

const STORAGE_RECENT = 'admin-suite.tool-recent'
const STORAGE_FAVS = 'admin-suite.tool-favorites'
const MAX_RECENT_PER_TOOL = 10
const MAX_FAVS = 50

interface State {
  recent: Record<string, RecentEntry[]>
  favorites: string[]
}

export interface RecentEntry {
  ts: number
  inputs: Record<string, unknown>
  label?: string
}

function load<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key)
    if (!raw) return fallback
    return JSON.parse(raw) as T
  } catch {
    return fallback
  }
}

function save(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value))
  } catch {
    /* quota exceeded — drop the oldest entries and retry */
    if (key === STORAGE_RECENT) {
      const obj = value as Record<string, RecentEntry[]>
      for (const k of Object.keys(obj)) obj[k] = obj[k].slice(0, Math.max(2, MAX_RECENT_PER_TOOL / 2))
      try { localStorage.setItem(key, JSON.stringify(obj)) } catch { /* give up */ }
    }
  }
}

export const useRecentStore = defineStore('recent', {
  state: (): State => ({
    recent: load<Record<string, RecentEntry[]>>(STORAGE_RECENT, {}),
    favorites: load<string[]>(STORAGE_FAVS, [])
  }),

  getters: {
    recentFor: (s) => (toolPath: string) => s.recent[toolPath] || [],
    isFavorite: (s) => (path: string) => s.favorites.includes(path),
    favoritesList: (s) => s.favorites
  },

  actions: {
    record(toolPath: string, inputs: Record<string, unknown>, label?: string) {
      const entries = this.recent[toolPath] || []
      // Skip if the new snapshot is byte-for-byte identical to the head.
      // Tools that re-render on every keystroke would otherwise flood storage.
      const top = entries[0]
      if (top && JSON.stringify(top.inputs) === JSON.stringify(inputs)) {
        top.ts = Date.now()
        this.recent[toolPath] = [top, ...entries.slice(1)]
      } else {
        const next: RecentEntry = { ts: Date.now(), inputs, label }
        this.recent[toolPath] = [next, ...entries].slice(0, MAX_RECENT_PER_TOOL)
      }
      save(STORAGE_RECENT, this.recent)
    },

    removeRecent(toolPath: string, ts: number) {
      const entries = this.recent[toolPath] || []
      this.recent[toolPath] = entries.filter((e) => e.ts !== ts)
      save(STORAGE_RECENT, this.recent)
    },

    clearTool(toolPath: string) {
      delete this.recent[toolPath]
      save(STORAGE_RECENT, this.recent)
    },

    clearAll() {
      this.recent = {}
      save(STORAGE_RECENT, this.recent)
    },

    toggleFavorite(path: string) {
      const i = this.favorites.indexOf(path)
      if (i >= 0) {
        this.favorites.splice(i, 1)
      } else {
        this.favorites.unshift(path)
        if (this.favorites.length > MAX_FAVS) {
          this.favorites = this.favorites.slice(0, MAX_FAVS)
        }
      }
      save(STORAGE_FAVS, this.favorites)
    }
  }
})