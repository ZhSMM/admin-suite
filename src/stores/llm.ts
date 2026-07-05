import { defineStore } from 'pinia'
import { llmApi, type ChatMessage, type LlmModel, type LlmProvider } from '@/api/llm'

interface State {
  providers: LlmProvider[]
  models: LlmModel[]
  loading: boolean
  error: string | null
  // Default selections (per-user) cached in localStorage so they survive reloads
  defaultProviderId: string | null
  defaultModelId: string | null
}

export const useLlmStore = defineStore('llm', {
  state: (): State => ({
    providers: [],
    models: [],
    loading: false,
    error: null,
    defaultProviderId: localStorage.getItem('llm.defaultProviderId'),
    defaultModelId: localStorage.getItem('llm.defaultModelId')
  }),
  getters: {
    enabledProviders: (s) => s.providers.filter((p) => p.enabled),
    modelsFor: (s) => (providerId: string) =>
      s.models.filter((m) => m.provider_id === providerId && m.enabled)
  },
  actions: {
    async loadAll(token: string) {
      this.loading = true
      this.error = null
      try {
        const [providers, models] = await Promise.all([
          llmApi.listProviders(token),
          llmApi.listModels(token)
        ])
        this.providers = providers
        this.models = models
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