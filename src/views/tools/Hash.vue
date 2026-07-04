<template>
  <div class="page-container">
    <div class="page-header">
      <h2>{{ t('tools.hash.title') }}</h2>
      <el-radio-group v-model="source" @change="reset">
        <el-radio-button value="text">{{ t('tools.hash.fromText') }}</el-radio-button>
        <el-radio-button value="file">{{ t('tools.hash.fromFile') }}</el-radio-button>
      </el-radio-group>
    </div>

    <el-card shadow="never" v-if="source === 'text'">
      <el-input
        v-model="text"
        type="textarea"
        :rows="6"
        :placeholder="t('tools.hash.textPlaceholder')"
        @input="recompute"
      />
    </el-card>
    <el-card shadow="never" v-else>
      <el-upload
        drag
        :auto-upload="false"
        :show-file-list="false"
        :on-change="onFile"
      >
        <el-icon class="el-icon--upload"><UploadFilled /></el-icon>
        <div class="el-upload__text">{{ t('tools.hash.dropOrClick') }}</div>
        <template #tip>
          <div class="el-upload__tip">{{ fileName }}</div>
        </template>
      </el-upload>
    </el-card>

    <el-table :data="rows" v-loading="loading" style="margin-top: 16px">
      <el-table-column :label="t('tools.hash.algorithm')" width="140">
        <template #default="{ row }">
          <el-tag :type="row.tag as any">{{ algoLabel(row.algo) }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('tools.hash.digest')">
        <template #default="{ row }">
          <code class="digest">{{ row.value || '—' }}</code>
        </template>
      </el-table-column>
      <el-table-column :label="t('common.actions')" width="120">
        <template #default="{ row }">
          <el-button text size="small" @click="copy(row.value)">
            <el-icon><CopyDocument /></el-icon>
            {{ t('common.copy') }}
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <el-card shadow="never" style="margin-top: 16px" v-if="source === 'text'">
      <template #header><strong>{{ t('tools.hash.hmac') }}</strong></template>
      <el-form label-width="120px">
        <el-form-item :label="t('tools.hash.hmacKey')">
          <el-input v-model="hmacKey" :placeholder="t('tools.hash.hmacKeyPlaceholder')" />
        </el-form-item>
      </el-form>
      <el-table :data="hmacRows" :show-header="false">
        <el-table-column width="160">
          <template #default="{ row }">
            <el-tag>{{ algoLabel(row.algo.replace('HMAC-', '')) }} (HMAC)</el-tag>
          </template>
        </el-table-column>
        <el-table-column>
          <template #default="{ row }">
            <code class="digest">{{ row.value || '—' }}</code>
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { CopyDocument, UploadFilled } from '@element-plus/icons-vue'
import md5 from 'md5'

const { t } = useI18n()

const source = ref<'text' | 'file'>('text')
const text = ref('')
const fileName = ref('')
const fileBytes = ref<Uint8Array | null>(null)
const hmacKey = ref('')
const loading = ref(false)

const algorithms = ['MD5', 'SHA-1', 'SHA-256', 'SHA-384', 'SHA-512'] as const
type Algo = (typeof algorithms)[number]

const tagByAlgo: Record<Algo, 'primary' | 'success' | 'info' | 'warning' | 'danger'> = {
  'MD5': 'danger',
  'SHA-1': 'warning',
  'SHA-256': 'success',
  'SHA-384': 'info',
  'SHA-512': 'primary'
}

function algoLabel(algo: string): string {
  return t(`tools.hash.algo.${algo}`)
}

const rows = reactive<{ algo: string; value: string; tag: string }[]>(
  algorithms.map((a) => ({ algo: a, value: '', tag: tagByAlgo[a] }))
)
const hmacRows = reactive<{ algo: string; value: string }[]>(
  algorithms.slice(1).map((a) => ({ algo: `HMAC-${a}`, value: '' }))
)

async function hashBytes(algo: Algo, data: Uint8Array): Promise<string> {
  if (algo === 'MD5') return md5(Buffer.from(data))
  const buf = await crypto.subtle.digest(algo, data as BufferSource)
  return Array.from(new Uint8Array(buf))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('')
}

async function computeAll(input: Uint8Array | string) {
  loading.value = true
  try {
    for (let i = 0; i < algorithms.length; i++) {
      const algo = algorithms[i]
      let bytes: Uint8Array
      if (typeof input === 'string') {
        bytes = new TextEncoder().encode(input)
      } else {
        bytes = input
      }
      rows[i].value = await hashBytes(algo, bytes)
    }
    await computeHmac()
  } finally {
    loading.value = false
  }
}

async function computeHmac() {
  if (!hmacKey.value) {
    hmacRows.forEach((r) => (r.value = ''))
    return
  }
  const keyBytes = new TextEncoder().encode(hmacKey.value)
  for (let i = 0; i < hmacRows.length; i++) {
    const algo = hmacRows[i].algo.replace('HMAC-', '') as Exclude<Algo, 'MD5'>
    hmacRows[i].value = await hmac(algo, keyBytes)
  }
}

async function hmac(algo: Exclude<Algo, 'MD5'>, keyBytes: Uint8Array): Promise<string> {
  const blockSize = algo === 'SHA-384' || algo === 'SHA-512' ? 128 : 64
  let key = keyBytes
  if (key.length > blockSize) {
    const buf = await crypto.subtle.digest(algo, key as BufferSource)
    key = new Uint8Array(buf)
  }
  if (key.length < blockSize) {
    const padded = new Uint8Array(blockSize)
    padded.set(key)
    key = padded
  }
  const oKey = new Uint8Array(blockSize)
  const iKey = new Uint8Array(blockSize)
  for (let i = 0; i < blockSize; i++) {
    oKey[i] = key[i] ^ 0x5c
    iKey[i] = key[i] ^ 0x36
  }
  const input = source.value === 'text'
    ? new TextEncoder().encode(text.value)
    : (fileBytes.value || new Uint8Array())
  const inner = new Uint8Array(iKey.length + input.length)
  inner.set(iKey)
  inner.set(input, iKey.length)
  const innerHash = new Uint8Array(await crypto.subtle.digest(algo, inner as BufferSource))
  const outer = new Uint8Array(oKey.length + innerHash.length)
  outer.set(oKey)
  outer.set(innerHash, oKey.length)
  const out = new Uint8Array(await crypto.subtle.digest(algo, outer as BufferSource))
  return Array.from(out).map((b) => b.toString(16).padStart(2, '0')).join('')
}

function recompute() {
  computeAll(text.value)
}

function reset() {
  text.value = ''
  fileBytes.value = null
  fileName.value = ''
  rows.forEach((r) => (r.value = ''))
  hmacRows.forEach((r) => (r.value = ''))
}

async function onFile(uploadFile: any) {
  fileName.value = uploadFile.name
  const ab = await uploadFile.raw.arrayBuffer()
  fileBytes.value = new Uint8Array(ab)
  await computeAll(fileBytes.value)
}

watch(hmacKey, () => computeHmac())

async function copy(value: string) {
  if (!value) return
  try {
    await navigator.clipboard.writeText(value)
    ElMessage.success(t('common.copySuccess'))
  } catch {
    ElMessage.error(t('common.copyFailed'))
  }
}
</script>

<style scoped lang="scss">
.digest {
  background: var(--bg-secondary);
  padding: 2px 6px;
  border-radius: 4px;
  font-family: 'JetBrains Mono', Consolas, monospace;
  font-size: 12px;
  word-break: break-all;
}
</style>