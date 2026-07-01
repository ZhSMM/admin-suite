<template>
  <el-container class="layout-root">
    <el-aside :width="collapsed ? '64px' : '220px'" class="layout-aside">
      <Sidebar :collapsed="collapsed" />
    </el-aside>
    <el-container>
      <el-header class="layout-header">
        <HeaderBar
          :collapsed="collapsed"
          @toggle="collapsed = !collapsed"
        />
      </el-header>
      <el-main class="layout-main">
        <router-view v-slot="{ Component }">
          <transition name="fade" mode="out-in">
            <component :is="Component" />
          </transition>
        </router-view>
      </el-main>
    </el-container>
  </el-container>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import Sidebar from '@/components/Sidebar.vue'
import HeaderBar from '@/components/HeaderBar.vue'

const collapsed = ref(false)
</script>

<style scoped lang="scss">
.layout-root {
  height: 100vh;
}

.layout-aside {
  background-color: var(--bg-sidebar, #001529);
  color: var(--bg-sidebar-text, #ffffff);
  transition: width 0.2s;
  overflow-x: hidden;
}

.layout-header {
  height: var(--header-height);
  line-height: var(--header-height);
  background-color: var(--bg-primary, #ffffff);
  border-bottom: 1px solid var(--border-color, #e5e6eb);
  padding: 0;
}

.layout-main {
  background-color: var(--bg-secondary, #f5f7fa);
  padding: 0;
  overflow: auto;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.15s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>