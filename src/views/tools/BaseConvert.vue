<template>
  <div class="page-container">
    <div class="page-header">
      <h2>{{ t('tools.base.title') }}</h2>
      <el-radio-group v-model="mode" @change="resetOutputs">
        <el-radio-button value="number">{{ t('tools.base.modeNumber') }}</el-radio-button>
        <el-radio-button value="text">{{ t('tools.base.modeText') }}</el-radio-button>
      </el-radio-group>
    </div>

    <!-- ============ Number mode ============ -->
    <template v-if="mode === 'number'">
      <el-card shadow="never">
        <el-form label-width="100px">
          <el-form-item :label="t('tools.base.input')">
            <el-input
              v-model="numInput"
              :placeholder="t('tools.base.inputPlaceholder')"
              clearable
              @input="computeNumber"
            />
          </el-form-item>
          <el-form-item :label="t('tools.base.fromBase')">
            <el-select v-model="fromBase" style="width: 200px" @change="computeNumber">
              <el-option :label="t('tools.base.fromBase.binary')" :value="2" />
              <el-option :label="t('tools.base.fromBase.octal')" :value="8" />
              <el-option :label="t('tools.base.fromBase.decimal')" :value="10" />
              <el-option :label="t('tools.base.fromBase.hex')" :value="16" />
            </el-select>
          </el-form-item>
        </el-form>
      </el-card>

      <el-row :gutter="16" style="margin-top: 16px">
        <el-col :span="8" v-for="card in numberCards" :key="card.label">
          <el-card shadow="hover" class="output-card">
            <template #header>
              <strong>{{ numberLabel(card) }}</strong>
            </template>
            <div class="output-value">
              <code>{{ card.value || '—' }}</code>
            </div>
            <el-button text size="small" @click="copy(card.value)">
              <el-icon><CopyDocument /></el-icon>
              {{ t('common.copy') }}
            </el-button>
          </el-card>
        </el-col>
      </el-row>
    </template>

    <!-- ============ Text mode ============ -->
    <template v-else>
      <el-card shadow="never">
        <el-form label-width="100px">
          <el-form-item :label="t('tools.base.textInput')">
            <el-input
              v-model="textInput"
              type="textarea"
              :rows="6"
              :placeholder="t('tools.base.textPlaceholder')"
              @input="computeText"
            />
          </el-form-item>
        </el-form>
      </el-card>

      <el-row :gutter="16" style="margin-top: 16px">
        <el-col :span="12" v-for="card in textCards" :key="card.label">
          <el-card shadow="hover" class="output-card">
            <template #header>
              <strong>{{ textLabel(card) }}</strong>
            </template>
            <div class="output-value">
              <code :class="{ mono: true }">{{ card.value || '—' }}</code>
            </div>
            <el-button text size="small" @click="copy(card.value)">
              <el-icon><CopyDocument /></el-icon>
              {{ t('common.copy') }}
            </el-button>
          </el-card>
        </el-col>
      </el-row>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, reactive } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { CopyDocument } from '@element-plus/icons-vue'

const { t } = useI18n()

const mode = ref<'number' | 'text'>('number')
const numInput = ref('255')
const fromBase = ref<2 | 8 | 10 | 16>(10)
const textInput = ref('Hello, Admin Suite!')

// -------- Number mode --------
const numberCards = reactive([
  { label: 'tools.base.output.bin', value: '' },
  { label: 'tools.base.output.oct', value: '' },
  { label: 'tools.base.output.dec', value: '' },
  { label: 'tools.base.output.hex', value: '' }
])
// `label` here is a translation key — the template renders it via t().
function numberLabel(card: { label: string }) { return t(card.label) }

function computeNumber() {
  const raw = numInput.value.trim()
  if (!raw) {
    numberCards.forEach((c) => (c.value = ''))
    return
  }
  // Parse `raw` as a number in the selected base.
  const n = parseInt(raw, fromBase.value)
  if (Number.isNaN(n)) {
    numberCards.forEach((c) => (c.value = t('tools.base.invalid')))
    return
  }
  // Convert to other bases. Use BigInt only when value fits; otherwise clamp.
  const safe = (fn: (x: number) => string) => {
    try {
      return fn(n)
    } catch {
      return 'overflow'
    }
  }
  numberCards[0].value = safe((x) => (x >= 0 ? x.toString(2) : '-' + (-x).toString(2)))
  numberCards[1].value = safe((x) => (x >= 0 ? x.toString(8) : '-' + (-x).toString(8)))
  numberCards[2].value = safe((x) => x.toString(10))
  numberCards[3].value = safe((x) => (x >= 0 ? x.toString(16) : '-' + (-x).toString(16)))
}

// -------- Text mode --------
const textCards = reactive([
  { label: 'tools.base.output.hexBytes',   value: '' },
  { label: 'tools.base.output.binBytes',   value: '' },
  { label: 'tools.base.output.base64',     value: '' },
  { label: 'tools.base.output.url',        value: '' },
  { label: 'tools.base.output.charCodes',  value: '' },
  { label: 'tools.base.output.length',     value: '' }
])
function textLabel(card: { label: string }) { return t(card.label) }

function computeText() {
  const s = textInput.value
  const enc = new TextEncoder()
  const bytes = enc.encode(s)

  textCards[0].value = bytesToHex(bytes)
  textCards[1].value = bytesToBin(bytes)
  textCards[2].value = bytesToBase64(bytes)
  textCards[3].value = urlEncode(s)
  textCards[4].value = charCodes(s)
  textCards[5].value = String(bytes.length)
}

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, '0'))
    .join(' ')
}
function bytesToBin(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(2).padStart(8, '0'))
    .join(' ')
}
function bytesToBase64(bytes: Uint8Array): string {
  let bin = ''
  for (const b of bytes) bin += String.fromCharCode(b)
  return btoa(bin)
}
function urlEncode(s: string): string {
  return encodeURIComponent(s)
}
function charCodes(s: string): string {
  return Array.from(s)
    .map((c) => `${c} (${c.charCodeAt(0)})`)
    .join(' ')
}

function resetOutputs() {
  if (mode.value === 'number') computeNumber()
  else computeText()
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

computeNumber()
</script>

<style scoped lang="scss">
.output-card {
  margin-bottom: 16px;
}
.output-value {
  min-height: 36px;
  word-break: break-all;
  font-size: 13px;
  margin-bottom: 8px;
  code {
    background: var(--bg-secondary);
    padding: 2px 6px;
    border-radius: 4px;
    color: var(--text-primary);
  }
}
.mono {
  font-family: 'JetBrains Mono', Consolas, monospace;
  font-size: 12px;
}
</style>