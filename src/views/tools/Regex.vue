<template>
  <div class="page-container">
    <div class="page-header">
      <h2>{{ t('tools.regex.title') }}</h2>
    </div>

    <el-card shadow="never">
      <el-form label-width="80px">
        <el-form-item :label="t('tools.regex.pattern')">
          <el-input v-model="pattern" placeholder="^(\\w+)\\s+(\\d+)$" @input="run" />
        </el-form-item>
        <el-form-item :label="t('tools.regex.flags')">
          <el-checkbox-group v-model="flagList" @change="onFlagsChange">
            <el-checkbox value="g">g (global)</el-checkbox>
            <el-checkbox value="i">i (case-insensitive)</el-checkbox>
            <el-checkbox value="m">m (multiline)</el-checkbox>
            <el-checkbox value="s">s (dotAll)</el-checkbox>
            <el-checkbox value="u">u (unicode)</el-checkbox>
            <el-checkbox value="y">y (sticky)</el-checkbox>
          </el-checkbox-group>
        </el-form-item>
        <el-form-item :label="t('tools.regex.input')">
          <el-input v-model="text" type="textarea" :rows="8" @input="run" />
        </el-form-item>
        <el-form-item :label="t('tools.regex.replace')">
          <el-input v-model="replacement" :placeholder="t('tools.regex.replacePlaceholder')" />
          <el-button size="small" type="primary" :icon="DocumentCopy" @click="copyReplacement" style="margin-left: 8px">
            {{ t('common.copy') }}
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-row :gutter="16" style="margin-top: 16px">
      <el-col :span="12">
        <el-card shadow="never">
          <template #header>
            <div class="card-header">
              <strong>{{ t('tools.regex.matches') }} ({{ matches.length }})</strong>
              <span v-if="error" class="err">{{ error }}</span>
            </div>
          </template>
          <div v-if="!matches.length && !error" class="empty">
            {{ t('tools.regex.noMatch') }}
          </div>
          <el-table :data="matches" :show-header="false" max-height="400">
            <el-table-column label="#" type="index" width="50" />
            <el-table-column :label="t('tools.regex.match')">
              <template #default="{ row }">
                <code>{{ row.match }}</code>
              </template>
            </el-table-column>
            <el-table-column :label="t('tools.regex.index')" width="80">
              <template #default="{ row }">{{ row.index }}</template>
            </el-table-column>
            <el-table-column :label="t('tools.regex.groups')">
              <template #default="{ row }">
                <code v-for="(g, i) in row.groups" :key="i" class="grp">{{ g ?? '∅' }}</code>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-col>

      <el-col :span="12">
        <el-card shadow="never">
          <template #header><strong>{{ t('tools.regex.preview') }}</strong></template>
          <pre class="preview">{{ previewText }}</pre>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { DocumentCopy } from '@element-plus/icons-vue'

const { t } = useI18n()

const pattern = ref('\\b(\\w+)\\b')
const flagList = ref<string[]>(['g'])
const text = ref('The quick brown fox jumps over the lazy dog.')
const replacement = ref('[$1]')

const matches = ref<{ match: string; index: number; groups: (string | undefined)[] }[]>([])
const error = ref('')

const flags = computed(() => flagList.value.join(''))

function run() {
  matches.value = []
  error.value = ''
  if (!pattern.value) return
  try {
    const re = new RegExp(pattern.value, flags.value)
    if (flagList.value.includes('g')) {
      let m: RegExpExecArray | null
      while ((m = re.exec(text.value)) !== null) {
        matches.value.push({
          match: m[0],
          index: m.index,
          groups: m.slice(1)
        })
        if (m.index === re.lastIndex) re.lastIndex++ // avoid infinite loop on empty matches
      }
    } else {
      const m = re.exec(text.value)
      if (m) {
        matches.value.push({
          match: m[0],
          index: m.index,
          groups: m.slice(1)
        })
      }
    }
  } catch (e: any) {
    error.value = e.message
  }
}

function onFlagsChange() {
  // global is required for matchAll-style listing; auto-add if user wants /g semantics
  run()
}

const previewText = computed(() => {
  if (error.value) return text.value
  if (!pattern.value) return text.value
  try {
    const re = new RegExp(pattern.value, 'g' + flags.value.replace('g', ''))
    let out = ''
    let lastIdx = 0
    let m: RegExpExecArray | null
    while ((m = re.exec(text.value)) !== null) {
      out += text.value.slice(lastIdx, m.index)
      const replaced = replacement.value.replace(/\$(\d)/g, (_, n) => m![parseInt(n)] ?? '')
      out += `<<<${replaced}>>>`
      lastIdx = m.index + m[0].length
      if (m.index === re.lastIndex) re.lastIndex++
    }
    out += text.value.slice(lastIdx)
    return out
  } catch {
    return text.value
  }
})

async function copyReplacement() {
  if (!previewText.value) return
  try {
    await navigator.clipboard.writeText(previewText.value)
    ElMessage.success(t('common.copySuccess'))
  } catch {
    ElMessage.error(t('common.copyFailed'))
  }
}

run()
</script>

<style scoped lang="scss">
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.err {
  color: var(--danger-color);
  font-size: 12px;
}
.empty {
  color: var(--text-secondary);
  padding: 24px;
  text-align: center;
}
.preview {
  background: var(--bg-secondary);
  padding: 12px;
  border-radius: 6px;
  font-family: 'JetBrains Mono', Consolas, monospace;
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 400px;
  overflow: auto;
}
.grp {
  background: var(--bg-secondary);
  padding: 1px 6px;
  margin-right: 4px;
  border-radius: 4px;
  font-size: 12px;
}
</style>