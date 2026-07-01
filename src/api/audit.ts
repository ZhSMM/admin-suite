import { call } from './index'

export interface AuditEntry {
  id: string
  actor_id: string | null
  actor_name: string | null
  action: string
  resource: string | null
  target_id: string | null
  payload: string | null
  ip: string | null
  created_at: string
}

export interface AuditListResult {
  items: AuditEntry[]
  total: number
  page: number
  page_size: number
}

export const auditApi = {
  list: (token: string, action?: string, page = 1, page_size = 50) =>
    call<AuditListResult>('audit_list', {
      token,
      query: { action: action || null, actor_id: null, page, page_size }
    })
}