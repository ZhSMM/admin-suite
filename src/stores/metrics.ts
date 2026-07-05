import { defineStore } from 'pinia'
import { metricsApi, type IpcMetric } from '@/api/metrics'

interface State {
  items: IpcMetric[]
  loading: boolean
  lastLoaded: number
  autoRefresh: boolean
}

export const useMetricsStore = defineStore('metrics', {
  state: (): State => ({
    items: [],
    loading: false,
    lastLoaded: 0,
    autoRefresh: false
  }),
  actions: {
    async refresh() {
      this.loading = true
      try {
        this.items = await metricsApi.snapshot()
        this.lastLoaded = Date.now()
      } finally {
        this.loading = false
      }
    },
    async clear() {
      await metricsApi.clear()
      this.items = []
    }
  }
})