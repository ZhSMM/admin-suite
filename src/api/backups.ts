import { invoke } from '@tauri-apps/api/tauri'

export interface BackupInfo {
  name: string
  path: string
  size_bytes: number
  created_at: string
}

export interface RestoreRequest {
  backup_name: string
  backup_path: string
  restart_required: boolean
}

export const backupsApi = {
  list: (token: string) => invoke<BackupInfo[]>('backup_list', { token }),
  create: (token: string) => invoke<BackupInfo>('backup_create', { token }),
  delete: (token: string, name: string) => invoke<void>('backup_delete', { token, name }),
  restore: (token: string, name: string) =>
    invoke<RestoreRequest>('backup_restore', { token, name })
}

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`
}