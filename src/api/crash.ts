import { call } from './index'

export type CrashKind = 'rust_panic' | 'frontend_error' | 'frontend_unhandled_rejection'

export interface CrashReport {
  id: string
  ts_unix_ms: number
  kind: CrashKind
  message: string
  source: string | null
  app_version: string | null
  detail: string | null
}

export interface CrashReportInput {
  kind: CrashKind
  message: string
  source?: string | null
  app_version?: string | null
  detail?: string | null
}

export const crashApi = {
  log: (input: CrashReportInput) =>
    call<CrashReport>('crash_log', { input }),
  list: (token: string) => call<CrashReport[]>('crash_list', { token }),
  clear: (token: string) => call<number>('crash_clear', { token })
}