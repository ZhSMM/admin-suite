<template>
  <div class="page-container">
    <div class="page-header">
      <h2>{{ t('backups.title') }}</h2>
      <el-button type="primary" :loading="creating" :icon="Plus" @click="createNow">
        {{ t('backups.createNow') }}
      </el-button>
    </div>

    <el-alert :title="t('backups.help')" type="info" :closable="false" style="margin-bottom: 12px" />

    <el-table :data="items" v-loading="loading" border>
      <el-table-column :label="t('backups.columns.name')" prop="name" min-width="260" />
      <el-table-column :label="t('backups.columns.size')" width="120">
        <template #default="{ row }">{{ formatBytes(row.size_bytes) }}</template>
      </el-table-column>
      <el-table-column :label="t('backups.columns.createdAt')" prop="created_at" width="220" />
      <el-table-column :label="t('common.actions')" width="240" fixed="right">
        <template #default="{ row }">
          <el-button text type="danger" :icon="Delete" @click="onDelete(row)">
            {{ t('common.delete') }}
          </el-button>
          <el-button text type="primary" :icon="RefreshRight" @click="onRestore(row)">
            {{ t('backups.restore') }}
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- Restore confirmation: warns that restart is required -->
    <el-dialog
      v-model="restoreDialog.open"
      :title="t('backups.restoreDialog')"
      width="520"
    >
      <el-alert :title="t('backups.restoreWarning')" type="warning" :closable="false" />
      <p style="margin-top: 12px">
        <strong>{{ restoreDialog.name }}</strong>
      </p>
      <p style="color: var(--text-secondary); font-size: 12px">{{ restoreDialog.path }}</p>
      <template #footer>
        <el-button @click="restoreDialog.open = false">{{ t('common.cancel') }}</el-button>
        <el-button
          type="danger"
          :icon="RefreshRight"
          :loading="restoreDialog.loading"
          @click="confirmRestore"
        >
          {{ t('backups.restore') }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Delete, Plus, RefreshRight } from '@element-plus/icons-vue'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import { backupsApi, formatBytes, type BackupInfo } from '@/api/backups'

const { t } = useI18n()
const auth = useAuthStore()

const items = ref<BackupInfo[]>([])
const loading = ref(false)
const creating = ref(false)

const restoreDialog = reactive<{ open: boolean; name: string; path: string; loading: boolean }>({
  open: false,
  name: '',
  path: '',
  loading: false
})

async function reload() {
  loading.value = true
  try {
    items.value = await backupsApi.list(auth.token)
  } catch (e) {
    ElMessage.error((e as Error).message)
  } finally {
    loading.value = false
  }
}

async function createNow() {
  creating.value = true
  try {
    const info = await backupsApi.create(auth.token)
    ElMessage.success(t('backups.createSuccess', { name: info.name }))
    await reload()
  } catch (e) {
    ElMessage.error((e as Error).message)
  } finally {
    creating.value = false
  }
}

async function onDelete(row: BackupInfo) {
  try {
    await ElMessageBox.confirm(t('backups.delete.confirm', { name: row.name }), '', {
      type: 'warning'
    })
    await backupsApi.delete(auth.token, row.name)
    ElMessage.success(t('common.success'))
    await reload()
  } catch {
    /* user cancelled */
  }
}

function onRestore(row: BackupInfo) {
  restoreDialog.name = row.name
  restoreDialog.path = row.path
  restoreDialog.open = true
}

async function confirmRestore() {
  restoreDialog.loading = true
  try {
    const r = await backupsApi.restore(auth.token, restoreDialog.name)
    restoreDialog.open = false
    if (r.restart_required) {
      ElMessage.warning(t('backups.restoreScheduled'))
    } else {
      ElMessage.success(t('backups.restoreDone'))
    }
  } catch (e) {
    ElMessage.error((e as Error).message)
  } finally {
    restoreDialog.loading = false
  }
}

onMounted(reload)
</script>