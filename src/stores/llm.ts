import { defineStore } from 'pinia'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { ElMessage } from 'element-plus'
import { llmApi, type ChatMessage, type LlmModel, type LlmProvider, type FallbackMirror, type FallbackState, type SpeedTestResult, type RemoteModelInfo } from '@/api/llm'
import { settingsApi } from '@/api/settings'

export interface FallbackProgress {
  stage: 'model' | 'server'
  bytesDone: number
  totalBytes: number
  speedBps: number
  etaSeconds: number
  currentStage: 'model' | 'server'
  modelId: string
}

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
  // ---- v0.6.2 local install ----
  fallbackState: FallbackState | null
  fallbackModels: FallbackMirror['models']
  installInFlight: boolean
  installProgress: FallbackProgress | null
  /** "all" | "server" | "model" — which stage just finished emitting the
   *  most recent tick. Used to label the progress bar. */
  installCurrentStage: 'server' | 'model' | null
  installError: string | null
  fallbackEventUnlisteners: UnlistenFn[]
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
    publicSettingsLoaded: false,
    fallbackState: null,
    fallbackModels: [],
    installInFlight: false,
    installProgress: null,
    installCurrentStage: null,
    installError: null,
    fallbackEventUnlisteners: []
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
    },
    /** Has the user accepted the license disclaimer? Stored locally so we
     * only pester once per machine. */
    disclaimerAccepted(): boolean {
      return localStorage.getItem('llm.fallback.disclaimer_accepted_v1') === 'true'
    }
  },
  actions: {
    async loadAll(token: string) {
      this.loading = true
      this.error = null
      try {
        const [providers, models, publicSettings, fallback] = await Promise.all([
          llmApi.listProviders(token),
          llmApi.listModels(token),
          settingsApi.listPublic(token).catch(() => [] as Awaited<ReturnType<typeof settingsApi.listPublic>>),
          llmApi.fallbackStatus().catch(() => null as FallbackMirror | null)
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
        if (fallback) {
          this.fallbackState = fallback.state
          this.fallbackModels = fallback.models
        }
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
    },
    acceptDisclaimer() {
      localStorage.setItem('llm.fallback.disclaimer_accepted_v1', 'true')
    },
    /** Subscribe to `llm:fallback:progress` and `llm:fallback:done` events
     * for the duration of one install. Both events carry `model_id` in the
     * payload; the UI filters by the requested model. (Tauri's event-name
     * validator rejects `.` so we can't put model_id in the event name.)
     * Cleared automatically on done/error/cancel. */
    async subscribeInstallEvents(modelId: string, token: string) {
      // Tear down any prior listeners first.
      this.unsubscribeInstallEvents()
      const target = modelId
      const unlistenProgress = await listen<FallbackProgress>(
        'llm:fallback:progress',
        (e) => {
          if (e.payload.modelId !== target) return
          this.installProgress = e.payload
          this.installCurrentStage = e.payload.stage
        }
      )
      const unlistenDone = await listen<{ model_id: string; success: boolean; error: string }>(
        'llm:fallback:done',
        async (e) => {
          if (e.payload.model_id !== target) return
          this.installInFlight = false
          // Always wipe the in-flight UI state so the panel doesn't get
          // stuck showing "Downloading 0%" after the install finishes.
          this.installProgress = null
          this.installCurrentStage = null
          if (!e.payload.success) {
            this.installError = e.payload.error || 'install failed'
          }
          // Refresh the snapshot so the UI sees the new phase.
          try {
            const fb = await llmApi.fallbackStatus()
            this.fallbackState = fb.state
          } catch { /* ignore */ }
          this.unsubscribeInstallEvents()
        }
      )
      this.fallbackEventUnlisteners = [unlistenProgress, unlistenDone]
    },
    unsubscribeInstallEvents() {
      for (const u of this.fallbackEventUnlisteners) u()
      this.fallbackEventUnlisteners = []
    },
    async refreshFallback(token: string) {
      try {
        const fb = await llmApi.fallbackStatus()
        this.fallbackState = fb.state
        this.fallbackModels = fb.models
      } catch (e) {
        this.error = e instanceof Error ? e.message : String(e)
      }
    },
    async installModel(token: string, modelId: string, preferredUrl?: string) {
      this.installInFlight = true
      this.installProgress = null
      this.installCurrentStage = null
      this.installError = null
      await this.subscribeInstallEvents(modelId, token)
      try {
        await llmApi.fallbackInstallStart(token, modelId, preferredUrl)
      } catch (e) {
        this.installInFlight = false
        this.installError = e instanceof Error ? e.message : String(e)
        this.unsubscribeInstallEvents()
        throw e
      }
    },
    async cancelInstall(token: string) {
      // Cancel is fire-and-forget on the backend — it just flips a flag.
      // The download future will exit on its next chunk, but no `done`
      // event is guaranteed. Clear the UI state immediately so the
      // panel doesn't get stuck showing "downloading 0%" with a stale
      // Cancel button.
      try {
        await llmApi.fallbackInstallCancel(token)
      } catch (e) {
        this.installError = e instanceof Error ? e.message : String(e)
      } finally {
        this.installInFlight = false
        this.installProgress = null
        this.installCurrentStage = null
        this.unsubscribeInstallEvents()
        // Refresh the snapshot so the phase / size reflect what
        // actually got downloaded before cancellation.
        try { await this.refreshFallback(token) } catch { /* ignore */ }
      }
    },
    async startServer(token: string) {
      try {
        await llmApi.fallbackServerStart(token)
        await this.refreshFallback(token)
      } catch (e) {
        this.installError = e instanceof Error ? e.message : String(e)
        throw e
      }
    },
    async stopServer(token: string) {
      try {
        await llmApi.fallbackServerStop(token)
        await this.refreshFallback(token)
      } catch (e) {
        this.installError = e instanceof Error ? e.message : String(e)
        throw e
      }
    },
    async removeModel(token: string) {
      try {
        await llmApi.fallbackRemove(token)
        await this.refreshFallback(token)
      } catch (e) {
        this.installError = e instanceof Error ? e.message : String(e)
        throw e
      }
    },
    async fetchDiskFree(): Promise<number | null> {
      try {
        return await llmApi.fallbackDiskFree()
      } catch {
        return null
      }
    },
    async speedTest(token: string, modelId: string, manualUrl?: string): Promise<SpeedTestResult[]> {
      try {
        return await llmApi.fallbackSpeedTest(token, { modelId, manualUrl })
      } catch (e) {
        this.installError = e instanceof Error ? e.message : String(e)
        return []
      }
    },
    /** v0.7.1 — fetch provider catalog (OpenAI/Anthropic/Google). */
    async fetchProviderModels(token: string, providerId: string): Promise<RemoteModelInfo[]> {
      try {
        return await llmApi.listProviderModels(token, providerId)
      } catch (e) {
        ElMessage.error(e instanceof Error ? e.message : String(e))
        return []
      }
    },
    async importLocal(token: string, modelId: string, sourcePath: string, expectedSha256?: string): Promise<number> {
      try {
        return await llmApi.fallbackImportLocal(token, modelId, sourcePath, expectedSha256)
      } catch (e) {
        this.installError = e instanceof Error ? e.message : String(e)
        throw e
      } finally {
        await this.refreshFallback(token)
      }
    }
  }
})