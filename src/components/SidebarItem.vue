<template>
  <template v-if="!node.children || node.children.length === 0">
    <el-menu-item :index="resolveIndex(node)">
      <el-icon v-if="iconName(node.icon)"><component :is="iconName(node.icon)" /></el-icon>
      <template #title>{{ node.title }}</template>
    </el-menu-item>
  </template>
  <template v-else>
    <el-sub-menu :index="resolveIndex(node)">
      <template #title>
        <el-icon v-if="iconName(node.icon)"><component :is="iconName(node.icon)" /></el-icon>
        <span>{{ node.title }}</span>
      </template>
      <template v-for="child in node.children" :key="child.id">
        <SidebarItem :node="child" />
      </template>
    </el-sub-menu>
  </template>
</template>

<script setup lang="ts">
import { resolveIndex, iconName } from './sidebar-utils'
import type { MenuNode } from '@/api/roles'

defineProps<{ node: MenuNode }>()
</script>