import { createI18n } from 'vue-i18n'
import en_US from './locales/en-US'
import zh_CN from './locales/zh-CN'

/**
 * The vue-i18n instance is created with the bundled locales only — DB-loaded
 * locales are merged in via `mergeMessages` once the user logs in.
 */
export const SUPPORTED_LOCALES = ['en-US', 'zh-CN']

const messages: Record<string, Record<string, string>> = {
  'en-US': { ...en_US },
  'zh-CN': { ...zh_CN }
}

export const i18n = createI18n({
  legacy: false,
  locale: localStorage.getItem('admin-suite.active-locale') || 'en-US',
  fallbackLocale: 'en-US',
  messages
})

/**
 * Adds (and overrides) messages on top of the bundled locale.
 * Called by the locale store when the user activates a custom locale resource.
 */
export function useLocale() {
  function mergeMessages(messages: Record<string, string>) {
    const cur = (i18n.global.messages.value[i18n.global.locale.value] as Record<string, string>) || {}
    i18n.global.setLocaleMessage(i18n.global.locale.value, { ...cur, ...messages })
  }
  function setLocale(code: string) {
    if (!SUPPORTED_LOCALES.includes(code)) {
      // Always pre-seed with en-US so untranslated keys resolve to English.
      // We do this even for already-registered custom locales — vue-i18n
      // keeps the message set across hot reloads and we want the merge in
      // the following apply() to be authoritative.  Existing keys are
      // preserved by spreading `existing` after `enUS`.
      const enUS = (i18n.global.messages.value['en-US'] as Record<string, string>) || {}
      const existing = (i18n.global.messages.value[code] as Record<string, string>) || {}
      i18n.global.setLocaleMessage(code, { ...enUS, ...existing })
    }
    i18n.global.locale.value = code
  }
  return { mergeMessages, setLocale, t: i18n.global.t.bind(i18n.global) }
}