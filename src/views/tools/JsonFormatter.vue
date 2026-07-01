<template>
  <div class="page-container">
    <div class="page-header">
      <h2>{{ t('tools.json.title') }}</h2>
      <div class="actions">
        <el-button :icon="Folder" @click="loadFile">{{ t('tools.json.loadFile') }}</el-button>
        <input ref="fileInput" type="file" accept="application/json,.json" hidden @change="onFile" />
        <el-button :icon="DocumentCopy" @click="copy">{{ t('common.copy') }}</el-button>
        <el-button :icon="Delete" @click="clear">{{ t('common.delete') }}</el-button>
      </div>
    </div>

    <el-row :gutter="16">
      <el-col :span="12">
        <el-card shadow="never">
          <template #header>
            <div class="card-header">
              <strong>{{ t('tools.json.input') }}</strong>
              <div class="hint" :class="{ err: !!error }">
                {{ error || t('tools.json.status', { lines: lineCount }) }}
              </div>
            </div>
          </template>
          <el-input
            v-model="input"
            type="textarea"
            :rows="20"
            spellcheck="false"
            :placeholder="t('tools.json.placeholder')"
            @input="recompute"
          />
          <div class="actions" style="margin-top: 12px">
            <el-button type="primary" :icon="MagicStick" @click="format">{{ t('tools.json.format') }}</el-button>
            <el-button @click="minify">{{ t('tools.json.minify') }}</el-button>
            <el-space>
              <span>{{ t('tools.json.indent') }}</span>
              <el-select v-model="indent" style="width: 100px">
                <el-option :value="2" label="2" />
                <el-option :value="4" label="4" />
                <el-option value="tab" label="Tab" />
              </el-select>
            </el-space>
            <el-checkbox v-model="sortKeys">{{ t('tools.json.sortKeys') }}</el-checkbox>
          </div>
        </el-card>
      </el-col>

      <el-col :span="12">
        <el-card shadow="never">
          <template #header>
            <div class="card-header">
              <strong>{{ t('tools.json.tree') }}</strong>
              <el-radio-group v-model="view" size="small">
                <el-radio-button value="tree">{{ t('tools.json.viewTree') }}</el-radio-button>
                <el-radio-button value="text">{{ t('tools.json.viewText') }}</el-radio-button>
              </el-radio-group>
            </div>
          </template>

          <div v-if="view === 'tree'" class="tree-pane">
            <el-tree
              v-if="treeData.length"
              :data="treeData"
              :props="{ label: 'label', children: 'children' }"
              node-key="id"
              default-expand-all
            >
              <template #default="{ node, data }">
                <span class="tree-row">
                  <el-tag
                    :type="data.kind === 'object' ? 'primary' : data.kind === 'array' ? 'success' : 'info'"
                    size="small"
                  >
                    {{ data.kind }}
                  </el-tag>
                  <strong>{{ data.label }}</strong>
                  <span v-if="!data.isContainer" class="preview">{{ data.preview }}</span>
                </span>
              </template>
            </el-tree>
            <el-empty v-else :description="t('tools.json.empty')" />
          </div>

          <pre v-else class="pretty">{{ formatted || '—' }}</pre>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  ElMessage,
  type UploadRawFile
} from 'element-plus'
import {
  MagicStick,
  DocumentCopy,
  Folder,
  Delete
} from '@element-plus/icons-vue'

const { t } = useI18n()

const input = ref('')
const error = ref('')
const indent = ref<number | 'tab'>(2)
const sortKeys = ref(false)
const view = ref<'tree' | 'text'>('tree')

const formatted = computed(() => formatJson(input.value))

const lineCount = computed(() => (input.value ? input.value.split('\n').length : 0))

const treeData = computed(() => {
  if (error.value) return []
  try {
    if (!input.value.trim()) return []
    const parsed = JSON.parse(input.value)
    return [toTree(parsed, '$')]
  } catch (e: any) {
    return []
  }
})

function formatJson(raw: string): string {
  if (!raw.trim()) return ''
  try {
    const parsed = JSON.parse(raw)
    return stringify(parsed)
  } catch (e: any) {
    return ''
  }
}

function stringify(value: unknown): string {
  const replacer = sortKeys.value
    ? (_k: string, v: unknown) => {
        if (v && typeof v === 'object' && !Array.isArray(v)) {
          const sorted: Record<string, unknown> = {}
          for (const k of Object.keys(v as Record<string, unknown>).sort()) {
            sorted[k] = (v as Record<string, unknown>)[k]
          }
          return sorted
        }
        return v
      }
    : undefined
  const ind = indent.value === 'tab' ? '\t' : indent.value
  return JSON.stringify(value, replacer as any, ind)
}

function toTree(value: unknown, key: string | number): any {
  const id = String(Math.random()).slice(2)
  if (value === null) {
    return { id, label: key, kind: 'null', preview: 'null', isContainer: false }
  }
  if (Array.isArray(value)) {
    return {
      id,
      label: key,
      kind: 'array',
      isContainer: true,
      children: value.map((v, i) => toTree(v, i))
    }
  }
  if (typeof value === 'object') {
    const obj = value as Record<string, unknown>
    return {
      id,
      label: key,
      kind: 'object',
      isContainer: true,
      children: Object.entries(obj).map(([k, v]) => toTree(v, k))
    }
  }
  const preview = typeof value === 'string' ? `"${value}"` : String(value)
  return {
    id,
    label: key,
    kind: typeof value,
    preview,
    isContainer: false
  }
}

function format() {
  if (!input.value.trim()) return
  try {
    input.value = stringify(JSON.parse(input.value))
    error.value = ''
    ElMessage.success(t('common.success'))
  } catch (e: any) {
    error.value = e.message
    ElMessage.error(t('tools.json.parseError'))
  }
}

function minify() {
  if (!input.value.trim()) return
  try {
    input.value = JSON.stringify(JSON.parse(input.value))
    error.value = ''
    ElMessage.success(t('common.success'))
  } catch (e: any) {
    error.value = e.message
    ElMessage.error(t('tools.json.parseError'))
  }
}

function recompute() {
  if (!input.value.trim()) {
    error.value = ''
    return
  }
  try {
    JSON.parse(input.value)
    error.value = ''
  } catch (e: any) {
    error.value = e.message
  }
}

function clear() {
  input.value = ''
  error.value = ''
}

async function copy() {
  const text = view.value === 'text' ? formatted.value : JSON.stringify(JSON.parse(input.value || 'null'), null, 2)
  if (!text) return
  try {
    await navigator.clipboard.writeText(text)
    ElMessage.success(t('common.copySuccess'))
  } catch {
    ElMessage.error(t('common.copyFailed'))
  }
}

const fileInput = ref<HTMLInputElement>()
function loadFile() {
  fileInput.value?.click()
}
async function onFile(e: Event) {
  const f = (e.target as HTMLInputElement).files?.[0]
  if (!f) return
  input.value = await f.text()
  recompute()
  ;(e.target as HTMLInputElement).value = ''
}
</script>

<style scoped lang="scss">
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.hint {
  font-size: 12px;
  color: var(--text-secondary);
  &.err {
    color: var(--danger-color);
  }
}
.pretty {
  background: var(--bg-secondary);
  padding: 12px;
  border-radius: 6px;
  font-family: 'JetBrains Mono', Consolas, monospace;
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 480px;
  overflow: auto;
}
.tree-pane {
  max-height: 480px;
  overflow: auto;
}
.tree-row {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
}
.preview {
  color: var(--text-secondary);
  font-family: 'JetBrains Mono', Consolas, monospace;
  font-size: 12px;
}
.actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  align-items: center;
}
</style>