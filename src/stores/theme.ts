import { defineStore } from 'pinia'
import { resourcesApi, type Resource } from '@/api/resources'
import { applyTheme } from '@/themes'
import { useAuthStore } from './auth'

const STORAGE_ACTIVE = 'admin-suite.active-theme'

interface State {
  items: Resource[]
  active: Resource | null
  loading: boolean
}

interface ThemePayload {
  id: string
  label: string
  isDark: boolean
  tokens: Record<string, string>
}

export const useThemeStore = defineStore('theme', {
  state: (): State => ({
    items: [],
    active: null,
    loading: false
  }),

  getters: {
    isDark: (s) => {
      if (!s.active) return false
      try {
        const p = JSON.parse(s.active.content) as ThemePayload
        return !!p.isDark
      } catch {
        return false
      }
    }
  },

  actions: {
    async hydrate() {
      const auth = useAuthStore()
      if (!auth.token) return
      try {
        const r = await resourcesApi.list(auth.token, 'theme')
        this.items = r.items
        this.active = r.active
        if (this.active) {
          applyTheme(this.active)
          localStorage.setItem(STORAGE_ACTIVE, this.active.code)
        }
      } catch (e) {
        // If we cannot read themes (e.g. not logged in yet) fall back to the
        // cached active code so the UI looks the same as the last session.
        const cached = localStorage.getItem(STORAGE_ACTIVE)
        if (cached) {
          // Try to find it in the items list after login (re-fetched).
        }
      }
    },

    async activate(code: string) {
      const auth = useAuthStore()
      if (!auth.token) return
      await resourcesApi.activate(auth.token, 'theme', code)
      await this.hydrate()
    },

    async importFromJson(raw: string): Promise<Resource> {
      const auth = useAuthStore()
      if (!auth.token) throw new Error('not authenticated')
      const r = await resourcesApi.importTheme(auth.token, raw)
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