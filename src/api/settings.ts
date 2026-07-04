import { invoke } from '@tauri-apps/api/tauri'

export interface Setting {
  key: string
  value: string
  updated_at: string
}

export interface SettingUpdate {
  key: string
  value: string
}

export const settingsApi = {
  list: (token: string) => invoke<Setting[]>('settings_list', { token }),
  set: (token: string, updates: SettingUpdate[]) =>
    invoke<Setting[]>('settings_set', { token, updates })
}