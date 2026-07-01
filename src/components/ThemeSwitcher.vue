<template>
  <el-dropdown trigger="click" @command="(c: string) => theme.activate(c)">
    <el-button text :title="t('themes.active')">
      <el-icon :size="20"><Brush /></el-icon>
    </el-button>
    <template #dropdown>
      <el-dropdown-menu>
        <el-dropdown-item
          v-for="it in theme.items"
          :key="it.id"
          :command="it.code"
          :disabled="it.active"
        >
          {{ it.name }}
          <el-icon v-if="it.active" style="margin-left:6px"><Check /></el-icon>
        </el-dropdown-item>
      </el-dropdown-menu>
    </template>
  </el-dropdown>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useThemeStore } from '@/stores/theme'

const theme = useThemeStore()
const { t } = useI18n()
</script>