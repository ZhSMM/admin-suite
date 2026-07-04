<template>
  <el-button text :icon="Star" @click="store.toggleFavorite(route.path)" :title="title">
    <el-icon :size="18" :color="active ? '#e6a23c' : 'var(--text-secondary)'">
      <component :is="active ? StarFilled : Star" />
    </el-icon>
  </el-button>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { Star, StarFilled } from '@element-plus/icons-vue'
import { useI18n } from 'vue-i18n'
import { useRecentStore } from '@/stores/recent'

const route = useRoute()
const store = useRecentStore()
const { t } = useI18n()

const active = computed(() => store.isFavorite(route.path))
const title = computed(() => (active.value ? t('recents.unpin') : t('recents.pin')))
</script>