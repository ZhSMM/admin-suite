import { call } from './index'

export interface IpcMetric {
  command: string
  count: number
  total_ms: number
  last_ms: number
  max_ms: number
  avg_ms: number
  error_count: number
  last_error: string | null
  last_ts: number
  history_ms: number[]
}

export const metricsApi = {
  snapshot: () => call<IpcMetric[]>('metrics_snapshot'),
  clear: () => call<void>('metrics_clear')
}