<template>
  <div class="page-container notfound">
    <el-result icon="warning" :title="t('notfound.title')" :sub-title="t('notfound.desc')">
      <template #extra>
        <el-button type="primary" @click="back">{{ t('notfound.back') }}</el-button>
      </template>
    </el-result>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
const { t } = useI18n()
const router = useRouter()
function back() {
  router.replace({ name: 'dashboard' })
}
</script>