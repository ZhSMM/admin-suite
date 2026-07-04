<template>
  <div class="page-container">
    <div class="page-header">
      <h2>{{ t('perms.title') }}</h2>
    </div>
    <el-alert :title="t('perms.help')" type="info" :closable="false" style="margin-bottom: 12px" />
    <el-table :data="perms" v-loading="loading" border>
      <el-table-column :label="t('perms.columns.code')" prop="code" width="240" />
      <el-table-column :label="t('perms.columns.resource')" prop="resource" width="120" />
      <el-table-column :label="t('perms.columns.action')" prop="action" width="120" />
      <el-table-column :label="t('perms.columns.name')" prop="name" />
      <el-table-column :label="t('perms.columns.description')" prop="description" />
    </el-table>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import { permissionsApi, type Permission } from '@/api/roles'

const { t } = useI18n()
const auth = useAuthStore()

const perms = ref<Permission[]>([])
const loading = ref(false)

onMounted(async () => {
  loading.value = true
  try {
    perms.value = await permissionsApi.list(auth.token)
  } finally {
    loading.value = false
  }
})
</script>