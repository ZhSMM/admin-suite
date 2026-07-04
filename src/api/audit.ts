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

export interface AuditFilter {
  action?: string
  actor_id?: string
  resource?: string
  payload_search?: string
  from?: string
  to?: string
}

export const auditApi = {
  list: (token: string, filter: AuditFilter = {}, page = 1, pageSize = 50) =>
    call<AuditListResult>('audit_list', {
      token,
      query: {
        action: filter.action || null,
        actor_id: filter.actor_id || null,
        resource: filter.resource || null,
        payload_search: filter.payload_search || null,
        from: filter.from || null,
        to: filter.to || null,
        page,
        page_size: pageSize
      }
    })
}