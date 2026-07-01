import { defineStore } from 'pinia'
import { authApi, type LoginResult } from '@/api/auth'
import type { UserSafe } from '@/api/users'
import { ApiException } from '@/api'

const STORAGE_TOKEN = 'admin-suite.token'
const STORAGE_USER = 'admin-suite.user'

interface State {
  token: string
  user: UserSafe | null
  permissions: string[]
  expiresAt: string | null
  loading: boolean
}

export const useAuthStore = defineStore('auth', {
  state: (): State => ({
    token: '',
    user: null,
    permissions: [],
    expiresAt: null,
    loading: false
  }),

  getters: {
    isAuthenticated: (s) => !!s.token && !!s.user,
    isSuperAdmin: (s) => !!s.user?.is_super_admin,
    hasPermission: (s) => (code: string) => {
      if (s.user?.is_super_admin) return true
      if (s.permissions.includes('*:*')) return true
      if (s.permissions.includes(code)) return true
      const [res] = code.split(':')
      return s.permissions.includes(`${res}:*`)
    }
  },

  actions: {
    async login(username: string, password: string) {
      this.loading = true
      try {
        const r: LoginResult = await authApi.login(username, password)
        this.token = r.token
        this.user = r.user
        this.permissions = r.permissions
        this.expiresAt = r.expires_at
        localStorage.setItem(STORAGE_TOKEN, r.token)
        localStorage.setItem(STORAGE_USER, JSON.stringify(r.user))
        return r
      } finally {
        this.loading = false
      }
    },

    async logout() {
      try {
        if (this.token) await authApi.logout(this.token)
      } catch (e) {
        // best-effort
      }
      this.token = ''
      this.user = null
      this.permissions = []
      this.expiresAt = null
      localStorage.removeItem(STORAGE_TOKEN)
      localStorage.removeItem(STORAGE_USER)
    },

    /** Restore from localStorage on app start. */
    async restore() {
      const token = localStorage.getItem(STORAGE_TOKEN)
      const userRaw = localStorage.getItem(STORAGE_USER)
      if (!token || !userRaw) return
      try {
        const user = JSON.parse(userRaw) as UserSafe
        this.token = token
        this.user = user
        // Refresh the user object from the backend so updates take effect.
        try {
          const fresh = await authApi.me(token)
          this.user = fresh
          localStorage.setItem(STORAGE_USER, JSON.stringify(fresh))
        } catch (e) {
          if (e instanceof ApiException && e.code === 'UNAUTHORIZED') {
            await this.logout()
          }
        }
      } catch {
        await this.logout()
      }
    },

    refreshPermissions(perms: string[]) {
      this.permissions = perms
    }
  }
})