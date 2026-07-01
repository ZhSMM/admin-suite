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
      // Set a non-bundled locale by registering it as an alias of fallback first,
      // then mergeMessages fills the keys.
      if (!i18n.global.availableLocales.includes(code)) {
        i18n.global.setLocaleMessage(code, { ...(i18n.global.messages.value['en-US'] as any) })
      }
    }
    i18n.global.locale.value = code
  }
  return { mergeMessages, setLocale, t: i18n.global.t.bind(i18n.global) }
}