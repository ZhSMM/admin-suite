<template>
  <AiToolLayout
    :title="t('ai.translate.title')"
    :input-placeholder="t('ai.translate.inputPlaceholder')"
    :output-placeholder="t('ai.translate.outputPlaceholder')"
    :run-label="t('ai.translate.run')"
    :system-prompt="(text) => t('ai.translate.system', { text })"
  />
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import AiToolLayout from './AiToolLayout.vue'

const { t } = useI18n()
</script>