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
import './styles/index.scss'

async function bootstrap() {
  const app = createApp(App)

  // Register every Element Plus icon globally.
  for (const [key, comp] of Object.entries(ElementPlusIconsVue)) {
    app.component(key, comp as any)
  }

  app.use(createPinia())
  app.use(router)
  app.use(ElementPlus)
  app.use(i18n)

  // Apply persisted theme + locale before mounting so the first paint is correct.
  const themeStore = useThemeStore()
  const localeStore = useLocaleStore()
  await themeStore.hydrate()
  await localeStore.hydrate()

  app.mount('#app')
}

bootstrap().catch((e) => {
  console.error('bootstrap failed', e)
})