<template>
  <div class="page-container">
    <div class="page-header">
      <h2>{{ t('llm.models.title') }}</h2>
      <el-select
        v-model="filterProviderId"
        :placeholder="t('llm.models.allProviders')"
        clearable
        style="width: 280px"
      >
        <el-option
          v-for="p in llm.providers"
          :key="p.id"
          :label="`${p.name} (${p.code})`"
          :value="p.id"
        />
      </el-select>
      <el-button type="primary" :icon="Plus" style="margin-left: 12px" @click="openCreate">
        {{ t('llm.models.add') }}
      </el-button>
      <el-button
        :icon="Download"
        style="margin-left: 8px"
        :loading="fetching"
        @click="onFetchModels"
      >
        {{ t('llm.models.fetch') }}
      </el-button>
    </div>

    <el-alert
      v-if="!supportsAnyFetch"
      type="info"
      :closable="false"
      style="margin: 12px 0"
    >
      {{ t('llm.models.fetchHint') }}
    </el-alert>

    <el-table :data="filteredModels" stripe size="small" empty-text="-">

    <el-dialog
      v-model="dialog.visible"
      :title="dialog.id ? t('llm.models.editTitle') : t('llm.models.addTitle')"
      width="640"
      :close-on-click-modal="false"
    >
      <el-form :model="dialog.form" label-width="160" size="default">
        <el-form-item :label="t('llm.models.col.provider')">
          <el-select v-model="dialog.form.provider_id" :disabled="!!dialog.id" style="width: 100%">
            <el-option
              v-for="p in llm.providers"
              :key="p.id"
              :label="`${p.name} (${p.code})`"
              :value="p.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="Code">
          <el-input v-model="dialog.form.code" :disabled="!!dialog.id" />
        </el-form-item>
        <el-form-item :label="t('llm.models.col.name')">
          <el-input v-model="dialog.form.display_name" />
        </el-form-item>
        <el-form-item :label="t('llm.models.col.context')">
          <el-input-number v-model="dialog.form.context_window" :min="512" :max="2_000_000" :step="512" />
        </el-form-item>
        <el-form-item :label="t('llm.models.col.maxOutput')">
          <el-input-number v-model="dialog.form.max_output" :min="64" :max="128_000" :step="64" />
        </el-form-item>
        <el-form-item :label="t('llm.models.col.caps')">
          <el-input v-model="dialog.form.capabilities" placeholder='["chat","stream"]' />
        </el-form-item>
        <el-form-item :label="t('llm.models.col.enabled')">
          <el-switch v-model="dialog.form.enabled" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialog.visible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="dialog.saving" @click="onSave">
          {{ t('common.save') }}
        </el-button>
      </template>
    </el-dialog>

    <!-- v0.7.1 — fetch model catalog -->
    <el-dialog
      v-model="fetchDialog.visible"
      :title="t('llm.models.fetchTitle')"
      width="780"
      :close-on-click-modal="false"
    >
      <div class="fetch-row">
        <el-select
          v-model="fetchDialog.providerId"
          :placeholder="t('llm.models.col.provider')"
          style="flex: 1"
          @change="onFetchModels"
        >
          <el-option
            v-for="p in fetchableProviders"
            :key="p.id"
            :label="`${p.name} (${p.code})`"
            :value="p.id"
          />
        </el-select>
        <el-button :icon="Refresh" :loading="fetching" @click="onFetchModels">
          {{ t('common.refresh') }}
        </el-button>
      </div>
      <el-table
        ref="fetchTableRef"
        :data="fetchDialog.candidates"
        size="small"
        empty-text="-"
        @selection-change="onSelectionChange"
      >
        <el-table-column type="selection" width="40" />
        <el-table-column prop="id" label="Code" min-width="220">
          <template #default="{ row }"><code>{{ row.id }}</code></template>
        </el-table-column>
        <el-table-column :label="t('llm.models.col.name')" min-width="180">
          <template #default="{ row }">{{ row.display_name || row.id }}</template>
        </el-table-column>
        <el-table-column :label="t('llm.models.col.context')" width="100" align="right">
          <template #default="{ row }">{{ row.context_window ?? '-' }}</template>
        </el-table-column>
        <el-table-column :label="t('llm.models.col.caps')" width="120">
          <template #default="{ row }">
            <el-tag v-if="row.kind" size="small">{{ row.kind }}</el-tag>
            <span v-else class="muted">-</span>
          </template>
        </el-table-column>
        <el-table-column :label="t('llm.models.alreadyExists')" width="120">
          <template #default="{ row }">
            <el-tag v-if="existingCodeSet.has(row.id)" size="small" type="info">
              {{ t('llm.models.exists') }}
            </el-tag>
          </template>
        </el-table-column>
      </el-table>
      <template #footer>
        <el-button @click="fetchDialog.visible = false">{{ t('common.cancel') }}</el-button>
        <el-button
          type="primary"
          :loading="importing"
          :disabled="fetchDialog.selected.length === 0"
          @click="onImportSelected"
        >
          {{ t('llm.models.importSelected', { n: fetchDialog.selected.length }) }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Download, Plus, Refresh } from '@element-plus/icons-vue'
import { useAuthStore } from '@/stores/auth'
import { useLlmStore } from '@/stores/llm'
import { llmApi, type LlmModel, type RemoteModelInfo } from '@/api/llm'

const { t } = useI18n()
const auth = useAuthStore()
const llm = useLlmStore()

const filterProviderId = ref<string | undefined>(undefined)
const filteredModels = computed(() =>
  filterProviderId.value
    ? llm.models.filter((m) => m.provider_id === filterProviderId.value)
    : llm.models
)

const providerName = (id: string) => {
  const p = llm.providers.find((x) => x.id === id)
  return p ? p.name : id
}

const caps = (json: string): string[] => {
  try {
    const v = JSON.parse(json)
    return Array.isArray(v) ? v : []
  } catch {
    return []
  }
}

const dialog = reactive<{
  visible: boolean
  saving: boolean
  id: string | null
  form: {
    provider_id: string
    code: string
    display_name: string
    context_window: number
    max_output: number
    capabilities: string
    enabled: boolean
  }
}>({
  visible: false,
  saving: false,
  id: null,
  form: emptyForm()
})

function emptyForm() {
  return {
    provider_id: '',
    code: '',
    display_name: '',
    context_window: 4096,
    max_output: 2048,
    capabilities: '["chat","stream"]',
    enabled: true
  }
}

const openCreate = () => {
  dialog.id = null
  dialog.form = emptyForm()
  dialog.visible = true
}

const openEdit = (row: LlmModel) => {
  dialog.id = row.id
  dialog.form = {
    provider_id: row.provider_id,
    code: row.code,
    display_name: row.display_name,
    context_window: row.context_window,
    max_output: row.max_output,
    capabilities: row.capabilities,
    enabled: row.enabled
  }
  dialog.visible = true
}

const onSave = async () => {
  dialog.saving = true
  try {
    if (dialog.id) {
      await llmApi.updateModel(auth.token || '', {
        id: dialog.id,
        display_name: dialog.form.display_name,
        context_window: dialog.form.context_window,
        max_output: dialog.form.max_output,
        capabilities: dialog.form.capabilities,
        enabled: dialog.form.enabled
      })
    } else {
      await llmApi.createModel(auth.token || '', {
        provider_id: dialog.form.provider_id,
        code: dialog.form.code,
        display_name: dialog.form.display_name,
        context_window: dialog.form.context_window,
        max_output: dialog.form.max_output,
        capabilities: dialog.form.capabilities,
        enabled: dialog.form.enabled
      })
    }
    ElMessage.success(t('common.saveSuccess'))
    dialog.visible = false
    await llm.loadAll(auth.token || '')
  } catch (e) {
    ElMessage.error(e instanceof Error ? e.message : String(e))
  } finally {
    dialog.saving = false
  }
}

const onDelete = async (row: LlmModel) => {
  try {
    await ElMessageBox.confirm(
      t('llm.models.deleteConfirm', { name: row.display_name }),
      t('common.confirm'),
      { type: 'warning' }
    )
  } catch {
    return
  }
  await llmApi.deleteModel(auth.token || '', row.id)
  await llm.loadAll(auth.token || '')
  ElMessage.success(t('common.deleteSuccess'))
}

// ---- v0.7.1 — fetch from provider catalog ----
const FETCHABLE_KINDS = new Set(['openai_compat', 'anthropic', 'google'])
const fetchableProviders = computed(() =>
  llm.providers.filter((p) => FETCHABLE_KINDS.has(p.kind))
)
const supportsAnyFetch = computed(() => fetchableProviders.value.length > 0)

const fetchDialog = reactive<{
  visible: boolean
  providerId: string
  candidates: RemoteModelInfo[]
  selected: RemoteModelInfo[]
}>({
  visible: false,
  providerId: '',
  candidates: [],
  selected: []
})

const fetching = ref(false)
const importing = ref(false)

// Used by the import dialog to highlight rows whose code is already on file.
const existingCodeSet = computed(() => {
  const set = new Set<string>()
  for (const m of llm.models) {
    if (fetchDialog.providerId && m.provider_id === fetchDialog.providerId) {
      set.add(m.code)
    }
  }
  return set
})

const onSelectionChange = (rows: RemoteModelInfo[]) => {
  fetchDialog.selected = rows
}

const onFetchModels = async () => {
  // First click from the page toolbar — open dialog with default selection
  // (first fetchable provider).
  if (!fetchDialog.visible) {
    fetchDialog.visible = true
    if (!fetchDialog.providerId && fetchableProviders.value.length > 0) {
      fetchDialog.providerId = fetchableProviders.value[0].id
    }
  }
  if (!fetchDialog.providerId) return
  fetching.value = true
  try {
    const items = await llm.fetchProviderModels(auth.token || '', fetchDialog.providerId)
    fetchDialog.candidates = items
    fetchDialog.selected = []
    if (items.length === 0) {
      ElMessage.warning(t('llm.models.fetchEmpty'))
    }
  } finally {
    fetching.value = false
  }
}

const onImportSelected = async () => {
  if (fetchDialog.selected.length === 0) return
  importing.value = true
  let ok = 0
  let failed = 0
  for (const m of fetchDialog.selected) {
    try {
      // Skip ones that already match an existing code under the same provider.
      if (existingCodeSet.value.has(m.id)) continue
      await llmApi.createModel(auth.token || '', {
        provider_id: fetchDialog.providerId,
        code: m.id,
        display_name: m.display_name || m.id,
        context_window: m.context_window ?? 4096,
        max_output: 2048,
        capabilities: JSON.stringify(['chat', 'stream']),
        enabled: true
      })
      ok += 1
    } catch (e) {
      failed += 1
    }
  }
  importing.value = false
  await llm.loadAll(auth.token || '')
  fetchDialog.selected = []
  ElMessage.success(t('llm.models.importDone', { ok, failed }))
}

onMounted(() => llm.loadAll(auth.token || ''))
</script>

<style scoped>
code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  background: var(--el-fill-color-light);
  padding: 1px 6px;
  border-radius: 3px;
}
</style>