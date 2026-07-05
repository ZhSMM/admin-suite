import { call } from './index'

export interface LlmProvider {
  id: string
  code: string
  name: string
  kind: 'openai_compat' | 'anthropic' | 'google' | 'custom'
  base_url: string
  auth_type: 'bearer' | 'header' | 'none'
  auth_header: string | null
  /** write-only: never read back from the backend */
  api_key?: string | null
  settings_json: string
  default_model_id: string | null
  enabled: boolean
  sort_order: number
  created_at: string
  updated_at: string
}

export interface LlmProviderInput {
  code: string
  name: string
  kind: LlmProvider['kind']
  base_url: string
  auth_type: LlmProvider['auth_type']
  auth_header?: string | null
  api_key?: string | null
  settings_json?: string
  default_model_id?: string | null
  enabled?: boolean
  sort_order?: number
}

export interface LlmProviderUpdate {
  id: string
  name?: string
  base_url?: string
  auth_type?: LlmProvider['auth_type']
  auth_header?: string | null
  api_key?: string | null
  settings_json?: string
  default_model_id?: string | null
  enabled?: boolean
  sort_order?: number
}

export interface LlmModel {
  id: string
  provider_id: string
  code: string
  display_name: string
  capabilities: string
  context_window: number
  max_output: number
  pricing_json: string
  enabled: boolean
  sort_order: number
  created_at: string
  updated_at: string
}

export interface LlmModelInput {
  provider_id: string
  code: string
  display_name: string
  capabilities?: string
  context_window?: number
  max_output?: number
  pricing_json?: string
  enabled?: boolean
  sort_order?: number
}

export interface LlmModelUpdate {
  id: string
  display_name?: string
  capabilities?: string
  context_window?: number
  max_output?: number
  pricing_json?: string
  enabled?: boolean
  sort_order?: number
}

export interface ChatMessage {
  role: 'system' | 'user' | 'assistant'
  content: string
}

export interface ChatArgs {
  provider_id: string
  model_id: string
  messages: ChatMessage[]
  system?: string
  temperature?: number
  max_tokens?: number
}

export interface ChatResult {
  content: string
  prompt_tokens: number
  completion_tokens: number
  total_tokens: number
  model: string | null
  finish_reason: string | null
  request_id: string
}

export interface LlmUsageRow {
  id: string
  ts_unix_ms: number
  user_id: string
  provider_id: string
  model_id: string
  capability: string
  prompt_tokens: number
  completion_tokens: number
  total_tokens: number
  cost_usd: number
  latency_ms: number
  success: boolean
  error: string | null
  request_id: string
}

export interface FallbackState {
  enabled: boolean
  selected_model_id: string | null
  // The Rust side serializes `Phase` with `rename_all = "snake_case"`.
  // Unit variants (no fields) come through as bare strings:
  //   "not_downloaded" | "verifying"
  // Struct variants come through as objects with the variant name as key:
  //   { downloading: {...} } | { ready: {...} } | { error: {...} } |
  //   { hash_mismatch: {...} }
  // Helpers in LocalModelPanel.vue (`phaseIs` / `phaseHas`) handle both
  // shapes so the UI doesn't crash on either form.
  phase:
    | 'not_downloaded'
    | 'verifying'
    | { downloading: { bytes_done: number; total_bytes: number; speed_bps: number; eta_seconds: number } }
    | { ready: { path: string; downloaded_at_unix_ms: number } }
    | { error: { message: string } }
    | { hash_mismatch: { actual: string; expected: string } }
  model_path: string | null
  llama_server_path: string | null
  llama_server_port: number | null
  last_error: string | null
  last_started_unix_ms: number | null
}

export interface FallbackModelMirror {
  id: string
  display_name: string
  size_bytes: number
  min_ram_gb: number
  primary_url: string
}

export interface FallbackMirror {
  state: FallbackState
  models: FallbackModelMirror[]
}

export const llmApi = {
  // Providers
  listProviders: (token: string) => call<LlmProvider[]>('llm_providers_list', { token }),
  getProvider: (token: string, id: string) => call<LlmProvider>('llm_providers_get', { token, id }),
  createProvider: (token: string, payload: LlmProviderInput) =>
    call<LlmProvider>('llm_providers_create', { token, payload }),
  updateProvider: (token: string, payload: LlmProviderUpdate) =>
    call<LlmProvider>('llm_providers_update', { token, payload }),
  deleteProvider: (token: string, id: string) =>
    call<void>('llm_providers_delete', { token, id }),

  // Models
  listModels: (token: string, provider_id?: string) =>
    call<LlmModel[]>('llm_models_list', { token, providerId: provider_id ?? null }),
  getModel: (token: string, id: string) => call<LlmModel>('llm_models_get', { token, id }),
  createModel: (token: string, payload: LlmModelInput) =>
    call<LlmModel>('llm_models_create', { token, payload }),
  updateModel: (token: string, payload: LlmModelUpdate) =>
    call<LlmModel>('llm_models_update', { token, payload }),
  deleteModel: (token: string, id: string) =>
    call<void>('llm_models_delete', { token, id }),

  // Chat
  chat: (token: string, args: ChatArgs) => call<ChatResult>('llm_chat', { token, args }),
  chatStream: (token: string, args: ChatArgs) =>
    call<ChatResult>('llm_chat_stream', { token, args }),

  // Usage
  queryUsage: (
    token: string,
    opts: { from?: number; to?: number; user_id?: string; provider_id?: string; limit?: number } = {}
  ) =>
    call<LlmUsageRow[]>('llm_usage_query', {
      token,
      from_unix_ms: opts.from ?? null,
      to_unix_ms: opts.to ?? null,
      user_id: opts.user_id ?? null,
      provider_id: opts.provider_id ?? null,
      limit: opts.limit ?? 200
    }),

  // Fallback (v0.6.0)
  fallbackStatus: () => call<FallbackMirror>('llm_fallback_status'),
  fallbackSelectModel: (token: string, model_id: string) =>
    call<void>('llm_fallback_select_model', { token, modelId: model_id }),
  fallbackSetEnabled: (token: string, enabled: boolean) =>
    call<void>('llm_fallback_set_enabled', { token, enabled }),
  fallbackDismissStartupPrompt: (token: string) =>
    call<void>('llm_fallback_dismiss_startup_prompt', { token }),
  fallbackStartupPromptNeeded: (token: string) =>
    call<boolean>('llm_fallback_startup_prompt_needed', { token }),

  // Fallback (v0.6.2 — one-click local install)
  fallbackInstallStart: (token: string, model_id: string) =>
    call<{ model_id: string; model_size_bytes: number; server_size_bytes: number; already_installed: boolean }>(
      'llm_fallback_install_start',
      { token, modelId: model_id }
    ),
  fallbackInstallCancel: (token: string) =>
    call<boolean>('llm_fallback_install_cancel', { token }),
  fallbackServerStart: (token: string) =>
    call<number>('llm_fallback_server_start', { token }),
  fallbackServerStop: (token: string) =>
    call<void>('llm_fallback_server_stop', { token }),
  fallbackRemove: (token: string) =>
    call<void>('llm_fallback_remove', { token }),
  fallbackDiskFree: () => call<number>('llm_fallback_disk_free')
}