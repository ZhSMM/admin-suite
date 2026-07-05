<template>
  <AiToolLayout
    :title="t('ai.summarize.title')"
    :input-placeholder="t('ai.summarize.inputPlaceholder')"
    :output-placeholder="t('ai.summarize.outputPlaceholder')"
    :run-label="t('ai.summarize.run')"
    :system-prompt="t('ai.summarize.system')"
  />
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import AiToolLayout from './AiToolLayout.vue'

const { t } = useI18n()
</script>