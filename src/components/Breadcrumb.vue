<template>
  <el-breadcrumb separator="/">
    <el-breadcrumb-item :to="{ name: 'dashboard' }">{{ t('menu.dashboard') }}</el-breadcrumb-item>
    <el-breadcrumb-item v-for="(crumb, i) in crumbs" :key="i">
      {{ t(crumb.title) }}
    </el-breadcrumb-item>
  </el-breadcrumb>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'

const route = useRoute()
const { t } = useI18n()

const crumbs = computed(() => {
  return route.matched
    .filter((m) => m.meta?.title && m.name !== 'dashboard')
    .map((m) => ({ title: String(m.meta?.title) }))
})
</script>