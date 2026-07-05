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
        <div v-if="updaterBanner.visible" class="updater-banner">
          <el-alert
            type="info"
            show-icon
            :closable="true"
            @close="updater.dismiss()"
          >
            <template #default>
              <div class="banner-row">
                <span class="banner-msg">
                  {{ t('updater.available') }}
                  <code class="banner-ver">{{ updater.manifest?.latest_version }}</code>
                </span>
                <el-button type="primary" size="small" @click="goUpdater">
                  {{ t('updater.download') }}
                </el-button>
              </div>
            </template>
          </el-alert>
        </div>
        <router-view v-slot="{ Component }">
          <transition name="fade" mode="out-in">
            <component :is="Component" />
          </transition>
        </router-view>
      </el-main>
    </el-container>
    <CommandPalette />
  </el-container>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import Sidebar from '@/components/Sidebar.vue'
import HeaderBar from '@/components/HeaderBar.vue'
import CommandPalette from '@/components/CommandPalette.vue'
import { useAuthStore } from '@/stores/auth'
import { useUpdaterStore } from '@/stores/updater'

const { t } = useI18n()
const router = useRouter()
const auth = useAuthStore()
const updater = useUpdaterStore()
const collapsed = ref(false)

// Show banner only when an update is available AND the user has the perm
// (so non-admin users never see it). Dismissable per-session via dismiss().
const updaterBanner = computed(() => ({
  visible: updater.status === 'available' && auth.hasPermission('updater:check')
}))

const goUpdater = () => router.push('/system/updater')

onMounted(() => {
  // Background auto-check once per session.  We only run if the updater
  // perm is held — the Updater page itself re-checks on mount.
  if (auth.isAuthenticated && auth.hasPermission('updater:check') && updater.status === 'idle') {
    void updater.check(auth.token || '')
  }
})
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