<template>
  <div class="page-container">
    <div class="page-header">
      <h2>{{ t('llm.providers.title') }}</h2>
      <el-button type="primary" :icon="Plus" @click="openCreate">
        {{ t('llm.providers.add') }}
      </el-button>
    </div>

    <el-alert
      v-if="llm.error"
      type="error"
      :title="llm.error"
      show-icon
      :closable="false"
      style="margin-bottom: 12px"
    />

    <el-table :data="llm.providers" stripe size="small" empty-text="-">
      <el-table-column prop="code" label="Code" width="180">
        <template #default="{ row }"><code>{{ row.code }}</code></template>
      </el-table-column>
      <el-table-column prop="name" :label="t('llm.providers.col.name')" min-width="160" />
      <el-table-column prop="kind" :label="t('llm.providers.col.kind')" width="140" />
      <el-table-column prop="base_url" :label="t('llm.providers.col.url')" min-width="220">
        <template #default="{ row }"><code class="url">{{ row.base_url }}</code></template>
      </el-table-column>
      <el-table-column prop="auth_type" :label="t('llm.providers.col.auth')" width="100" />
      <el-table-column :label="t('llm.providers.col.enabled')" width="80">
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
      :title="dialog.id ? t('llm.providers.editTitle') : t('llm.providers.addTitle')"
      width="640"
      :close-on-click-modal="false"
    >
      <el-form :model="dialog.form" label-width="140" size="default">
        <el-form-item :label="t('llm.providers.col.code')">
          <el-input v-model="dialog.form.code" :disabled="!!dialog.id" />
        </el-form-item>
        <el-form-item :label="t('llm.providers.col.name')">
          <el-input v-model="dialog.form.name" />
        </el-form-item>
        <el-form-item :label="t('llm.providers.col.kind')">
          <el-select v-model="dialog.form.kind" style="width: 100%">
            <el-option label="OpenAI Compatible" value="openai_compat" />
            <el-option label="Anthropic" value="anthropic" />
            <el-option label="Google Gemini" value="google" />
            <el-option label="Custom" value="custom" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('llm.providers.col.url')">
          <el-input v-model="dialog.form.base_url" placeholder="https://api.openai.com/v1" />
        </el-form-item>
        <el-form-item :label="t('llm.providers.col.auth')">
          <el-select v-model="dialog.form.auth_type" style="width: 100%">
            <el-option label="Bearer" value="bearer" />
            <el-option label="Custom Header" value="header" />
            <el-option label="None" value="none" />
          </el-select>
        </el-form-item>
        <el-form-item v-if="dialog.form.auth_type === 'header'" :label="t('llm.providers.col.header')">
          <el-input v-model="dialog.form.auth_header" placeholder="x-api-key" />
        </el-form-item>
        <el-form-item :label="t('llm.providers.col.apiKey')">
          <el-input
            v-model="dialog.form.api_key"
            type="password"
            show-password
            :placeholder="dialog.id ? t('llm.providers.apiKeyHint') : ''"
          />
        </el-form-item>
        <el-form-item :label="t('llm.providers.col.enabled')">
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
import { onMounted, reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import { useAuthStore } from '@/stores/auth'
import { useLlmStore } from '@/stores/llm'
import { llmApi, type LlmProvider } from '@/api/llm'

const { t } = useI18n()
const auth = useAuthStore()
const llm = useLlmStore()

const dialog = reactive<{
  visible: boolean
  saving: boolean
  id: string | null
  form: {
    code: string
    name: string
    kind: LlmProvider['kind']
    base_url: string
    auth_type: LlmProvider['auth_type']
    auth_header: string
    api_key: string
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
    code: '',
    name: '',
    kind: 'openai_compat' as LlmProvider['kind'],
    base_url: '',
    auth_type: 'bearer' as LlmProvider['auth_type'],
    auth_header: '',
    api_key: '',
    enabled: true
  }
}

const openCreate = () => {
  dialog.id = null
  dialog.form = emptyForm()
  dialog.visible = true
}

const openEdit = (row: LlmProvider) => {
  dialog.id = row.id
  dialog.form = {
    code: row.code,
    name: row.name,
    kind: row.kind,
    base_url: row.base_url,
    auth_type: row.auth_type,
    auth_header: row.auth_header ?? '',
    api_key: '', // never echo back
    enabled: row.enabled
  }
  dialog.visible = true
}

const onSave = async () => {
  dialog.saving = true
  try {
    if (dialog.id) {
      await llmApi.updateProvider(auth.token || '', {
        id: dialog.id,
        name: dialog.form.name,
        base_url: dialog.form.base_url,
        auth_type: dialog.form.auth_type,
        auth_header: dialog.form.auth_header || null,
        api_key: dialog.form.api_key || '', // empty string => leave unchanged
        enabled: dialog.form.enabled
      })
    } else {
      await llmApi.createProvider(auth.token || '', {
        code: dialog.form.code,
        name: dialog.form.name,
        kind: dialog.form.kind,
        base_url: dialog.form.base_url,
        auth_type: dialog.form.auth_type,
        auth_header: dialog.form.auth_header || null,
        api_key: dialog.form.api_key || null,
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

const onDelete = async (row: LlmProvider) => {
  try {
    await ElMessageBox.confirm(
      t('llm.providers.deleteConfirm', { name: row.name }),
      t('common.confirm'),
      { type: 'warning' }
    )
  } catch {
    return
  }
  await llmApi.deleteProvider(auth.token || '', row.id)
  await llm.loadAll(auth.token || '')
  ElMessage.success(t('common.deleteSuccess'))
}

onMounted(() => llm.loadAll(auth.token || ''))
</script>

<style scoped>
.url {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 12px;
}
code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  background: var(--el-fill-color-light);
  padding: 1px 6px;
  border-radius: 3px;
}
</style>