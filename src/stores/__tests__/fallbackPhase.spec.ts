// v0.6.3 regression test — Rust serializes `Phase` with mixed shapes:
// unit variants come through as bare strings ("not_downloaded"), struct
// variants as objects ({ ready: {...} }). The UI helpers below handle both.
//
// If the Rust enum gains a new variant and this test doesn't update, the
// UI may crash with "Cannot use 'in' operator to search for X in Y".

import { describe, it, expect } from 'vitest'
import type { FallbackState } from '../../api/llm'

// Re-implement the helpers here so we can exercise them without mounting
// the Vue component. Keep this in sync with LocalModelPanel.vue.
function isPhaseString(p: unknown): boolean {
  return typeof p === 'string'
}
function phaseHas<K extends string>(p: FallbackState['phase'], key: K): boolean {
  if (typeof p !== 'object' || p == null) return false
  return key in (p as Record<string, unknown>)
}

describe('FallbackState phase', () => {
  it('handles unit variants as bare strings', () => {
    const s: FallbackState = {
      enabled: false,
      selected_model_id: null,
      phase: 'not_downloaded',
      model_path: null,
      llama_server_path: null,
      llama_server_port: null,
      last_error: null,
      last_started_unix_ms: null
    }
    expect(isPhaseString(s.phase)).toBe(true)
    expect(phaseHas(s.phase, 'ready')).toBe(false)
  })

  it('handles struct variants as objects', () => {
    const s: FallbackState = {
      enabled: true,
      selected_model_id: 'qwen2.5-1.5b-instruct-q4km',
      phase: { ready: { path: '/tmp/qwen.gguf', downloaded_at_unix_ms: 1234 } },
      model_path: '/tmp/qwen.gguf',
      llama_server_path: '/tmp/llama-server.exe',
      llama_server_port: 39135,
      last_error: null,
      last_started_unix_ms: 1234
    }
    expect(isPhaseString(s.phase)).toBe(false)
    expect(phaseHas(s.phase, 'ready')).toBe(true)
    expect(phaseHas(s.phase, 'not_downloaded')).toBe(false)
  })

  it('does not throw when reading a bare-string phase', () => {
    const p: FallbackState['phase'] = 'verifying'
    // The OLD code did `'verifying' in p` which threw a TypeError because
    // `p` is a string. The new code uses `phaseHas` which guards on
    // `typeof p === 'string'` first.
    expect(() => phaseHas(p, 'verifying')).not.toThrow()
  })

  it('does not throw when reading an object phase', () => {
    const p: FallbackState['phase'] = { error: { message: 'hash mismatch' } }
    expect(() => phaseHas(p, 'error')).not.toThrow()
    expect(phaseHas(p, 'error')).toBe(true)
  })
})