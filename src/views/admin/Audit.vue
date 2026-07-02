<template>
  <div class="page-container">
    <div class="page-header">
      <h2>{{ t('audit.title') }}</h2>
      <div>
        <el-input
          v-model="actionFilter"
          :placeholder="t('audit.actionFilter')"
          clearable
          style="width: 200px; margin-right: 8px"
          @change="reload"
        />
      </div>
    </div>
    <el-table :data="items" v-loading="loading" border>
      <el-table-column :label="t('audit.columns.action')" prop="action" width="220" />
      <el-table-column :label="t('audit.columns.actor')" prop="actor_name" width="140" />
      <el-table-column :label="t('audit.columns.resource')" prop="resource" width="120" />
      <el-table-column :label="t('audit.columns.target')" prop="target_id" width="160" />
      <el-table-column :label="t('audit.columns.time')" prop="created_at" width="220" />
      <el-table-column :label="t('audit.columns.payload')">
        <template #default="{ row }">
          <code style="font-size: 12px; color: var(--text-secondary)">
            {{ row.payload || '-' }}
          </code>
        </template>
      </el-table-column>
    </el-table>
    <el-pagination
      v-model:current-page="page"
      :total="total"
      :page-size="pageSize"
      layout="total, prev, pager, next"
      style="margin-top: 12px; justify-content: flex-end"
      @current-change="reload"
    />
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import { auditApi, type AuditEntry } from '@/api/audit'

const { t } = useI18n()
const auth = useAuthStore()

const items = ref<AuditEntry[]>([])
const total = ref(0)
const page = ref(1)
const pageSize = 50
const actionFilter = ref('')
const loading = ref(false)

async function reload() {
  loading.value = true
  try {
    const r = await auditApi.list(auth.token, actionFilter.value || undefined, page.value, pageSize)
    items.value = r.items
    total.value = r.total
  } finally {
    loading.value = false
  }
}
onMounted(reload)
</script>