<template>
  <div class="page-container">
    <div class="page-header">
      <h2>{{ t('audit.title') }}</h2>
      <el-button :icon="Refresh" @click="reload">{{ t('common.refresh') }}</el-button>
    </div>

    <!-- Filter bar -->
    <el-card shadow="never" style="margin-bottom: 12px">
      <el-form label-width="80px" size="small">
        <el-row :gutter="12">
          <el-col :span="6">
            <el-form-item :label="t('audit.filter.action')">
              <el-input
                v-model="filter.action"
                :placeholder="t('audit.filter.actionPlaceholder')"
                clearable
              />
            </el-form-item>
          </el-col>
          <el-col :span="6">
            <el-form-item :label="t('audit.filter.actor')">
              <el-input
                v-model="filter.actor_id"
                :placeholder="t('audit.filter.actorPlaceholder')"
                clearable
              />
            </el-form-item>
          </el-col>
          <el-col :span="6">
            <el-form-item :label="t('audit.filter.resource')">
              <el-input
                v-model="filter.resource"
                :placeholder="t('audit.filter.resourcePlaceholder')"
                clearable
              />
            </el-form-item>
          </el-col>
          <el-col :span="6">
            <el-form-item :label="t('audit.filter.payload')">
              <el-input
                v-model="filter.payload_search"
                :placeholder="t('audit.filter.payloadPlaceholder')"
                clearable
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="12">
          <el-col :span="6">
            <el-form-item :label="t('audit.filter.from')">
              <el-date-picker
                v-model="fromDate"
                type="datetime"
                :placeholder="t('audit.filter.fromPlaceholder')"
                style="width: 100%"
                value-format="YYYY-MM-DDTHH:mm:ss[Z]"
                @change="onFromChange"
              />
            </el-form-item>
          </el-col>
          <el-col :span="6">
            <el-form-item :label="t('audit.filter.to')">
              <el-date-picker
                v-model="toDate"
                type="datetime"
                :placeholder="t('audit.filter.toPlaceholder')"
                style="width: 100%"
                value-format="YYYY-MM-DDTHH:mm:ss[Z]"
                @change="onToChange"
              />
            </el-form-item>
          </el-col>
          <el-col :span="6">
            <el-form-item :label="t('audit.filter.preset')">
              <el-radio-group v-model="preset" @change="onPreset">
                <el-radio-button value="1h">{{ t('audit.filter.preset1h') }}</el-radio-button>
                <el-radio-button value="24h">{{ t('audit.filter.preset24h') }}</el-radio-button>
                <el-radio-button value="7d">{{ t('audit.filter.preset7d') }}</el-radio-button>
              </el-radio-group>
            </el-form-item>
          </el-col>
          <el-col :span="6">
            <el-form-item label=" ">
              <el-button @click="resetFilters" plain>{{ t('audit.filter.reset') }}</el-button>
            </el-form-item>
          </el-col>
        </el-row>
      </el-form>
    </el-card>

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
import { onMounted, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Refresh } from '@element-plus/icons-vue'
import { useAuthStore } from '@/stores/auth'
import { auditApi, type AuditEntry, type AuditFilter } from '@/api/audit'

const { t } = useI18n()
const auth = useAuthStore()

const items = ref<AuditEntry[]>([])
const total = ref(0)
const page = ref(1)
const pageSize = 50
const loading = ref(false)

const filter = reactive<AuditFilter>({
  action: '',
  actor_id: '',
  resource: '',
  payload_search: '',
  from: '',
  to: ''
})

// Date-picker bound values are strings (ISO without Z).  We tag the Z on
// the way out so the backend can compare lexically against `created_at`.
const fromDate = ref<string>('')
const toDate = ref<string>('')
const preset = ref<'1h' | '24h' | '7d' | ''>('')

let timer: ReturnType<typeof setTimeout> | null = null

function scheduleReload() {
  if (timer) clearTimeout(timer)
  timer = setTimeout(() => {
    page.value = 1
    reload()
  }, 250)
}

async function reload() {
  loading.value = true
  try {
    const r = await auditApi.list(auth.token, filter, page.value, pageSize)
    items.value = r.items
    total.value = r.total
  } finally {
    loading.value = false
  }
}

function resetFilters() {
  filter.action = ''
  filter.actor_id = ''
  filter.resource = ''
  filter.payload_search = ''
  filter.from = ''
  filter.to = ''
  fromDate.value = ''
  toDate.value = ''
  preset.value = ''
  page.value = 1
  reload()
}

function onFromChange(v: string | null) {
  filter.from = v ? `${v}Z` : ''
  preset.value = ''
  scheduleReload()
}

function onToChange(v: string | null) {
  filter.to = v ? `${v}Z` : ''
  preset.value = ''
  scheduleReload()
}

function onPreset(v: '1h' | '24h' | '7d' | '') {
  if (!v) return
  const now = Date.now()
  const span = v === '1h' ? 3600_000 : v === '24h' ? 86_400_000 : 7 * 86_400_000
  const from = new Date(now - span)
  const to = new Date(now)
  filter.from = from.toISOString().replace(/\.\d{3}Z$/, 'Z')
  filter.to = to.toISOString().replace(/\.\d{3}Z$/, 'Z')
  fromDate.value = filter.from.replace(/Z$/, '')
  toDate.value = filter.to.replace(/Z$/, '')
  scheduleReload()
}

watch(
  () => [filter.action, filter.actor_id, filter.resource, filter.payload_search],
  scheduleReload
)

onMounted(reload)
</script>