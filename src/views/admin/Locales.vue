<template>
  <div class="page-container">
    <div class="page-header">
      <h2>{{ t('locales.title') }}</h2>
      <div>
        <el-button @click="triggerImport">
          <el-icon><Upload /></el-icon>
          {{ t('locales.import') }}
        </el-button>
        <input ref="fileInput" type="file" accept="application/json" hidden @change="onFile" />
      </div>
    </div>

    <el-alert :title="t('locales.importHelp')" type="info" :closable="false" style="margin-bottom: 12px" />

    <el-table :data="items" v-loading="loading" border>
      <el-table-column label="Code" prop="code" width="160" />
      <el-table-column label="Name" prop="name" width="180" />
      <el-table-column label="Messages" width="120">
        <template #default="{ row }">{{ messageCount(row) }}</template>
      </el-table-column>
      <el-table-column :label="t('common.source')" width="120">
        <template #default="{ row }">
          <el-tag :type="row.source === 'builtin' ? 'info' : 'success'" size="small">
            {{ row.source }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('common.actions')" width="240">
        <template #default="{ row }">
          <el-button text type="primary" :disabled="row.active" @click="activate(row.code)">
            {{ t('themes.activate') }}
          </el-button>
          <el-button
            text
            type="danger"
            :disabled="row.built_in || row.active"
            @click="remove(row)"
          >
            {{ t('common.delete') }}
          </el-button>
        </template>
      </el-table-column>
    </el-table>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import { useLocaleStore } from '@/stores/locale'
import { resourcesApi, type Resource } from '@/api/resources'

const { t } = useI18n()
const auth = useAuthStore()
const locale = useLocaleStore()

const items = ref<Resource[]>([])
const loading = ref(false)

async function reload() {
  loading.value = true
  try {
    const r = await resourcesApi.list(auth.token, 'locale')
    items.value = r.items
  } finally {
    loading.value = false
  }
}
onMounted(reload)

function messageCount(r: Resource) {
  try {
    const p = JSON.parse(r.content)
    return Object.keys(p.messages || {}).length
  } catch {
    return 0
  }
}

async function activate(code: string) {
  await locale.activate(code)
  await reload()
  ElMessage.success(t('common.success'))
}

async function remove(row: Resource) {
  await ElMessageBox.confirm(t('common.confirmDelete'), '', { type: 'warning' })
  await locale.remove(row.id)
  await reload()
  ElMessage.success(t('common.success'))
}

const fileInput = ref<HTMLInputElement>()
function triggerImport() {
  fileInput.value?.click()
}
async function onFile(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  const text = await file.text()
  try {
    await locale.importFromJson(text)
    await reload()
    ElMessage.success(t('common.success'))
  } catch (err) {
    ElMessage.error((err as Error).message)
  } finally {
    input.value = ''
  }
}
</script>