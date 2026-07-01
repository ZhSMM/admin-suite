<template>
  <div class="header-bar">
    <div class="left">
      <el-button text @click="$emit('toggle')">
        <el-icon :size="20">
          <Fold v-if="!collapsed" />
          <Expand v-else />
        </el-icon>
      </el-button>
      <Breadcrumb />
    </div>
    <div class="right">
      <ThemeSwitcher />
      <LanguageSwitcher />
      <el-dropdown trigger="click" @command="handleCommand">
        <span class="user-trigger">
          <el-icon><User /></el-icon>
          <span class="username">{{ auth.user?.display_name || auth.user?.username }}</span>
          <el-icon><ArrowDown /></el-icon>
        </span>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item command="logout">
              <el-icon><SwitchButton /></el-icon>
              {{ t('auth.logout') }}
            </el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import Breadcrumb from './Breadcrumb.vue'
import ThemeSwitcher from './ThemeSwitcher.vue'
import LanguageSwitcher from './LanguageSwitcher.vue'

defineProps<{ collapsed: boolean }>()
defineEmits<{ (e: 'toggle'): void }>()

const { t } = useI18n()
const auth = useAuthStore()
const router = useRouter()

function handleCommand(cmd: string) {
  if (cmd === 'logout') {
    auth.logout().then(() => router.replace({ name: 'login' }))
  }
}
</script>

<style scoped lang="scss">
.header-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  height: 100%;
  padding: 0 16px;
}
.left {
  display: flex;
  align-items: center;
  gap: 8px;
}
.right {
  display: flex;
  align-items: center;
  gap: 8px;
}
.user-trigger {
  display: flex;
  align-items: center;
  gap: 4px;
  cursor: pointer;
  color: var(--text-primary);
  padding: 0 8px;
}
.username {
  font-size: 14px;
}
</style>