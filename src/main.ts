import { createApp } from 'vue'
import { createPinia } from 'pinia'
import ElementPlus from 'element-plus'
import * as ElementPlusIconsVue from '@element-plus/icons-vue'
import 'element-plus/dist/index.css'

import App from './App.vue'
import { router } from './router'
import { i18n } from './i18n'
import { useThemeStore } from './stores/theme'
import { useLocaleStore } from './stores/locale'
import { useCrashStore } from './stores/crash'
import './styles/index.scss'

async function bootstrap() {
  const app = createApp(App)

  // Register every Element Plus icon globally.
  for (const [key, comp] of Object.entries(ElementPlusIconsVue)) {
    app.component(key, comp as any)
  }

  // Pinia must be installed before anything that touches a store.  We also
  // wire the global Vue error handler here so any uncaught error inside a
  // component render / lifecycle hook is captured to the crash store before
  // the user even sees it.
  const pinia = createPinia()
  app.use(pinia)
  app.use(router)
  app.use(ElementPlus)
  app.use(i18n)

  const crashStore = useCrashStore()

  app.config.errorHandler = (err, instance, info) => {
    // Always log to dev console first — easier debugging.
    // eslint-disable-next-line no-console
    console.error('[vue:error]', err, info)
    const message = err instanceof Error ? `${err.name}: ${err.message}` : String(err)
    const stack = err instanceof Error ? (err.stack ?? '') : ''
    const source = instance?.$options?.name
      ? `Component: ${String(instance.$options.name)} (${info})`
      : `Global: ${info}`
    void crashStore.log({
      kind: 'frontend_error',
      message,
      source,
      detail: stack
    })
  }

  // window.onerror + onunhandledrejection for errors that escape Vue
  // entirely (async callbacks, third-party scripts).
  window.addEventListener('error', (e) => {
    void crashStore.log({
      kind: 'frontend_error',
      message: e.message || 'unknown error',
      source: e.filename ? `${e.filename}:${e.lineno}:${e.colno}` : 'window.error',
      detail: e.error instanceof Error ? (e.error.stack ?? '') : ''
    })
  })
  window.addEventListener('unhandledrejection', (e) => {
    const reason = e.reason
    const message =
      reason instanceof Error ? `${reason.name}: ${reason.message}` : String(reason)
    void crashStore.log({
      kind: 'frontend_unhandled_rejection',
      message,
      source: 'window.unhandledrejection',
      detail: reason instanceof Error ? (reason.stack ?? '') : ''
    })
  })

  // Apply persisted theme + locale before mounting so the first paint is correct.
  const themeStore = useThemeStore()
  const localeStore = useLocaleStore()
  await themeStore.hydrate()
  await localeStore.hydrate()

  app.mount('#app')
}

bootstrap().catch((e) => {
  // eslint-disable-next-line no-console
  console.error('bootstrap failed', e)
})