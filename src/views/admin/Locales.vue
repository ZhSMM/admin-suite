<template>
  <div class="page-container">
    <div class="page-header">
      <h2>{{ t('locales.title') }}</h2>
      <div>
        <el-button @click="triggerImport">
          <el-icon><Upload /></el-icon>
          {{ t('locales.import') }}
        </el-button>
        <el-button type="primary" @click="triggerExport">
          <el-icon><Download /></el-icon>
          {{ t('locales.export') }}
        </el-button>
        <input ref="fileInput" type="file" accept="application/json" hidden @change="onFile" />
      </div>
    </div>

    <el-alert :title="t('locales.importHelp')" type="info" :closable="false" style="margin-bottom: 12px" />

    <el-table :data="items" v-loading="loading" border>
      <el-table-column :label="t('locales.code')" prop="code" width="160" />
      <el-table-column :label="t('locales.name')" prop="name" width="180" />
      <el-table-column :label="t('locales.messages')" width="120">
        <template #default="{ row }">{{ messageCount(row) }}</template>
      </el-table-column>
      <el-table-column :label="t('common.source')" width="120">
        <template #default="{ row }">
          <el-tag :type="row.source === 'builtin' ? 'info' : 'success'" size="small">
            {{ row.source }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('common.actions')" width="320">
        <template #default="{ row }">
          <el-button text type="primary" :disabled="row.active" @click="activate(row.code)">
            {{ t('themes.activate') }}
          </el-button>
          <el-button text type="primary" @click="exportOne(row)">
            <el-icon><Download /></el-icon>
            {{ t('locales.exportOne') }}
          </el-button>
          <el-button
            text
            type="danger"
            :disabled="row.built_in || row.active"
            @click="remove(row)"
          >
            {{ t('common.delete') }}
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- Export dialog: pick source + target code, see the result -->
    <el-dialog v-model="exportDialog.open" :title="t('locales.exportDialog')" width="640">
      <el-form label-width="120px">
        <el-form-item :label="t('locales.exportSource')">
          <el-select v-model="exportDialog.source" filterable @change="rebuildExport">
            <el-option
              v-for="it in items"
              :key="it.id"
              :label="`${it.code} (${it.name})`"
              :value="it.code"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('locales.exportTargetCode')">
          <el-input v-model="exportDialog.targetCode" :placeholder="exportDialog.source" @input="rebuildExport" />
        </el-form-item>
        <el-form-item :label="t('locales.exportTargetLabel')">
          <el-input v-model="exportDialog.targetLabel" :placeholder="t('locales.exportTargetLabelPlaceholder')" @input="rebuildExport" />
        </el-form-item>
        <el-form-item :label="t('locales.exportFillEmpty')">
          <el-switch v-model="exportDialog.fillEmpty" @change="rebuildExport" />
          <small style="margin-left: 8px; color: var(--text-secondary)">{{ t('locales.exportFillEmptyHelp') }}</small>
        </el-form-item>
        <el-form-item :label="t('locales.preview')">
          <pre class="export-preview">{{ exportDialog.text }}</pre>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="copyExport">{{ t('common.copy') }}</el-button>
        <el-button type="primary" :icon="Download" @click="downloadExport">
          {{ t('locales.download') }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { useI18n } from 'vue-i18n'
import { Upload, Download } from '@element-plus/icons-vue'
import { useAuthStore } from '@/stores/auth'
import { useLocaleStore } from '@/stores/locale'
import { resourcesApi, type Resource } from '@/api/resources'

const { t, locale: currentLocale } = useI18n()
const auth = useAuthStore()
const locale = useLocaleStore()

const items = ref<Resource[]>([])
const loading = ref(false)

async function reload() {
  loading.value = true
  try {
    const r = await resourcesApi.list(auth.token, 'locale')
    items.value = r.items
  } finally {
    loading.value = false
  }
}
onMounted(reload)

function messageCount(r: Resource) {
  try {
    const p = JSON.parse(r.content)
    return Object.keys(p.messages || {}).length
  } catch {
    return 0
  }
}

function parseResource(r: Resource): { id: string; label: string; messages: Record<string, string> } {
  try {
    const p = JSON.parse(r.content)
    return {
      id: p.id || r.code,
      label: p.label || r.name,
      messages: p.messages || {}
    }
  } catch {
    return { id: r.code, label: r.name, messages: {} }
  }
}

async function activate(code: string) {
  await locale.activate(code)
  await reload()
  ElMessage.success(t('common.success'))
}

async function remove(row: Resource) {
  await ElMessageBox.confirm(t('common.confirmDelete'), '', { type: 'warning' })
  await locale.remove(row.id)
  await reload()
  ElMessage.success(t('common.success'))
}

const fileInput = ref<HTMLInputElement>()
function triggerImport() {
  fileInput.value?.click()
}
async function onFile(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  const text = await file.text()
  try {
    await locale.importFromJson(text)
    await reload()
    ElMessage.success(t('common.success'))
  } catch (err) {
    ElMessage.error((err as Error).message)
  } finally {
    input.value = ''
  }
}

// ---- Export ----
const exportDialog = reactive({
  open: false,
  source: '',
  targetCode: '',
  targetLabel: '',
  fillEmpty: true,
  text: ''
})

function triggerExport() {
  if (!items.value.length) {
    ElMessage.warning(t('locales.noLocale'))
    return
  }
  exportDialog.open = true
  // Default: export the active locale (or the first one)
  const active = items.value.find((it) => it.active) || items.value[0]
  exportDialog.source = active.code
  exportDialog.targetCode = active.code
  exportDialog.targetLabel = active.name
  rebuildExport()
}

function exportOne(row: Resource) {
  exportDialog.open = true
  exportDialog.source = row.code
  exportDialog.targetCode = row.code
  exportDialog.targetLabel = row.name
  rebuildExport()
}

function rebuildExport() {
  const src = items.value.find((it) => it.code === exportDialog.source)
  if (!src) {
    exportDialog.text = ''
    return
  }
  const parsed = parseResource(src)
  const code = exportDialog.targetCode.trim() || parsed.id
  const label = exportDialog.targetLabel.trim() || parsed.label
  // Collect the union of keys across all locale resources so the translator
  // sees every key the app might ask for, even if the source is missing some.
  const keyUnion = new Set<string>()
  const allMessages: Record<string, string> = {}
  for (const it of items.value) {
    try {
      const p = JSON.parse(it.content)
      for (const k of Object.keys(p.messages || {})) keyUnion.add(k)
    } catch { /* ignore */ }
  }
  for (const k of keyUnion) {
    if (parsed.messages[k] !== undefined) {
      allMessages[k] = parsed.messages[k]
    } else if (exportDialog.fillEmpty) {
      allMessages[k] = ''
    }
  }
  const payload = { id: code, label, messages: allMessages }
  exportDialog.text = JSON.stringify(payload, null, 2)
}

watch(() => exportDialog.source, rebuildExport)
watch(() => exportDialog.fillEmpty, rebuildExport)

async function copyExport() {
  try {
    await navigator.clipboard.writeText(exportDialog.text)
    ElMessage.success(t('common.copySuccess'))
  } catch {
    ElMessage.error(t('common.copyFailed'))
  }
}

function downloadExport() {
  const code = exportDialog.targetCode.trim() || exportDialog.source || 'locale'
  const filename = `${code}.json`
  const blob = new Blob([exportDialog.text], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  document.body.appendChild(a)
  a.click()
  document.body.removeChild(a)
  URL.revokeObjectURL(url)
  ElMessage.success(t('common.success'))
}
</script>

<style scoped lang="scss">
.export-preview {
  background: var(--bg-secondary);
  padding: 12px;
  border-radius: 6px;
  font-family: 'JetBrains Mono', Consolas, monospace;
  font-size: 12px;
  max-height: 320px;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-all;
  margin: 0;
}
</style>