import { call } from './index'

export interface UserSafe {
  id: string
  username: string
  display_name: string
  email: string | null
  phone: string | null
  avatar: string | null
  status: string
  is_super_admin: boolean
  created_at: string
  updated_at: string
  last_login_at: string | null
  role_ids: string[]
  role_codes: string[]
}

export interface UserListQuery {
  keyword?: string
  status?: string
  role_id?: string
  page?: number
  page_size?: number
}

export interface UserListResult {
  items: UserSafe[]
  total: number
  page: number
  page_size: number
}

export interface UserCreate {
  username: string
  display_name: string
  password: string
  email?: string | null
  phone?: string | null
  avatar?: string | null
  status?: string
  role_ids: string[]
}

export interface UserUpdate {
  id: string
  display_name?: string
  email?: string | null
  phone?: string | null
  avatar?: string | null
  status?: string
  password?: string
  role_ids?: string[]
}

export const usersApi = {
  list: (token: string, query: UserListQuery = {}) =>
    call<UserListResult>('users_list', { token, query }),
  get: (token: string, id: string) =>
    call<UserSafe>('users_get', { token, id }),
  create: (token: string, payload: UserCreate) =>
    call<UserSafe>('users_create', { token, payload }),
  update: (token: string, payload: UserUpdate) =>
    call<UserSafe>('users_update', { token, payload }),
  remove: (token: string, id: string) =>
    call<void>('users_delete', { token, id })
}