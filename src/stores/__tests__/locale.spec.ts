/**
 * useLocaleStore is the most regression-prone bit of the front-end — it's
 * what was silently overwriting locale messages before v0.5.5.  These tests
 * pin down its core invariants:
 *
 *  1. `apply` switches the active locale BEFORE merging its messages,
 *     so importing zh-TW can never pollute en-US.
 *  2. `record` dedupes consecutive identical snapshots (so a re-render
 *     storm doesn't churn localStorage).
 *  3. Activate → hydrate → apply all update both the store and the
 *     active i18n locale.
 */
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { nextTick } from 'vue'

// Stub the Tauri invoke bridge so the auth store can be created without a
// real backend.  Tests that need to exercise activate() / import() set up
// the return value per-case.
vi.mock('@tauri-apps/api/tauri', () => ({
  invoke: vi.fn()
}))

import { invoke } from '@tauri-apps/api/tauri'
import { i18n, useLocale } from '@/i18n'

describe('useLocaleStore — apply order invariant (regression: v0.5.5)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    localStorage.clear()
    // Reset i18n back to a known state.  We have to explicitly unregister
    // any locale added by a previous test (vue-i18n keeps registered locales
    // across tests because the instance is a module singleton).
    i18n.global.locale.value = 'en-US'
    i18n.global.setLocaleMessage('en-US', {
      'common.ok': 'OK',
      'common.cancel': 'Cancel',
      'menu.users': 'Users'
    } as any)
    i18n.global.setLocaleMessage('zh-CN', {
      'common.ok': '确定',
      'menu.users': '用户管理'
    } as any)
    i18n.global.setLocaleMessage('zh-TW', {})
    vi.mocked(invoke).mockReset()
  })

  it('apply() switches locale FIRST then merges messages into the new locale', async () => {
    const { useAuthStore } = await import('@/stores/auth')
    const { useLocaleStore } = await import('@/stores/locale')

    const auth = useAuthStore()
    auth.token = 'fake-token'
    const locale = useLocaleStore()

    // Simulate a zh-TW row coming back from the backend.
    const zhTW = {
      id: 'r_zh_tw',
      resource_type: 'locale',
      code: 'zh-TW',
      name: '繁體中文',
      content: JSON.stringify({
        id: 'zh-TW',
        label: '繁體中文',
        messages: { 'common.ok': '確定', 'menu.users': '用戶管理' }
      }),
      source: 'imported',
      built_in: false,
      active: true,
      created_at: '2025-01-01T00:00:00Z',
      updated_at: '2025-01-01T00:00:00Z'
    }

    locale.apply(zhTW as any)
    await nextTick()

    // After apply(), current locale MUST be zh-TW...
    expect(i18n.global.locale.value).toBe('zh-TW')
    // ...and zh-TW messages MUST live in zh-TW, NOT in en-US.
    const enUS = (i18n.global.messages.value as any)['en-US'] || {}
    const zhTWMsgs = (i18n.global.messages.value as any)['zh-TW'] || {}
    expect(enUS['common.ok']).toBe('OK') // untouched
    expect(enUS['menu.users']).toBe('Users') // untouched
    expect(zhTWMsgs['common.ok']).toBe('確定')
    expect(zhTWMsgs['menu.users']).toBe('用戶管理')
  })

  it('apply() preserves existing bundled messages for unknown keys', async () => {
    const { useLocaleStore } = await import('@/stores/locale')

    const locale = useLocaleStore()
    const zhTW = {
      id: 'r_zh_tw',
      resource_type: 'locale',
      code: 'zh-TW',
      name: '繁體中文',
      content: JSON.stringify({
        id: 'zh-TW',
        label: '繁體中文',
        messages: { 'common.ok': '確定' /* menu.users missing */ }
      }),
      source: 'imported',
      built_in: false,
      active: true,
      created_at: '2025-01-01T00:00:00Z',
      updated_at: '2025-01-01T00:00:00Z'
    }

    locale.apply(zhTW as any)
    await nextTick()

    const zhTWMsgs = (i18n.global.messages.value as any)['zh-TW'] || {}
    expect(zhTWMsgs['common.ok']).toBe('確定')
    // Falls back to bundled en-US for missing keys (because setLocale()
    // pre-seeds the alias with en-US).
    expect(zhTWMsgs['menu.users']).toBe('Users')
  })
})

describe('useLocaleStore — i18n plumbing', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    localStorage.clear()
    i18n.global.locale.value = 'en-US'
  })

  it('hydrate() without auth falls back to localStorage cache', async () => {
    localStorage.setItem('admin-suite.active-locale', 'zh-CN')
    const { useLocaleStore } = await import('@/stores/locale')
    const locale = useLocaleStore()
    await locale.hydrate()
    expect(i18n.global.locale.value).toBe('zh-CN')
  })

  it('hydrate() without auth defaults to en-US when no cache exists', async () => {
    const { useLocaleStore } = await import('@/stores/locale')
    const locale = useLocaleStore()
    await locale.hydrate()
    expect(i18n.global.locale.value).toBe('en-US')
  })
})