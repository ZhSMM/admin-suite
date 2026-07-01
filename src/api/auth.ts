import { call } from './index'
import type { Menu } from './roles'
import type { UserSafe } from './users'

export interface LoginResult {
  token: string
  user: UserSafe
  permissions: string[]
  menus: Menu[]
  expires_at: string
}

export const authApi = {
  login: (username: string, password: string) =>
    call<LoginResult>('auth_login', { username, password }),
  logout: (token: string) => call<void>('auth_logout', { token }),
  me: (token: string) => call<UserSafe>('auth_me', { token })
}

export interface AppInfo {
  data_dir: string
  db_path: string
  migrations_dir: string
  default_admin: { username: string; password: string; note: string }
}

export const appApi = {
  info: () => call<AppInfo>('app_info')
}