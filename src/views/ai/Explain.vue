<template>
  <AiToolLayout
    :title="t('ai.explain.title')"
    :input-placeholder="t('ai.explain.inputPlaceholder')"
    :output-placeholder="t('ai.explain.outputPlaceholder')"
    :run-label="t('ai.explain.run')"
    :system-prompt="t('ai.explain.system')"
    :initial-input="initialInput"
  />
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import AiToolLayout from './AiToolLayout.vue'

const { t } = useI18n()
// Allow Diagnostics → Explain handoff (router.push({ name:'ai-explain', state:{prefill} }))
const initialInput = ref<string>(
  (history.state && typeof history.state.prefill === 'string') ? history.state.prefill : ''
)
</script>