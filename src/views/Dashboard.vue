<template>
  <div class="page-container">
    <el-row :gutter="16">
      <el-col :span="16">
        <el-card>
          <template #header>
            <strong>{{ t('dashboard.title') }}</strong>
          </template>
          <h3>{{ t('dashboard.welcome') }}, {{ auth.user?.display_name }}</h3>
          <p style="color: var(--text-secondary)">{{ t('dashboard.desc') }}</p>
        </el-card>
      </el-col>
      <el-col :span="8">
        <el-card>
          <template #header>
            <strong>{{ t('dashboard.info') }}</strong>
          </template>
          <el-descriptions :column="1" size="small">
            <el-descriptions-item :label="t('dashboard.dataDir')">
              {{ info?.data_dir }}
            </el-descriptions-item>
            <el-descriptions-item :label="t('dashboard.dbPath')">
              {{ info?.db_path }}
            </el-descriptions-item>
            <el-descriptions-item :label="t('dashboard.migrationsDir')">
              {{ info?.migrations_dir }}
            </el-descriptions-item>
            <el-descriptions-item :label="t('dashboard.defaultAdmin')">
              <code>{{ info?.default_admin.username }}</code>
              /
              <code>{{ info?.default_admin.password }}</code>
            </el-descriptions-item>
          </el-descriptions>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import { appApi, type AppInfo } from '@/api/auth'

const { t } = useI18n()
const auth = useAuthStore()
const info = ref<AppInfo | null>(null)

onMounted(async () => {
  try {
    info.value = await appApi.info()
  } catch (e) {
    // ignore — UI still renders
  }
})
</script>