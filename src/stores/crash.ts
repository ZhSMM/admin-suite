import { defineStore } from 'pinia'
import { crashApi, type CrashReport } from '@/api/crash'

interface State {
  items: CrashReport[]
  loading: boolean
  /** Last time `crash_log` was called from this session. Used by the global
   * error handler to debounce noisy errors (e.g. the same ResizeObserver
   * loop limit error firing on every animation frame). */
  lastLogAt: number
}

export const useCrashStore = defineStore('crash', {
  state: (): State => ({ items: [], loading: false, lastLogAt: 0 }),
  actions: {
    async refresh(token: string) {
      this.loading = true
      try {
        this.items = await crashApi.list(token)
      } finally {
        this.loading = false
      }
    },
    async clear(token: string) {
      await crashApi.clear(token)
      this.items = []
    },
    /** Best-effort frontend log — never throws. */
    async log(input: {
      kind: 'frontend_error' | 'frontend_unhandled_rejection'
      message: string
      source?: string
      detail?: string
    }) {
      // Debounce: if we logged within the last 1s, drop the new one. This
      // protects against errors that fire in a tight loop (ResizeObserver,
      // infinite render cycles, etc.).
      const now = Date.now()
      if (now - this.lastLogAt < 1000) return
      this.lastLogAt = now
      try {
        await crashApi.log({
          kind: input.kind,
          message: input.message,
          source: input.source ?? null,
          app_version: __APP_VERSION__ ?? null,
          detail: input.detail ?? null
        })
      } catch {
        // best-effort
      }
    }
  }
})