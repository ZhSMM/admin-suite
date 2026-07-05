import { call } from './index'

export interface UpdateManifest {
  available: boolean
  current_version: string
  latest_version: string | null
  date: string | null
  body: string | null
  mandatory: boolean
}

export const updaterApi = {
  check: (token: string) => call<UpdateManifest>('updater_check', { token }),
  install: (token: string) => call<void>('updater_install', { token })
}