<template>
  <div class="sidebar-wrap">
    <div class="sidebar-logo">
      <span v-if="!collapsed">{{ t('app.name') }}</span>
      <span v-else>AS</span>
    </div>
    <el-menu
      :default-active="activeKey"
      :collapse="collapsed"
      background-color="var(--bg-sidebar, #001529)"
      text-color="var(--bg-sidebar-text, #ffffff)"
      active-text-color="var(--primary-color, #409eff)"
      router
      unique-opened
    >
      <template v-for="top in topMenus" :key="top.id">
        <SidebarItem :node="top" />
      </template>
    </el-menu>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useMenuStore } from '@/stores/menu'
import { useAuthStore } from '@/stores/auth'
import SidebarItem from './SidebarItem.vue'
import type { MenuNode } from '@/api/roles'

const props = defineProps<{ collapsed: boolean }>()

const route = useRoute()
const menuStore = useMenuStore()
const auth = useAuthStore()
const { t } = useI18n()

// Top-level menus the user can see, optionally filtered by permission.
const topMenus = computed<MenuNode[]>(() => {
  const all = menuStore.tree
  return all
    .filter((m) => {
      if (!m.permission_code) return true
      return auth.hasPermission(m.permission_code)
    })
    .map((m) => filterByPermission(m))
})

function filterByPermission(node: MenuNode): MenuNode {
  return {
    ...node,
    children: node.children.filter((c) => {
      if (!c.permission_code) return true
      return auth.hasPermission(c.permission_code)
    })
  }
}

const activeKey = computed(() => route.path)
</script>

<style scoped lang="scss">
.sidebar-wrap {
  height: 100vh;
  display: flex;
  flex-direction: column;
}

.sidebar-logo {
  height: var(--header-height);
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-weight: 600;
  font-size: 18px;
  letter-spacing: 0.5px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}

:deep(.el-menu) {
  border-right: none;
}
</style>