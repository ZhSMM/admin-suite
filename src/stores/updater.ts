import { defineStore } from 'pinia'
import { updaterApi, type UpdateManifest } from '@/api/updater'

type Status = 'idle' | 'checking' | 'available' | 'uptodate' | 'downloading' | 'ready' | 'error' | 'disabled'

interface State {
  status: Status
  manifest: UpdateManifest | null
  error: string | null
  lastChecked: number
}

export const useUpdaterStore = defineStore('updater', {
  state: (): State => ({
    status: 'idle',
    manifest: null,
    error: null,
    lastChecked: 0
  }),
  actions: {
    async check(token: string) {
      this.status = 'checking'
      this.error = null
      try {
        this.manifest = await updaterApi.check(token)
        this.lastChecked = Date.now()
        this.status = this.manifest.available ? 'available' : 'uptodate'
      } catch (e) {
        this.status = 'error'
        this.error = e instanceof Error ? e.message : String(e)
      }
    },
    async install(token: string) {
      if (!this.manifest?.available) return
      this.status = 'downloading'
      this.error = null
      try {
        await updaterApi.install(token)
        this.status = 'ready'
      } catch (e) {
        this.status = 'error'
        this.error = e instanceof Error ? e.message : String(e)
      }
    },
    /** When the user wants to "snooze" the banner without acting on it. */
    dismiss() {
      this.status = 'idle'
    }
  }
})