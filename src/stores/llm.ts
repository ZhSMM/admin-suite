import { defineStore } from 'pinia'
import { llmApi, type ChatMessage, type LlmModel, type LlmProvider } from '@/api/llm'
import { settingsApi } from '@/api/settings'

interface State {
  providers: LlmProvider[]
  models: LlmModel[]
  loading: boolean
  error: string | null
  // Per-user overrides cached in localStorage so they survive reloads.
  defaultProviderId: string | null
  defaultModelId: string | null
  // Global defaults read from `app_state` (Settings → AI).  Used as fallback
  // when the per-user override is empty.
  globalDefaultProviderId: string | null
  globalDefaultModelId: string | null
  // When true, prefer the offline fallback provider (when ready).
  localFirst: boolean
  // True after we've hydrated from server-side public settings — prevents
  // loadAll() from clobbering a freshly-saved override with a stale empty value.
  publicSettingsLoaded: boolean
}

export const useLlmStore = defineStore('llm', {
  state: (): State => ({
    providers: [],
    models: [],
    loading: false,
    error: null,
    defaultProviderId: localStorage.getItem('llm.defaultProviderId'),
    defaultModelId: localStorage.getItem('llm.defaultModelId'),
    globalDefaultProviderId: null,
    globalDefaultModelId: null,
    localFirst: false,
    publicSettingsLoaded: false
  }),
  getters: {
    enabledProviders: (s) => s.providers.filter((p) => p.enabled),
    modelsFor: (s) => (providerId: string) =>
      s.models.filter((m) => m.provider_id === providerId && m.enabled),
    /**
     * Effective default provider — per-user override wins, otherwise fall
     * back to the global default from Settings → AI.
     */
    effectiveProviderId(state): string | null {
      return state.defaultProviderId || state.globalDefaultProviderId
    },
    effectiveModelId(state): string | null {
      return state.defaultModelId || state.globalDefaultModelId
    }
  },
  actions: {
    async loadAll(token: string) {
      this.loading = true
      this.error = null
      try {
        const [providers, models, publicSettings] = await Promise.all([
          llmApi.listProviders(token),
          llmApi.listModels(token),
          // settingsApi.listPublic is allowlisted server-side and only
          // returns non-secret keys, so this is safe to call on every load.
          settingsApi.listPublic(token).catch(() => [] as Awaited<ReturnType<typeof settingsApi.listPublic>>)
        ])
        this.providers = providers
        this.models = models
        for (const s of publicSettings) {
          if (s.key === 'ai.default_chat_provider') {
            this.globalDefaultProviderId = s.value || null
          } else if (s.key === 'ai.default_chat_model') {
            this.globalDefaultModelId = s.value || null
          } else if (s.key === 'ai.local_first') {
            this.localFirst = s.value === 'true'
          }
        }
        this.publicSettingsLoaded = true
      } catch (e) {
        this.error = e instanceof Error ? e.message : String(e)
      } finally {
        this.loading = false
      }
    },
    setDefault(providerId: string, modelId: string) {
      this.defaultProviderId = providerId
      this.defaultModelId = modelId
      localStorage.setItem('llm.defaultProviderId', providerId)
      localStorage.setItem('llm.defaultModelId', modelId)
    },
    clearLocalDefault() {
      this.defaultProviderId = null
      this.defaultModelId = null
      localStorage.removeItem('llm.defaultProviderId')
      localStorage.removeItem('llm.defaultModelId')
    },
    async sendChat(token: string, args: {
      provider_id: string
      model_id: string
      messages: ChatMessage[]
      system?: string
      temperature?: number
      max_tokens?: number
    }) {
      return await llmApi.chat(token, args)
    }
  }
})