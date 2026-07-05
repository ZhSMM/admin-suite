<template>
  <el-dialog
    v-model="dialog.open"
    :title="t('locales.aiFill.title')"
    width="780"
    :close-on-click-modal="false"
  >
    <el-form label-width="120px">
      <el-form-item :label="t('locales.aiFill.source')">
        <el-select v-model="dialog.sourceCode" filterable style="width: 100%">
          <el-option
            v-for="it in items"
            :key="it.id"
            :label="`${it.code} (${it.name})`"
            :value="it.code"
          />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('locales.aiFill.target')">
        <el-select v-model="dialog.targetCode" filterable style="width: 100%">
          <el-option
            v-for="it in items"
            :key="it.id"
            :label="`${it.code} (${it.name})`"
            :value="it.code"
          />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('locales.aiFill.provider')">
        <el-select v-model="dialog.providerId" style="width: 220px" @change="onProviderChange">
          <el-option
            v-for="p in llm.enabledProviders"
            :key="p.id"
            :label="p.name"
            :value="p.id"
          />
        </el-select>
        <el-select v-model="dialog.modelId" style="width: 220px; margin-left: 8px" :disabled="!dialog.providerId">
          <el-option
            v-for="m in modelsForProvider"
            :key="m.id"
            :label="m.display_name"
            :value="m.id"
          />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('locales.aiFill.missingCount')">
        <el-tag :type="missingKeys.length === 0 ? 'success' : 'warning'">
          {{ missingKeys.length }} / {{ totalKeys }}
        </el-tag>
        <span style="margin-left: 12px; color: var(--el-text-color-secondary)">
          {{ t('locales.aiFill.estimatedCost') }}: ~{{ estimatedCost }} tokens
        </span>
      </el-form-item>
      <el-alert
        v-if="!dialog.providerId || missingKeys.length === 0"
        :title="missingKeys.length === 0 ? t('locales.aiFill.allTranslated') : t('locales.aiFill.noProvider')"
        type="info"
        :closable="false"
        show-icon
      />
      <el-progress v-else-if="status === 'running'" :percentage="progress" />
    </el-form>
    <template #footer>
      <el-button @click="dialog.open = false">{{ t('common.close') }}</el-button>
      <el-button
        type="primary"
        :loading="status === 'running'"
        :disabled="missingKeys.length === 0 || !dialog.providerId || !dialog.modelId"
        @click="onRun"
      >
        {{ t('locales.aiFill.run') }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { useAuthStore } from '@/stores/auth'
import { useLlmStore } from '@/stores/llm'
import { useLocaleStore } from '@/stores/locale'
import type { Resource } from '@/api/resources'

const props = defineProps<{
  items: Resource[]
}>()

const { t } = useI18n()
const auth = useAuthStore()
const llm = useLlmStore()
const localeStore = useLocaleStore()

const dialog = reactive({
  open: false,
  sourceCode: '',
  targetCode: '',
  providerId: '',
  modelId: ''
})

const status = ref<'idle' | 'running'>('idle')
const progress = ref(0)

const modelsForProvider = computed(() =>
  dialog.providerId ? llm.modelsFor(dialog.providerId) : []
)

const sourceMessages = computed<Record<string, string>>(() => {
  const r = props.items.find((it) => it.code === dialog.sourceCode)
  if (!r) return {}
  try {
    return JSON.parse(r.content).messages || {}
  } catch {
    return {}
  }
})

const targetMessages = computed<Record<string, string>>(() => {
  const r = props.items.find((it) => it.code === dialog.targetCode)
  if (!r) return {}
  try {
    return JSON.parse(r.content).messages || {}
  } catch {
    return {}
  }
})

const totalKeys = computed(() => Object.keys(sourceMessages.value).length)

const missingKeys = computed<string[]>(() => {
  const t = targetMessages.value
  const s = sourceMessages.value
  return Object.keys(s).filter(
    (k) => !t[k] || t[k].trim() === ''
  )
})

// Rough cost estimate: chars / 4 (English), * 2 (input + output), + JSON overhead.
const estimatedCost = computed(() => {
  if (!missingKeys.value.length) return 0
  const charCount = missingKeys.value.reduce((acc, k) => {
    const v = sourceMessages.value[k] || ''
    return acc + k.length + v.length
  }, 0)
  return Math.round((charCount / 3) * 2)
})

function onProviderChange() {
  if (modelsForProvider.value.length > 0 && !dialog.modelId) {
    dialog.modelId = modelsForProvider.value[0].id
  }
}

function open(sourceCode = '', targetCode = '') {
  dialog.open = true
  dialog.sourceCode = sourceCode
  dialog.targetCode = targetCode
  dialog.providerId = llm.defaultProviderId ?? (llm.enabledProviders[0]?.id ?? '')
  onProviderChange()
  status.value = 'idle'
  progress.value = 0
}

defineExpose({ open })

// Watch dialog open → load providers if not yet loaded.
watch(
  () => dialog.open,
  (v) => {
    if (v) {
      void llm.loadAll(auth.token || '').then(() => {
        if (!dialog.providerId && llm.enabledProviders.length > 0) {
          dialog.providerId = llm.enabledProviders[0].id
          onProviderChange()
        }
      })
    }
  }
)

async function onRun() {
  if (!dialog.providerId || !dialog.modelId) return
  if (!missingKeys.value.length) return

  status.value = 'running'
  progress.value = 0

  const BATCH_SIZE = 30
  const batches: string[][] = []
  for (let i = 0; i < missingKeys.value.length; i += BATCH_SIZE) {
    batches.push(missingKeys.value.slice(i, i + BATCH_SIZE))
  }

  // The target locale name we want to translate INTO comes from the
  // selected target resource's name (e.g. "简体中文" → "Simplified Chinese").
  const targetResource = props.items.find((it) => it.code === dialog.targetCode)
  const targetLabel = targetResource?.name ?? dialog.targetCode

  const merged: Record<string, string> = { ...targetMessages.value }
  let filled = 0

  try {
    for (let bi = 0; bi < batches.length; bi++) {
      const batch = batches[bi]
      const lines = batch.map((k) => `${k}\t${sourceMessages.value[k] ?? ''}`)
      const sys = `You are a localization translator. Translate each line into ${targetLabel}.\n` +
        `Each line is "key<TAB>english-or-source-text". Output a single JSON object mapping each key to its translation.\n` +
        `Rules:\n` +
        `- Keep technical identifiers and code strings (camelCase, snake_case, paths, env keys) exactly as in the source unless the target language naturally requires lowercasing.\n` +
        `- Do NOT translate placeholder tokens like {name}, {count}, or {ts}.\n` +
        `- Do not invent keys that are not in the input.\n` +
        `- Return ONLY the JSON object, no markdown fences.`
      const userText = lines.join('\n')

      const result = await llm.sendChat(auth.token || '', {
        provider_id: dialog.providerId,
        model_id: dialog.modelId,
        system: sys,
        messages: [{ role: 'user', content: userText }],
        temperature: 0.2
      })

      // Parse JSON, fall back to a relaxed line-based parse.
      let parsed: Record<string, string> = {}
      const raw = result.content.trim()
      try {
        parsed = JSON.parse(raw)
      } catch {
        // Try to extract {...} block
        const m = raw.match(/\{[\s\S]*\}/)
        if (m) {
          try { parsed = JSON.parse(m[0]) } catch { /* ignore */ }
        }
      }
      if (!parsed || typeof parsed !== 'object') {
        throw new Error(t('locales.aiFill.parseError'))
      }

      for (const k of batch) {
        if (typeof parsed[k] === 'string' && parsed[k].trim()) {
          merged[k] = parsed[k].trim()
        }
      }
      filled += batch.length
      progress.value = Math.round((filled / missingKeys.value.length) * 100)
      llm.setDefault(dialog.providerId, dialog.modelId)
    }

    // Re-import the merged locale JSON.
    const targetResource = props.items.find((it) => it.code === dialog.targetCode)
    const payload = {
      id: targetResource ? JSON.parse(targetResource.content).id : dialog.targetCode,
      label: targetResource ? JSON.parse(targetResource.content).label : dialog.targetCode,
      messages: merged
    }
    await localeStore.importFromJson(JSON.stringify(payload, null, 2))
    ElMessage.success(t('locales.aiFill.success', { count: filled }))
    dialog.open = false
  } catch (e) {
    ElMessage.error(e instanceof Error ? e.message : String(e))
  } finally {
    status.value = 'idle'
  }
}
</script>