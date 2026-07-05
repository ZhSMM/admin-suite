<template>
  <div class="page-container ai-tool">
    <div class="page-header">
      <h2>{{ title }}</h2>
      <el-select v-model="providerId" :placeholder="t('ai.common.provider')" style="width: 180px" @change="onProviderChange">
        <el-option
          v-for="p in llm.enabledProviders"
          :key="p.id"
          :label="p.name"
          :value="p.id"
        />
      </el-select>
      <el-select v-model="modelId" :placeholder="t('ai.common.model')" style="width: 220px; margin-left: 8px" :disabled="!providerId">
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
    </el-alert>

    <div v-else class="ai-grid">
      <el-input
        v-model="input"
        type="textarea"
        :rows="10"
        :placeholder="inputPlaceholder"
        :disabled="status === 'sending'"
      />
      <div class="actions">
        <el-button @click="onCopy" :disabled="!output">{{ t('common.copy') }}</el-button>
        <el-button @click="onClear">{{ t('common.clear') }}</el-button>
        <el-button
          type="primary"
          :loading="status === 'sending'"
          :disabled="!input.trim() || !providerId || !modelId"
          @click="onRun"
        >
          {{ runLabel }}
        </el-button>
      </div>
      <el-input
        v-model="output"
        type="textarea"
        :rows="10"
        :placeholder="outputPlaceholder"
        readonly
      />
      <div v-if="usage" class="usage">
        <el-tag size="small">{{ t('ai.common.tokens') }}: {{ usage }}</el-tag>
        <el-tag size="small" type="info">{{ t('ai.common.latency') }}: {{ latency }}ms</el-tag>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { useAuthStore } from '@/stores/auth'
import { useLlmStore } from '@/stores/llm'

const props = defineProps<{
  title: string
  inputPlaceholder: string
  outputPlaceholder: string
  runLabel: string
  systemPrompt: string | ((input: string) => string)
  initialInput?: string
}>()

const { t } = useI18n()
const auth = useAuthStore()
const llm = useLlmStore()

const providerId = ref<string>(llm.effectiveProviderId ?? '')
const modelId = ref<string>(llm.effectiveModelId ?? '')
const input = ref(props.initialInput ?? '')
const output = ref('')
const usage = ref<string>('')
const latency = ref<number>(0)
const status = ref<'idle' | 'sending'>('idle')

const modelsForProvider = computed(() =>
  providerId.value ? llm.modelsFor(providerId.value) : []
)

const onProviderChange = () => {
  if (modelsForProvider.value.length > 0 && !modelId.value) {
    modelId.value = modelsForProvider.value[0].id
  }
}

const onRun = async () => {
  if (!providerId.value || !modelId.value || !input.value.trim()) return
  output.value = ''
  usage.value = ''
  latency.value = 0
  status.value = 'sending'
  const sys = typeof props.systemPrompt === 'function'
    ? props.systemPrompt(input.value)
    : props.systemPrompt
  const start = performance.now()
  try {
    const result = await llm.sendChat(auth.token || '', {
      provider_id: providerId.value,
      model_id: modelId.value,
      system: sys,
      messages: [{ role: 'user', content: input.value }],
      temperature: 0.3
    })
    output.value = result.content
    usage.value = `${result.prompt_tokens}+${result.completion_tokens}=${result.total_tokens}`
    latency.value = Math.round(performance.now() - start)
    llm.setDefault(providerId.value, modelId.value)
  } catch (e) {
    ElMessage.error(e instanceof Error ? e.message : String(e))
  } finally {
    status.value = 'idle'
  }
}

const onCopy = async () => {
  if (!output.value) return
  await navigator.clipboard.writeText(output.value)
  ElMessage.success(t('common.copySuccess'))
}

const onClear = () => {
  input.value = ''
  output.value = ''
  usage.value = ''
  latency.value = 0
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
.ai-tool {
  display: flex;
  flex-direction: column;
}
.ai-grid {
  display: flex;
  flex-direction: column;
  gap: 12px;
  background: var(--el-bg-color);
  padding: 16px;
  border-radius: 6px;
}
.actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
.usage {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}
</style>