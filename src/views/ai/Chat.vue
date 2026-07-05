<template>
  <div class="page-container chat-page">
    <div class="page-header">
      <h2>{{ t('ai.chat.title') }}</h2>
      <el-select v-model="providerId" :placeholder="t('ai.chat.provider')" style="width: 200px" @change="onProviderChange">
        <el-option
          v-for="p in llm.enabledProviders"
          :key="p.id"
          :label="p.name"
          :value="p.id"
        />
      </el-select>
      <el-select v-model="modelId" :placeholder="t('ai.chat.model')" style="width: 240px; margin-left: 8px" :disabled="!providerId">
        <el-option
          v-for="m in modelsForProvider"
          :key="m.id"
          :label="m.display_name"
          :value="m.id"
        />
      </el-select>
    </div>

    <el-alert
      v-if="llm.providers.length === 0"
      :title="t('ai.chat.noProvidersTitle')"
      type="info"
      show-icon
      :closable="false"
    >
      <p>{{ t('ai.chat.noProvidersDesc') }}</p>
      <el-button type="primary" size="small" @click="goSettings">
        {{ t('ai.chat.openSettings') }}
      </el-button>
    </el-alert>

    <div v-else class="chat-grid">
      <div class="chat-history">
        <div v-for="(m, i) in messages" :key="i" :class="['msg', `msg-${m.role}`]">
          <div class="msg-head">
            <span class="role">{{ roleLabel(m.role) }}</span>
          </div>
          <div class="msg-body">
            <pre>{{ m.content }}</pre>
          </div>
        </div>
        <div v-if="status === 'sending'" class="msg msg-assistant">
          <div class="msg-head"><span class="role">{{ t('ai.chat.assistant') }}</span></div>
          <div class="msg-body"><pre class="placeholder">{{ t('ai.chat.thinking') }}</pre></div>
        </div>
      </div>

      <div class="chat-input">
        <el-input
          v-model="input"
          type="textarea"
          :rows="4"
          :placeholder="t('ai.chat.inputPlaceholder')"
          :disabled="!providerId || !modelId || status === 'sending'"
          @keydown.enter.exact.prevent="onSend"
        />
        <div class="actions">
          <el-button :disabled="!providerId || !modelId" @click="onClear">
            {{ t('ai.chat.clear') }}
          </el-button>
          <el-button
            type="primary"
            :loading="status === 'sending'"
            :disabled="!input.trim() || !providerId || !modelId"
            @click="onSend"
          >
            {{ t('ai.chat.send') }}
          </el-button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { useLlmStore } from '@/stores/llm'
import { llmApi, type ChatMessage } from '@/api/llm'

const { t } = useI18n()
const router = useRouter()
const auth = useAuthStore()
const llm = useLlmStore()

const providerId = ref<string>(llm.effectiveProviderId ?? '')
const modelId = ref<string>(llm.effectiveModelId ?? '')
// Prefill from Command Palette `? <text>` or any other router state handoff.
const input = ref<string>(
  (history.state && typeof history.state.prefill === 'string') ? history.state.prefill : ''
)
const messages = ref<ChatMessage[]>([])
const status = ref<'idle' | 'sending'>('idle')

const modelsForProvider = computed(() =>
  providerId.value ? llm.modelsFor(providerId.value) : []
)

watch(providerId, () => {
  // reset model selection when provider changes
  modelId.value = ''
})

const onProviderChange = () => {
  // pick first available model when switching providers
  if (modelsForProvider.value.length > 0 && !modelId.value) {
    modelId.value = modelsForProvider.value[0].id
  }
}

const roleLabel = (r: string) => {
  if (r === 'user') return t('ai.chat.user')
  if (r === 'assistant') return t('ai.chat.assistant')
  return r
}

const goSettings = () => router.push('/system/llm/providers')

const onClear = () => {
  messages.value = []
}

const onSend = async () => {
  if (!providerId.value || !modelId.value || !input.value.trim()) return
  const userMsg: ChatMessage = { role: 'user', content: input.value.trim() }
  messages.value = [...messages.value, userMsg]
  input.value = ''
  status.value = 'sending'
  try {
    const result = await llmApi.chat(auth.token || '', {
      provider_id: providerId.value,
      model_id: modelId.value,
      messages: messages.value
    })
    messages.value = [...messages.value, { role: 'assistant', content: result.content }]
    llm.setDefault(providerId.value, modelId.value)
  } catch (e) {
    ElMessage.error(e instanceof Error ? e.message : String(e))
  } finally {
    status.value = 'idle'
  }
}

onMounted(async () => {
  await llm.loadAll(auth.token || '')
  if (!providerId.value && llm.enabledProviders.length > 0) {
    providerId.value = llm.enabledProviders[0].id
    onProviderChange()
  }
})
</script>

<style scoped>
.chat-page {
  display: flex;
  flex-direction: column;
  height: calc(100vh - var(--header-height));
}
.chat-grid {
  display: flex;
  flex-direction: column;
  flex: 1;
  background: var(--el-bg-color);
  border-radius: 6px;
  overflow: hidden;
}
.chat-history {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.msg {
  border-radius: 6px;
  padding: 8px 12px;
  max-width: 80%;
}
.msg-user {
  align-self: flex-end;
  background: var(--el-color-primary-light-9);
}
.msg-assistant {
  align-self: flex-start;
  background: var(--el-fill-color-light);
}
.msg-system {
  align-self: center;
  background: var(--el-color-warning-light-9);
  font-style: italic;
}
.msg-head {
  font-size: 11px;
  color: var(--el-text-color-secondary);
  margin-bottom: 4px;
}
.role {
  font-weight: 600;
}
.msg-body pre {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 13px;
}
.placeholder {
  color: var(--el-text-color-placeholder);
  font-style: italic;
}
.chat-input {
  border-top: 1px solid var(--border-color, #e5e6eb);
  padding: 12px;
  background: var(--bg-primary, #fff);
}
.actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 8px;
}
</style>