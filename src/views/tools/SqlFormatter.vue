<template>
  <div class="page-container">
    <div class="page-header">
      <h2>{{ t('tools.sql.title') }}</h2>
      <el-space>
        <el-button :icon="DocumentCopy" @click="copy">{{ t('common.copy') }}</el-button>
        <el-button :icon="Delete" @click="clear">{{ t('common.delete') }}</el-button>
      </el-space>
    </div>

    <el-row :gutter="16">
      <el-col :span="12">
        <el-card shadow="never">
          <template #header>
            <div class="card-header">
              <strong>{{ t('tools.sql.input') }}</strong>
              <el-space>
                <el-select v-model="lang" size="small" style="width: 140px">
                  <el-option v-for="l in langs" :key="l" :label="l" :value="l" />
                </el-select>
                <el-select v-model.number="indent" size="small" style="width: 90px">
                  <el-option :value="2" label="2" />
                  <el-option :value="4" label="4" />
                  <el-option value="tab" label="Tab" />
                </el-select>
                <el-checkbox v-model="uppercase" size="small">{{ t('tools.sql.uppercase') }}</el-checkbox>
              </el-space>
            </div>
          </template>
          <el-input
            v-model="input"
            type="textarea"
            :rows="20"
            spellcheck="false"
            :placeholder="t('tools.sql.placeholder')"
            @input="recompute"
          />
          <div v-if="error" class="err">
            <el-icon><CircleClose /></el-icon> {{ error }}
          </div>
          <div class="actions">
            <el-button type="primary" :icon="MagicStick" @click="format">{{ t('tools.sql.format') }}</el-button>
            <el-button :icon="Sort" @click="minify">{{ t('tools.sql.minify') }}</el-button>
          </div>
        </el-card>
      </el-col>

      <el-col :span="12">
        <el-card shadow="never">
          <template #header><strong>{{ t('tools.sql.output') }}</strong></template>
          <pre class="preview">{{ output || '—' }}</pre>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { format as formatSql } from 'sql-formatter'
import {
  MagicStick,
  Sort,
  DocumentCopy,
  Delete,
  CircleClose
} from '@element-plus/icons-vue'

const { t } = useI18n()

const langs = ['sql', 'mysql', 'postgresql', 'sqlite', 'bigquery', 'redshift', 'spark', 'tsql', 'plsql', 'n1ql']
const lang = ref('sql')
const indent = ref<number | 'tab'>(2)
const uppercase = ref(true)
const input = ref('select id, name, email from users where status = \'active\' and created_at > \'2025-01-01\' order by created_at desc limit 10')
const error = ref('')

const output = computed(() => {
  if (!input.value.trim()) return ''
  error.value = ''
  try {
    return formatSql(input.value, {
      language: lang.value as any,
      tabWidth: indent.value === 'tab' ? 2 : indent.value,
      keywordCase: uppercase.value ? 'upper' : 'preserve',
      indentStyle: 'standard'
    })
  } catch (e: any) {
    error.value = e.message
    return ''
  }
})

function format() {
  // force re-format from current state
  try {
    const out = formatSql(input.value, {
      language: lang.value as any,
      tabWidth: indent.value === 'tab' ? 2 : indent.value,
      keywordCase: uppercase.value ? 'upper' : 'preserve'
    })
    input.value = out
    error.value = ''
    ElMessage.success(t('common.success'))
  } catch (e: any) {
    error.value = e.message
    ElMessage.error(t('tools.sql.parseError'))
  }
}

function minify() {
  if (!input.value.trim()) return
  try {
    // Strip line comments, block comments, collapse whitespace.
    let s = input.value
      .replace(/\/\*[\s\S]*?\*\//g, ' ')
      .replace(/--.*$/gm, ' ')
      .replace(/\s+/g, ' ')
      .trim()
    input.value = s
    error.value = ''
    ElMessage.success(t('common.success'))
  } catch (e: any) {
    error.value = e.message
  }
}

function recompute() {
  // output is computed; this just clears stale errors
  if (!input.value.trim()) error.value = ''
}

function clear() {
  input.value = ''
  error.value = ''
}

async function copy() {
  if (!output.value) return
  try {
    await navigator.clipboard.writeText(output.value)
    ElMessage.success(t('common.copySuccess'))
  } catch {
    ElMessage.error(t('common.copyFailed'))
  }
}
</script>

<style scoped lang="scss">
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.err {
  margin-top: 8px;
  color: var(--danger-color);
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 13px;
}
.actions {
  margin-top: 12px;
  display: flex;
  gap: 8px;
}
.preview {
  background: var(--bg-secondary);
  padding: 12px;
  border-radius: 6px;
  font-family: 'JetBrains Mono', Consolas, monospace;
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 600px;
  overflow: auto;
}
</style>