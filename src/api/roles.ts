import { call } from './index'

export interface Role {
  id: string
  code: string
  name: string
  description: string | null
  status: string
  built_in: boolean
  sort_order: number
  created_at: string
  updated_at: string
  permission_codes: string[]
}

export interface RoleCreate {
  code: string
  name: string
  description?: string | null
  status?: string
  sort_order?: number
  permission_ids: string[]
}

export interface RoleUpdate {
  id: string
  name?: string
  description?: string | null
  status?: string
  sort_order?: number
  permission_ids?: string[]
}

export const rolesApi = {
  list: (token: string) => call<Role[]>('roles_list', { token }),
  get: (token: string, id: string) => call<Role>('roles_get', { token, id }),
  create: (token: string, payload: RoleCreate) => call<Role>('roles_create', { token, payload }),
  update: (token: string, payload: RoleUpdate) => call<Role>('roles_update', { token, payload }),
  remove: (token: string, id: string) => call<void>('roles_delete', { token, id }),
  assignMenus: (token: string, role_id: string, menu_ids: string[]) =>
    call<void>('roles_assign_menus', { token, role_id, menu_ids }),
  getMenus: (token: string, role_id: string) =>
    call<string[]>('roles_get_menus', { token, role_id })
}

export interface Permission {
  id: string
  code: string
  name: string
  resource: string
  action: string
  description: string | null
  created_at: string
}

export const permissionsApi = {
  list: (token: string) => call<Permission[]>('permissions_list', { token })
}

export interface Menu {
  id: string
  parent_id: string | null
  code: string
  title: string
  path: string | null
  icon: string | null
  component: string | null
  sort_order: number
  visible: boolean
  status: string
  menu_type: string
  permission_code: string | null
  created_at: string
  updated_at: string
}

export interface MenuNode {
  id: string
  parent_id: string | null
  code: string
  title: string
  path: string | null
  icon: string | null
  component: string | null
  sort_order: number
  visible: boolean
  status: string
  menu_type: string
  permission_code: string | null
  created_at: string
  updated_at: string
  children: MenuNode[]
}

export interface MenuCreate {
  parent_id?: string | null
  code: string
  title: string
  path?: string | null
  icon?: string | null
  component?: string | null
  sort_order?: number
  visible?: boolean
  status?: string
  menu_type?: string
  permission_code?: string | null
}

export interface MenuUpdate {
  id: string
  title?: string
  path?: string | null
  icon?: string | null
  component?: string | null
  sort_order?: number
  visible?: boolean
  status?: string
  menu_type?: string
  permission_code?: string | null
  parent_id?: string | null
}

export const menusApi = {
  tree: (token: string) => call<MenuNode[]>('menus_tree', { token }),
  create: (token: string, payload: MenuCreate) => call<Menu>('menus_create', { token, payload }),
  update: (token: string, payload: MenuUpdate) => call<Menu>('menus_update', { token, payload }),
  remove: (token: string, id: string) => call<void>('menus_delete', { token, id })
}