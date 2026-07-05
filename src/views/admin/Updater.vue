<template>
  <div class="page-container">
    <div class="page-header">
      <h2>{{ t('updater.title') }}</h2>
      <el-button
        :icon="Refresh"
        :loading="updater.status === 'checking'"
        @click="onCheck"
      >
        {{ updater.status === 'checking' ? t('updater.checking') : t('updater.check') }}
      </el-button>
    </div>

    <el-card v-if="updater.manifest || updater.error" shadow="never" class="result-card">
      <el-alert
        v-if="updater.status === 'uptodate'"
        type="success"
        :title="t('updater.upToDate')"
        :closable="false"
        show-icon
      />
      <el-alert
        v-else-if="updater.status === 'available' || updater.status === 'downloading'"
        type="info"
        :title="t('updater.available')"
        :closable="false"
        show-icon
      >
        <div class="info-grid">
          <div>
            <span class="muted">{{ t('updater.current') }}:</span>
            <code>{{ updater.manifest?.current_version }}</code>
          </div>
          <div>
            <span class="muted">{{ t('updater.latest') }}:</span>
            <code class="latest">{{ updater.manifest?.latest_version }}</code>
          </div>
        </div>
        <pre v-if="updater.manifest?.body" class="body">{{ updater.manifest.body }}</pre>
        <el-button
          type="primary"
          :loading="updater.status === 'downloading'"
          style="margin-top: 12px"
          @click="onInstall"
        >
          {{ t('updater.download') }}
        </el-button>
      </el-alert>
      <el-alert
        v-else-if="updater.status === 'ready'"
        type="warning"
        :title="t('updater.ready')"
        :closable="false"
        show-icon
      />
      <el-alert
        v-else-if="updater.status === 'error'"
        type="error"
        :title="t('updater.error')"
        :description="updater.error ?? ''"
        :closable="false"
        show-icon
      />
    </el-card>

    <el-empty
      v-else
      :description="t('updater.disabled')"
      :image-size="80"
    />
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { Refresh } from '@element-plus/icons-vue'
import { useAuthStore } from '@/stores/auth'
import { useUpdaterStore } from '@/stores/updater'

const { t } = useI18n()
const updater = useUpdaterStore()
const auth = useAuthStore()

const onCheck = () => updater.check(auth.token || '')

const onInstall = async () => {
  await updater.install(auth.token || '')
  if (updater.status === 'ready') {
    ElMessage.success(t('updater.ready'))
  } else if (updater.status === 'error') {
    ElMessage.error(updater.error || t('updater.error'))
  }
}

onMounted(() => {
  // Auto-check once on mount if user has the perm.
  if (auth.hasPermission('updater:check')) {
    onCheck()
  }
})
</script>

<style scoped>
.result-card {
  margin-bottom: 12px;
}
.info-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
  margin-top: 8px;
}
.muted {
  color: var(--el-text-color-secondary);
  margin-right: 6px;
  font-size: 12px;
}
code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  background: var(--el-fill-color-light);
  padding: 1px 6px;
  border-radius: 3px;
  font-size: 12px;
}
code.latest {
  background: var(--el-color-success-light-9);
  color: var(--el-color-success-dark-2);
  font-weight: 600;
}
.body {
  background: var(--el-fill-color-light);
  padding: 12px;
  border-radius: 4px;
  margin-top: 12px;
  max-height: 240px;
  overflow: auto;
  white-space: pre-wrap;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 12px;
}
</style>