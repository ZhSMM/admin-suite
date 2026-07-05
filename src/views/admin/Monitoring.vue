<template>
  <div class="page-container">
    <div class="page-header">
      <h2>{{ t('monitoring.title') }}</h2>
      <div>
        <el-checkbox v-model="autoRefresh" @change="onAutoRefreshToggle">
          {{ t('monitoring.autoRefresh') }}
        </el-checkbox>
        <el-button
          :icon="Refresh"
          :loading="metrics.loading"
          style="margin-left: 8px"
          @click="metrics.refresh()"
        >
          {{ t('common.refresh') }}
        </el-button>
        <el-button :icon="Delete" type="danger" plain @click="onClear">
          {{ t('monitoring.clear') }}
        </el-button>
      </div>
    </div>

    <el-alert
      v-if="metrics.items.length === 0 && !metrics.loading"
      :title="t('monitoring.emptyTitle')"
      :description="t('monitoring.emptyDesc')"
      type="info"
      show-icon
      :closable="false"
      style="margin-bottom: 12px"
    />

    <el-table
      v-else
      :data="metrics.items"
      stripe
      size="small"
      empty-text="-"
    >
      <el-table-column prop="command" :label="t('monitoring.col.command')" min-width="200">
        <template #default="{ row }">
          <code class="cmd-cell">{{ row.command }}</code>
        </template>
      </el-table-column>
      <el-table-column
        prop="count"
        :label="t('monitoring.col.count')"
        width="80"
        align="right"
      />
      <el-table-column
        :label="t('monitoring.col.last')"
        width="100"
        align="right"
      >
        <template #default="{ row }">{{ formatMs(row.last_ms) }}</template>
      </el-table-column>
      <el-table-column
        :label="t('monitoring.col.avg')"
        width="100"
        align="right"
      >
        <template #default="{ row }">{{ formatMs(row.avg_ms) }}</template>
      </el-table-column>
      <el-table-column
        :label="t('monitoring.col.max')"
        width="100"
        align="right"
      >
        <template #default="{ row }">
          <span :class="latencyClass(row.max_ms)">{{ formatMs(row.max_ms) }}</span>
        </template>
      </el-table-column>
      <el-table-column
        :label="t('monitoring.col.total')"
        width="100"
        align="right"
      >
        <template #default="{ row }">{{ formatMs(row.total_ms) }}</template>
      </el-table-column>
      <el-table-column :label="t('monitoring.col.errors')" width="80" align="right">
        <template #default="{ row }">
          <el-tag
            v-if="row.error_count > 0"
            type="danger"
            size="small"
            style="cursor: pointer"
            @click="showLastError(row)"
          >
            {{ row.error_count }}
          </el-tag>
          <span v-else class="muted">0</span>
        </template>
      </el-table-column>
      <el-table-column :label="t('monitoring.col.sparkline')" min-width="180">
        <template #default="{ row }">
          <div class="sparkline">
            <span
              v-for="(v, i) in row.history_ms"
              :key="i"
              class="sparkline-bar"
              :style="sparkStyle(v, row.max_ms)"
              :title="`${v} ms`"
            />
            <span v-if="row.history_ms.length === 0" class="muted">-</span>
          </div>
        </template>
      </el-table-column>
    </el-table>

    <el-dialog
      v-model="errorDialog.visible"
      :title="errorDialog.title"
      width="60%"
      :show-close="true"
    >
      <pre class="error-pre">{{ errorDialog.body }}</pre>
      <template #footer>
        <el-button @click="copyError">{{ t('common.copy') }}</el-button>
        <el-button type="primary" @click="errorDialog.visible = false">
          {{ t('common.close') }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onBeforeUnmount } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Refresh, Delete } from '@element-plus/icons-vue'
import { useMetricsStore } from '@/stores/metrics'

const { t } = useI18n()
const metrics = useMetricsStore()
const autoRefresh = ref(false)
const errorDialog = ref({ visible: false, title: '', body: '' })

let pollTimer: ReturnType<typeof setInterval> | null = null

const onAutoRefreshToggle = (val: string | number | boolean) => {
  if (pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  }
  if (val) {
    pollTimer = setInterval(() => metrics.refresh(), 3000)
  }
}

const onClear = async () => {
  try {
    await ElMessageBox.confirm(t('monitoring.clearConfirm'), t('common.confirm'), {
      type: 'warning'
    })
  } catch {
    return
  }
  await metrics.clear()
  ElMessage.success(t('monitoring.cleared'))
}

const formatMs = (ms: number | string) => {
  const n = typeof ms === 'string' ? parseFloat(ms) : ms
  if (!isFinite(n)) return '-'
  if (n < 1) return '<1 ms'
  if (n < 1000) return `${n.toFixed(1)} ms`
  return `${(n / 1000).toFixed(2)} s`
}

const latencyClass = (ms: number | string) => {
  const n = typeof ms === 'string' ? parseFloat(ms) : ms
  if (n >= 500) return 'latency-bad'
  if (n >= 100) return 'latency-warn'
  return ''
}

const sparkStyle = (v: number, max: number | string) => {
  const maxN = typeof max === 'string' ? parseFloat(max) : max
  const pct = maxN > 0 ? Math.max(4, Math.round((v / maxN) * 100)) : 4
  return { height: `${pct}%` }
}

// Click error cell -> dialog with last error message
const showLastError = (row: { command: string; last_error: string | null }) => {
  if (!row.last_error) return
  errorDialog.value = {
    visible: true,
    title: `${row.command} — last error`,
    body: row.last_error
  }
}

const copyError = async () => {
  await navigator.clipboard.writeText(errorDialog.value.body)
  ElMessage.success(t('common.copySuccess'))
}

metrics.refresh()

onBeforeUnmount(() => {
  if (pollTimer) clearInterval(pollTimer)
})
</script>

<style scoped>
.cmd-cell {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 12px;
  background: var(--el-fill-color-light);
  padding: 2px 6px;
  border-radius: 4px;
}
.muted {
  color: var(--el-text-color-secondary);
  font-size: 12px;
}
.latency-warn {
  color: var(--el-color-warning);
  font-weight: 600;
}
.latency-bad {
  color: var(--el-color-danger);
  font-weight: 600;
}
.sparkline {
  display: flex;
  align-items: flex-end;
  height: 24px;
  gap: 1px;
}
.sparkline-bar {
  flex: 1 1 0;
  background: var(--el-color-primary-light-5);
  min-height: 2px;
  border-radius: 1px 1px 0 0;
  transition: background 0.15s;
}
.sparkline-bar:hover {
  background: var(--el-color-primary);
}
.error-pre {
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