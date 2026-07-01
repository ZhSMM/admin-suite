import { defineStore } from 'pinia'
import { resourcesApi, type Resource } from '@/api/resources'
import { useAuthStore } from './auth'
import { useLocale } from '@/i18n'

const STORAGE_ACTIVE = 'admin-suite.active-locale'

interface State {
  items: Resource[]
  active: Resource | null
  loading: boolean
}

interface LocalePayload {
  id: string
  label: string
  messages: Record<string, string>
}

export const useLocaleStore = defineStore('locale', {
  state: (): State => ({
    items: [],
    active: null,
    loading: false
  }),

  actions: {
    async hydrate() {
      const auth = useAuthStore()
      if (!auth.token) {
        // No auth yet (login page). Use cached locale so the page renders.
        const cached = localStorage.getItem(STORAGE_ACTIVE) || 'en-US'
        const { setLocale } = useLocale()
        setLocale(cached)
        return
      }
      try {
        const r = await resourcesApi.list(auth.token, 'locale')
        this.items = r.items
        this.active = r.active
        if (this.active) {
          this.apply(this.active)
          localStorage.setItem(STORAGE_ACTIVE, this.active.code)
        }
      } catch (e) {
        // ignore; keep last locale
      }
    },

    apply(resource: Resource) {
      try {
        const p = JSON.parse(resource.content) as LocalePayload
        const { setLocale, mergeMessages } = useLocale()
        mergeMessages(p.messages || {})
        setLocale(resource.code)
      } catch (e) {
        console.warn('failed to apply locale', resource.code, e)
      }
    },

    async activate(code: string) {
      const auth = useAuthStore()
      if (!auth.token) return
      await resourcesApi.activate(auth.token, 'locale', code)
      await this.hydrate()
    },

    async importFromJson(raw: string): Promise<Resource> {
      const auth = useAuthStore()
      if (!auth.token) throw new Error('not authenticated')
      const r = await resourcesApi.importLocale(auth.token, raw)
      await this.hydrate()
      return r
    },

    async remove(id: string) {
      const auth = useAuthStore()
      if (!auth.token) return
      await resourcesApi.remove(auth.token, id)
      await this.hydrate()
    }
  }
})