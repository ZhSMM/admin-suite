<template>
  <div class="page-container">
    <div class="page-header">
      <h2>{{ t('tools.encode.title') }}</h2>
    </div>

    <el-row :gutter="16">
      <el-col :span="12">
        <el-card shadow="never">
          <template #header>
            <div class="card-header">
              <strong>{{ t('tools.encode.input') }}</strong>
              <el-space>
                <el-select v-model="mode" size="small" style="width: 200px">
                  <el-option v-for="m in modes" :key="m.value" :label="m.label" :value="m.value" />
                </el-select>
                <el-button size="small" @click="loadFile">{{ t('tools.encode.loadFile') }}</el-button>
                <input ref="fileInput" type="file" hidden @change="onFile" />
              </el-space>
            </div>
          </template>
          <el-input
            v-model="input"
            type="textarea"
            :rows="14"
            spellcheck="false"
            :placeholder="t('tools.encode.placeholder')"
          />
          <div class="actions">
            <el-button :icon="Delete" @click="input = ''">{{ t('common.delete') }}</el-button>
            <el-button :icon="DocumentCopy" @click="copy(input)">{{ t('common.copy') }}</el-button>
          </div>
        </el-card>
      </el-col>

      <el-col :span="12">
        <el-card shadow="never">
          <template #header>
            <div class="card-header">
              <strong>{{ t('tools.encode.output') }}</strong>
              <el-space>
                <el-button size="small" @click="swap">{{ t('tools.encode.swap') }}</el-button>
                <el-button size="small" :icon="DocumentCopy" @click="copy(output)">{{ t('common.copy') }}</el-button>
              </el-space>
            </div>
          </template>
          <el-input
            v-model="output"
            type="textarea"
            :rows="14"
            spellcheck="false"
            readonly
          />
          <div v-if="error" class="err">
            <el-icon><CircleClose /></el-icon> {{ error }}
          </div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { DocumentCopy, Delete, CircleClose } from '@element-plus/icons-vue'

const { t } = useI18n()

const modes = computed(() => [
  { label: t('tools.encode.mode.url'),           value: 'url' as const },
  { label: t('tools.encode.mode.url-dec'),      value: 'url-dec' as const },
  { label: t('tools.encode.mode.html'),         value: 'html' as const },
  { label: t('tools.encode.mode.html-dec'),     value: 'html-dec' as const },
  { label: t('tools.encode.mode.b64'),          value: 'b64' as const },
  { label: t('tools.encode.mode.b64-dec'),      value: 'b64-dec' as const },
  { label: t('tools.encode.mode.hex'),          value: 'hex' as const },
  { label: t('tools.encode.mode.hex-dec'),      value: 'hex-dec' as const },
  { label: t('tools.encode.mode.unicode-esc'),  value: 'unicode-esc' as const },
  { label: t('tools.encode.mode.unicode-unesc'), value: 'unicode-unesc' as const }
])

type Mode = 'url' | 'url-dec' | 'html' | 'html-dec' | 'b64' | 'b64-dec' | 'hex' | 'hex-dec' | 'unicode-esc' | 'unicode-unesc'

const mode = ref<Mode>('url')
const input = ref('')
const output = ref('')
const error = ref('')

function recompute() {
  error.value = ''
  if (!input.value && mode.value !== 'b64' && mode.value !== 'b64-dec') {
    output.value = ''
    return
  }
  try {
    switch (mode.value) {
      case 'url':
        output.value = encodeURIComponent(input.value)
        break
      case 'url-dec':
        output.value = decodeURIComponent(input.value)
        break
      case 'html':
        output.value = escapeHtml(input.value)
        break
      case 'html-dec':
        output.value = unescapeHtml(input.value)
        break
      case 'b64':
        output.value = btoa(unescape(encodeURIComponent(input.value)))
        break
      case 'b64-dec':
        output.value = decodeURIComponent(escape(atob(input.value)))
        break
      case 'hex':
        output.value = textToHex(input.value)
        break
      case 'hex-dec':
        output.value = hexToText(input.value.replace(/\s+/g, ''))
        break
      case 'unicode-esc':
        output.value = unicodeEscape(input.value)
        break
      case 'unicode-unesc':
        output.value = unicodeUnescape(input.value)
        break
    }
  } catch (e: any) {
    error.value = e.message
    output.value = ''
  }
}

watch([input, mode], recompute, { immediate: true })

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
}

function unescapeHtml(s: string): string {
  return s
    .replace(/&#(\d+);/g, (_, code) => String.fromCharCode(parseInt(code)))
    .replace(/&#x([0-9a-fA-F]+);/g, (_, code) => String.fromCharCode(parseInt(code, 16)))
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&amp;/g, '&')
}

function textToHex(s: string): string {
  return Array.from(new TextEncoder().encode(s))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join(' ')
}

function hexToText(s: string): string {
  const bytes = new Uint8Array(s.match(/.{1,2}/g)?.map((h) => parseInt(h, 16)) || [])
  return new TextDecoder().decode(bytes)
}

function unicodeEscape(s: string): string {
  return Array.from(s)
    .map((c) => {
      const code = c.charCodeAt(0)
      if (code < 128) return c
      if (code < 0x10000) return `\\u${code.toString(16).padStart(4, '0')}`
      return `\\u${(code - 0x10000).toString(16)}`
    })
    .join('')
}

function unicodeUnescape(s: string): string {
  return s.replace(/\\u([0-9a-fA-F]{4})/g, (_, h) => String.fromCharCode(parseInt(h, 16)))
}

function swap() {
  const tmp = input.value
  input.value = output.value
  // Toggle mode: encode -> decode.
  const map: Partial<Record<Mode, Mode>> = {
    'url': 'url-dec',
    'url-dec': 'url',
    'html': 'html-dec',
    'html-dec': 'html',
    'b64': 'b64-dec',
    'b64-dec': 'b64',
    'hex': 'hex-dec',
    'hex-dec': 'hex',
    'unicode-esc': 'unicode-unesc',
    'unicode-unesc': 'unicode-esc'
  }
  mode.value = map[mode.value] || mode.value
}

async function copy(value: string) {
  if (!value) return
  try {
    await navigator.clipboard.writeText(value)
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
  ;(e.target as HTMLInputElement).value = ''
}
</script>

<style scoped lang="scss">
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.actions {
  margin-top: 12px;
  display: flex;
  gap: 8px;
}
.err {
  margin-top: 12px;
  color: var(--danger-color);
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 13px;
}
</style>