<template>
  <el-dropdown trigger="click" @command="(c: string) => locale.activate(c)">
    <el-button text :title="locale.active?.name || t('common.language')">
      <el-icon :size="20"><ChatDotRound /></el-icon>
    </el-button>
    <template #dropdown>
      <el-dropdown-menu>
        <el-dropdown-item
          v-for="it in locale.items"
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
import { useLocaleStore } from '@/stores/locale'

const { t } = useI18n()
const locale = useLocaleStore()
</script>