/**
 * useRecentStore covers local persistence + dedup.  We mock localStorage
 * with happy-dom's built-in so each test gets a fresh store.
 */
import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import { useRecentStore } from '@/stores/recent'

describe('useRecentStore — record & dedup', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    localStorage.clear()
  })

  it('records the first snapshot at the head', () => {
    const s = useRecentStore()
    s.record('/tools/hash', { text: 'hello', algorithm: 'SHA-256' })
    expect(s.recentFor('/tools/hash')).toHaveLength(1)
    expect(s.recentFor('/tools/hash')[0].inputs.text).toBe('hello')
  })

  it('dedupes consecutive identical snapshots (bumping ts)', () => {
    const s = useRecentStore()
    const t0 = 1_000_000
    s.record('/tools/hash', { text: 'hello', algorithm: 'SHA-256' })
    s.recent['/tools/hash'][0].ts = t0
    const before = s.recentFor('/tools/hash')[0]
    s.record('/tools/hash', { text: 'hello', algorithm: 'SHA-256' })
    const after = s.recentFor('/tools/hash')[0]
    expect(after).toBe(before) // same object reference → dedup hit
    expect(s.recentFor('/tools/hash')).toHaveLength(1)
  })

  it('keeps at most 10 entries per tool, newest first', () => {
    const s = useRecentStore()
    for (let i = 0; i < 15; i++) {
      s.record('/tools/hash', { text: `msg-${i}` })
    }
    const entries = s.recentFor('/tools/hash')
    expect(entries).toHaveLength(10)
    expect(entries[0].inputs.text).toBe('msg-14')
    expect(entries[9].inputs.text).toBe('msg-5')
  })

  it('per-tool namespaces are independent', () => {
    const s = useRecentStore()
    s.record('/tools/hash', { x: 1 })
    s.record('/tools/crypto', { y: 2 })
    expect(s.recentFor('/tools/hash')).toHaveLength(1)
    expect(s.recentFor('/tools/crypto')).toHaveLength(1)
  })

  it('removeRecent and clearTool target the right entries', () => {
    const s = useRecentStore()
    s.record('/tools/hash', { a: 1 })
    // Force a unique timestamp for the second entry so the dedup branch
    // doesn't fire and we can verify the precise remove-by-ts contract.
    s.record('/tools/hash', { a: 2 })
    s.recent['/tools/hash'][1].ts = s.recent['/tools/hash'][0].ts - 1
    const ts = s.recentFor('/tools/hash')[1].ts
    s.removeRecent('/tools/hash', ts)
    expect(s.recentFor('/tools/hash')).toHaveLength(1)
    expect(s.recentFor('/tools/hash')[0].inputs.a).toBe(2)
    s.clearTool('/tools/hash')
    expect(s.recentFor('/tools/hash')).toHaveLength(0)
  })

  it('clearAll wipes every tool', () => {
    const s = useRecentStore()
    s.record('/tools/hash', {})
    s.record('/tools/crypto', {})
    s.clearAll()
    expect(Object.keys(s.recent)).toHaveLength(0)
  })
})

describe('useRecentStore — favorites', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    localStorage.clear()
  })

  it('toggleFavorite adds and removes', () => {
    const s = useRecentStore()
    expect(s.isFavorite('/tools/hash')).toBe(false)
    s.toggleFavorite('/tools/hash')
    expect(s.isFavorite('/tools/hash')).toBe(true)
    s.toggleFavorite('/tools/hash')
    expect(s.isFavorite('/tools/hash')).toBe(false)
  })

  it('newest favorite goes to the head', () => {
    const s = useRecentStore()
    s.toggleFavorite('/a')
    s.toggleFavorite('/b')
    s.toggleFavorite('/c')
    expect(s.favoritesList).toEqual(['/c', '/b', '/a'])
  })

  it('caps at 50 favorites', () => {
    const s = useRecentStore()
    for (let i = 0; i < 60; i++) s.toggleFavorite(`/p/${i}`)
    expect(s.favoritesList.length).toBe(50)
  })

  it('persists to localStorage', () => {
    const s = useRecentStore()
    s.toggleFavorite('/x')
    const raw = localStorage.getItem('admin-suite.tool-favorites')
    expect(raw).toBeTruthy()
    expect(JSON.parse(raw!)).toContain('/x')
  })
})