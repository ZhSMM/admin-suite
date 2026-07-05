<template>
  <div class="page-container">
    <div class="page-header">
      <h2>{{ t('diagnostics.title') }}</h2>
      <div>
        <el-button :icon="Refresh" :loading="crash.loading" @click="reload">
          {{ t('diagnostics.refresh') }}
        </el-button>
        <el-button
          :icon="Delete"
          type="danger"
          plain
          :disabled="crash.items.length === 0"
          @click="onClear"
        >
          {{ t('diagnostics.clear') }}
        </el-button>
      </div>
    </div>

    <el-alert
      v-if="crash.items.length === 0 && !crash.loading"
      :title="t('diagnostics.empty')"
      type="success"
      show-icon
      :closable="false"
    />

    <el-table
      v-else
      :data="crash.items"
      stripe
      size="small"
      :default-sort="{ prop: 'ts_unix_ms', order: 'descending' }"
    >
      <el-table-column
        :label="t('diagnostics.col.ts')"
        prop="ts_unix_ms"
        width="180"
        sortable
      >
        <template #default="{ row }">{{ formatTs(row.ts_unix_ms) }}</template>
      </el-table-column>
      <el-table-column :label="t('diagnostics.col.kind')" width="160">
        <template #default="{ row }">
          <el-tag :type="kindTag(row.kind)" size="small">
            {{ kindLabel(row.kind) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('diagnostics.col.message')" min-width="200">
        <template #default="{ row }">
          <span class="msg">{{ row.message }}</span>
          <div v-if="row.source" class="src">{{ row.source }}</div>
        </template>
      </el-table-column>
      <el-table-column :label="t('diagnostics.col.appVersion')" width="120">
        <template #default="{ row }">
          <span v-if="row.app_version" class="ver">{{ row.app_version }}</span>
          <span v-else class="muted">-</span>
        </template>
      </el-table-column>
      <el-table-column width="120" align="right">
        <template #default="{ row }">
          <el-button size="small" text @click="open(row)">
            {{ t('diagnostics.view') }}
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <el-dialog
      v-model="detail.visible"
      :title="detail.title"
      width="70%"
      :show-close="true"
    >
      <div v-if="detail.report" class="detail">
        <div class="detail-row">
          <span class="label">id</span>
          <code>{{ detail.report.id }}</code>
        </div>
        <div class="detail-row">
          <span class="label">{{ t('diagnostics.col.ts') }}</span>
          <span>{{ formatTs(detail.report.ts_unix_ms) }}</span>
        </div>
        <div class="detail-row">
          <span class="label">{{ t('diagnostics.col.kind') }}</span>
          <el-tag :type="kindTag(detail.report.kind)" size="small">
            {{ kindLabel(detail.report.kind) }}
          </el-tag>
        </div>
        <div class="detail-row">
          <span class="label">{{ t('diagnostics.col.message') }}</span>
          <span>{{ detail.report.message }}</span>
        </div>
        <div v-if="detail.report.source" class="detail-row">
          <span class="label">{{ t('diagnostics.col.source') }}</span>
          <code>{{ detail.report.source }}</code>
        </div>
        <div v-if="detail.report.app_version" class="detail-row">
          <span class="label">{{ t('diagnostics.col.appVersion') }}</span>
          <span>{{ detail.report.app_version }}</span>
        </div>
        <div v-if="detail.report.detail" class="detail-row block">
          <span class="label">{{ detail.report.kind === 'rust_panic' ? 'stack' : 'stack' }}</span>
          <pre class="pre">{{ detail.report.detail }}</pre>
        </div>
      </div>
      <template #footer>
        <el-button :disabled="!detail.report?.detail" @click="copyStack">
          {{ t('diagnostics.copy') }}
        </el-button>
        <el-button type="success" :disabled="!detail.report" @click="aiExplain">
          {{ t('diagnostics.aiExplain') }}
        </el-button>
        <el-button type="primary" @click="detail.visible = false">
          {{ t('common.close') }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Refresh, Delete } from '@element-plus/icons-vue'
import { useAuthStore } from '@/stores/auth'
import { useCrashStore } from '@/stores/crash'
import type { CrashReport, CrashKind } from '@/api/crash'

const { t, locale } = useI18n()
const auth = useAuthStore()
const crash = useCrashStore()
const router = useRouter()

const detail = reactive<{
  visible: boolean
  title: string
  report: CrashReport | null
}>({ visible: false, title: '', report: null })

const reload = () => crash.refresh(auth.token || '')

const onClear = async () => {
  try {
    await ElMessageBox.confirm(
      t('diagnostics.clearConfirm'),
      t('common.confirm'),
      { type: 'warning' }
    )
  } catch {
    return
  }
  await crash.clear(auth.token || '')
  ElMessage.success(t('diagnostics.cleared'))
}

const formatTs = (ts: number) => {
  const d = new Date(ts)
  // Match user's locale — `toLocaleString` on Tauri WebView respects it.
  return d.toLocaleString(locale.value === 'zh-CN' ? 'zh-CN' : 'en-US')
}

const kindLabel = (k: CrashKind) => {
  switch (k) {
    case 'rust_panic':
      return t('diagnostics.kind.rust_panic')
    case 'frontend_error':
      return t('diagnostics.kind.frontend_error')
    case 'frontend_unhandled_rejection':
      return t('diagnostics.kind.frontend_unhandled_rejection')
    default:
      return k
  }
}

const kindTag = (k: CrashKind): 'danger' | 'warning' | 'info' => {
  switch (k) {
    case 'rust_panic':
      return 'danger'
    case 'frontend_error':
      return 'warning'
    case 'frontend_unhandled_rejection':
      return 'info'
    default:
      return 'info'
  }
}

const open = (row: CrashReport) => {
  detail.report = row
  detail.title = `${formatTs(row.ts_unix_ms)} — ${kindLabel(row.kind)}`
  detail.visible = true
}

const copyStack = async () => {
  if (!detail.report?.detail) return
  await navigator.clipboard.writeText(detail.report.detail)
  ElMessage.success(t('diagnostics.copied'))
}

const aiExplain = () => {
  if (!detail.report) return
  const r = detail.report
  // Build a payload that explains exactly what the user is looking at.
  const lines = [
    `Kind: ${r.kind}`,
    `App version: ${r.app_version ?? 'unknown'}`,
    `Source: ${r.source ?? 'unknown'}`,
    `Message: ${r.message}`,
    '',
    r.detail ? `Detail:\n${r.detail}` : 'Detail: (none)'
  ]
  router.push({
    name: 'ai-explain',
    state: { prefill: lines.join('\n') }
  })
}

onMounted(() => reload())
</script>

<style scoped>
.msg {
  font-weight: 500;
  word-break: break-word;
}
.src {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 11px;
  color: var(--el-text-color-secondary);
  margin-top: 2px;
  word-break: break-all;
}
.ver {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 11px;
  background: var(--el-fill-color-light);
  padding: 1px 6px;
  border-radius: 3px;
}
.muted {
  color: var(--el-text-color-secondary);
}
.detail {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.detail-row {
  display: flex;
  gap: 12px;
  align-items: flex-start;
  font-size: 13px;
}
.detail-row.block {
  flex-direction: column;
}
.detail-row .label {
  flex: 0 0 100px;
  color: var(--el-text-color-secondary);
  font-weight: 500;
}
.pre {
  background: var(--el-fill-color-light);
  padding: 12px;
  border-radius: 4px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 12px;
  max-height: 50vh;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-all;
  margin: 0;
}
</style>