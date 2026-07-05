<template>
  <div class="page-container">
    <div class="page-header">
      <h2>{{ t('llm.usage.title') }}</h2>
      <el-button :icon="Refresh" :loading="loading" @click="reload">{{ t('common.refresh') }}</el-button>
    </div>

    <el-card shadow="never" class="summary">
      <el-row :gutter="16">
        <el-col :span="6">
          <div class="metric">
            <span class="label">{{ t('llm.usage.totalCalls') }}</span>
            <span class="value">{{ totals.calls }}</span>
          </div>
        </el-col>
        <el-col :span="6">
          <div class="metric">
            <span class="label">{{ t('llm.usage.totalTokens') }}</span>
            <span class="value">{{ totals.tokens.toLocaleString() }}</span>
          </div>
        </el-col>
        <el-col :span="6">
          <div class="metric">
            <span class="label">{{ t('llm.usage.totalCost') }}</span>
            <span class="value">${{ totals.cost.toFixed(4) }}</span>
          </div>
        </el-col>
        <el-col :span="6">
          <div class="metric">
            <span class="label">{{ t('llm.usage.errorRate') }}</span>
            <span class="value">{{ totals.errorRate.toFixed(1) }}%</span>
          </div>
        </el-col>
      </el-row>
    </el-card>

    <el-table :data="rows" stripe size="small" empty-text="-" style="margin-top: 12px">
      <el-table-column :label="t('llm.usage.col.when')" prop="ts_unix_ms" width="180" sortable>
        <template #default="{ row }">{{ formatTs(row.ts_unix_ms) }}</template>
      </el-table-column>
      <el-table-column :label="t('llm.usage.col.user')" prop="user_id" width="140" />
      <el-table-column :label="t('llm.usage.col.provider')" prop="provider_id" width="140" />
      <el-table-column :label="t('llm.usage.col.model')" prop="model_id" width="140" />
      <el-table-column :label="t('llm.usage.col.tokens')" width="120" align="right">
        <template #default="{ row }">
          {{ row.prompt_tokens }} + {{ row.completion_tokens }}
        </template>
      </el-table-column>
      <el-table-column :label="t('llm.usage.col.latency')" width="100" align="right">
        <template #default="{ row }">{{ row.latency_ms }} ms</template>
      </el-table-column>
      <el-table-column :label="t('llm.usage.col.success')" width="80">
        <template #default="{ row }">
          <el-tag v-if="row.success" type="success" size="small">OK</el-tag>
          <el-tag v-else type="danger" size="small">FAIL</el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('llm.usage.col.error')" min-width="200">
        <template #default="{ row }">
          <span v-if="row.error" class="err">{{ row.error }}</span>
          <span v-else class="muted">-</span>
        </template>
      </el-table-column>
    </el-table>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Refresh } from '@element-plus/icons-vue'
import { useAuthStore } from '@/stores/auth'
import { llmApi, type LlmUsageRow } from '@/api/llm'

const { t, locale } = useI18n()
const auth = useAuthStore()
const rows = ref<LlmUsageRow[]>([])
const loading = ref(false)

const reload = async () => {
  loading.value = true
  try {
    rows.value = await llmApi.queryUsage(auth.token || '', { limit: 200 })
  } finally {
    loading.value = false
  }
}

const totals = computed(() => {
  let calls = 0
  let tokens = 0
  let cost = 0
  let errors = 0
  for (const r of rows.value) {
    calls += 1
    tokens += r.total_tokens
    cost += r.cost_usd
    if (!r.success) errors += 1
  }
  const errorRate = calls > 0 ? (errors / calls) * 100 : 0
  return { calls, tokens, cost, errorRate }
})

const formatTs = (ts: number) => {
  return new Date(ts).toLocaleString(locale.value === 'zh-CN' ? 'zh-CN' : 'en-US')
}

onMounted(reload)
</script>

<style scoped>
.summary {
  margin-bottom: 12px;
}
.metric {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 4px 0;
}
.metric .label {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}
.metric .value {
  font-size: 22px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}
.err {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 11px;
  color: var(--el-color-danger);
}
.muted {
  color: var(--el-text-color-secondary);
}
</style>