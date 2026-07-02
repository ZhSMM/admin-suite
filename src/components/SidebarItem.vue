<template>
  <template v-if="!node.children || node.children.length === 0">
    <el-menu-item :index="resolveIndex(node)">
      <el-icon v-if="iconName(node.icon)"><component :is="iconName(node.icon)" /></el-icon>
      <template #title>{{ label(node) }}</template>
    </el-menu-item>
  </template>
  <template v-else>
    <el-sub-menu :index="resolveIndex(node)">
      <template #title>
        <el-icon v-if="iconName(node.icon)"><component :is="iconName(node.icon)" /></el-icon>
        <span>{{ label(node) }}</span>
      </template>
      <template v-for="child in node.children" :key="child.id">
        <SidebarItem :node="child" />
      </template>
    </el-sub-menu>
  </template>
</template>

<script setup lang="ts">
import { resolveIndex, iconName } from './sidebar-utils'
import { useI18n } from 'vue-i18n'
import type { MenuNode } from '@/api/roles'

const props = defineProps<{ node: MenuNode }>()
const { t } = useI18n()

/**
 * Resolve the displayed title for a menu entry.
 *  - Prefer the i18n key (so locale switching works).
 *  - Fall back to the raw DB title when no key is set, or when the
 *    key is missing in the current locale.
 */
function label(node: MenuNode): string {
  const key = node.title_key
  if (key) {
    const translated = t(key)
    // vue-i18n returns the key itself when not found. Detect that.
    if (translated !== key) return translated as string
  }
  return node.title
}
</script>