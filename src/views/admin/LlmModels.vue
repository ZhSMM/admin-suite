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
    </div>

    <el-table :data="filteredModels" stripe size="small" empty-text="-">
      <el-table-column prop="code" label="Code" width="200">
        <template #default="{ row }"><code>{{ row.code }}</code></template>
      </el-table-column>
      <el-table-column prop="display_name" :label="t('llm.models.col.name')" min-width="180" />
      <el-table-column :label="t('llm.models.col.provider')" width="180">
        <template #default="{ row }">
          <span>{{ providerName(row.provider_id) }}</span>
        </template>
      </el-table-column>
      <el-table-column :label="t('llm.models.col.context')" width="100" align="right">
        <template #default="{ row }">{{ row.context_window }}</template>
      </el-table-column>
      <el-table-column :label="t('llm.models.col.maxOutput')" width="100" align="right">
        <template #default="{ row }">{{ row.max_output }}</template>
      </el-table-column>
      <el-table-column :label="t('llm.models.col.caps')" min-width="160">
        <template #default="{ row }">
          <el-tag v-for="c in caps(row.capabilities)" :key="c" size="small" style="margin-right: 4px">
            {{ c }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('llm.models.col.enabled')" width="80">
        <template #default="{ row }">
          <el-tag v-if="row.enabled" type="success" size="small">ON</el-tag>
          <el-tag v-else type="info" size="small">OFF</el-tag>
        </template>
      </el-table-column>
      <el-table-column width="160" align="right">
        <template #default="{ row }">
          <el-button size="small" text @click="openEdit(row)">{{ t('common.edit') }}</el-button>
          <el-button size="small" text type="danger" @click="onDelete(row)">{{ t('common.delete') }}</el-button>
        </template>
      </el-table-column>
    </el-table>

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
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import { useAuthStore } from '@/stores/auth'
import { useLlmStore } from '@/stores/llm'
import { llmApi, type LlmModel } from '@/api/llm'

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