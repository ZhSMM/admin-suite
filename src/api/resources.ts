import { call } from './index'

export interface Resource {
  id: string
  resource_type: 'theme' | 'locale'
  code: string
  name: string
  content: string
  source: string
  built_in: boolean
  active: boolean
  created_at: string
  updated_at: string
}

export interface ResourceListResponse {
  items: Resource[]
  active: Resource | null
}

export const resourcesApi = {
  list: (token: string, resource_type: 'theme' | 'locale') =>
    call<ResourceListResponse>('resources_list', { token, resourceType: resource_type }),

  importTheme: (token: string, raw_json: string) =>
    call<Resource>('resources_import_theme', { token, rawJson: raw_json }),

  importLocale: (token: string, raw_json: string) =>
    call<Resource>('resources_import_locale', { token, rawJson: raw_json }),

  activate: (token: string, resource_type: 'theme' | 'locale', code: string) =>
    call<void>('resources_activate', { token, resourceType: resource_type, code }),

  remove: (token: string, id: string) => call<void>('resources_delete', { token, id })
}