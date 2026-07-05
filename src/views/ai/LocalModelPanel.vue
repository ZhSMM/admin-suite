<template>
  <div class="local-panel">
    <!-- Disclaimer modal (shown until user accepts once) -->
    <el-dialog
      v-model="disclaimerOpen"
      :title="t('settings.ai.fallback.disclaimerTitle')"
      width="560"
      :show-close="false"
      :close-on-click-modal="false"
    >
      <p style="white-space: pre-wrap">{{ disclaimerText }}</p>
      <template #footer>
        <el-button type="primary" @click="onAcceptDisclaimer">
          {{ t('settings.ai.fallback.disclaimerAccept') }}
        </el-button>
      </template>
    </el-dialog>

    <div class="row">
      <div class="meta">
        <div class="label">{{ t('settings.ai.fallback.status') }}</div>
        <div class="value">
          <el-tag :type="statusTagType" size="small">{{ statusLabel }}</el-tag>
        </div>
      </div>
      <div class="meta" v-if="isReady && serverRunning">
        <div class="label">{{ t('settings.ai.fallback.endpoint') }}</div>
        <div class="value">
          <code>{{ baseUrl }}</code>
        </div>
      </div>
    </div>

    <el-form label-width="180px">
      <el-form-item :label="t('settings.ai.fallback.model')">
        <el-select
          v-model="selectedModelId"
          :disabled="isInstalling"
          filterable
          style="width: 100%"
        >
          <el-option
            v-for="m in llm.fallbackModels"
            :key="m.id"
            :label="`${m.display_name} · ${formatSize(m.size_bytes)} · RAM≥${m.min_ram_gb}GB`"
            :value="m.id"
          />
        </el-select>
      </el-form-item>

      <!-- Not installed / errored -->
      <el-form-item v-if="showInstallButton" :label="t('settings.ai.fallback.diskFree')">
        <span v-if="diskFree == null">…</span>
        <span v-else>{{ diskFreeHuman }}</span>
        <el-tag
          v-if="diskFree != null && diskFree < selectedModelSize"
          type="danger"
          size="small"
          style="margin-left: 8px"
        >
          {{ t('settings.ai.fallback.diskInsufficient') }}
        </el-tag>
      </el-form-item>

      <!-- Download progress -->
      <el-form-item v-if="isInstalling" :label="t('settings.ai.fallback.progress')">
        <div class="progress-meta">
          <span class="stage">
            <span v-if="llm.installCurrentStage === 'server'">
              {{ t('settings.ai.fallback.stageServer') }}
            </span>
            <span v-else>{{ t('settings.ai.fallback.stageModel') }}</span>
          </span>
          <span class="counter" v-if="llm.installProgress">
            {{ formatBytes(llm.installProgress.bytesDone) }}
            <span class="counter-sep">/</span>
            {{ formatBytes(llm.installProgress.totalBytes || selectedModelSize) }}
          </span>
          <span class="speed" v-if="llm.installProgress && llm.installProgress.speedBps > 0">
            {{ formatSpeed(llm.installProgress.speedBps) }}
          </span>
          <span class="eta" v-if="llm.installProgress && llm.installProgress.etaSeconds > 0">
            ETA {{ formatEta(llm.installProgress.etaSeconds) }}
          </span>
        </div>
        <el-progress
          :percentage="progressPct"
          :stroke-width="14"
          :status="llm.installError ? 'exception' : undefined"
          :format="formatProgress"
        />
        <div v-if="stalledHint" class="stall-hint">
          <el-alert :title="stalledHint" type="warning" :closable="false" show-icon />
        </div>
      </el-form-item>

      <el-form-item>
        <template v-if="showInstallButton">
          <el-button
            type="primary"
            :icon="Download"
            :loading="isInstalling"
            @click="onInstall"
          >
            {{ t('settings.ai.fallback.install') }}
          </el-button>
          <el-button :icon="Connection" :loading="speedTesting" @click="runSpeedTest">
            {{ t('settings.ai.fallback.testSpeed') }}
          </el-button>
          <el-button :icon="FolderOpened" @click="onPickLocalFile">
            {{ t('settings.ai.fallback.importLocal') }}
          </el-button>
        </template>

        <template v-else-if="isInstalling">
          <el-button type="danger" plain :icon="CircleClose" @click="onCancel">
            {{ t('settings.ai.fallback.cancel') }}
          </el-button>
        </template>

        <template v-else-if="isReady && !serverRunning">
          <el-button type="success" :icon="VideoPlay" @click="onStartServer">
            {{ t('settings.ai.fallback.startServer') }}
          </el-button>
          <el-button type="danger" plain :icon="Delete" @click="onRemove">
            {{ t('settings.ai.fallback.remove') }}
          </el-button>
        </template>

        <template v-else-if="serverRunning">
          <el-button type="warning" :icon="VideoPause" @click="onStopServer">
            {{ t('settings.ai.fallback.stopServer') }}
          </el-button>
          <el-button type="danger" plain :icon="Delete" @click="onRemove">
            {{ t('settings.ai.fallback.remove') }}
          </el-button>
        </template>
      </el-form-item>

      <!-- Speed test results -->
      <div v-if="speedResults.length > 0" class="speed-results">
        <el-table :data="speedResults" size="small" border>
          <el-table-column :label="t('settings.ai.fallback.speedMirror')" prop="label" width="120">
            <template #default="{ row }">
              <el-tag v-if="row === bestMirror" type="success" size="small">
                ★ {{ row.label }}
              </el-tag>
              <span v-else>{{ row.label }}</span>
              <el-tag
                v-if="row.kind === 'probe'"
                size="small"
                type="info"
                effect="plain"
                style="margin-left: 6px"
              >
                HEAD
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column :label="t('settings.ai.fallback.speedReachable')" width="100">
            <template #default="{ row }">
              <el-tag v-if="row.reachable" type="success" size="small">
                {{ t('settings.ai.fallback.speedOk') }}
              </el-tag>
              <el-tooltip v-else :content="row.error || ''">
                <el-tag type="danger" size="small">{{ t('settings.ai.fallback.speedFail') }}</el-tag>
              </el-tooltip>
            </template>
          </el-table-column>
          <el-table-column :label="t('settings.ai.fallback.speedMbps')" width="120">
            <template #default="{ row }">
              <span v-if="row.reachable">{{ formatSpeed(row.speedBps) }}</span>
              <span v-else>—</span>
            </template>
          </el-table-column>
          <el-table-column :label="t('settings.ai.fallback.speedUrl')" prop="url" show-overflow-tooltip>
            <template #default="{ row }">
              <code class="mirror-url-code">{{ row.url }}</code>
            </template>
          </el-table-column>
          <el-table-column label="操作" width="200" fixed="right">
            <template #default="{ row }">
              <el-button
                v-if="row.reachable && row.kind === 'download'"
                size="small"
                :icon="DocumentCopy"
                @click="copyMirrorUrl(row)"
              >
                复制直链
              </el-button>
              <el-button
                v-if="row.reachable && row.kind === 'download' && !isInstalling"
                size="small"
                type="primary"
                :icon="Download"
                @click="useMirrorInstall(row)"
              >
                用此下载
              </el-button>
            </template>
          </el-table-column>
        </el-table>
        <div v-if="bestMirror" class="speed-hint">
          <el-alert
            :title="t('settings.ai.fallback.speedHint', { mirror: bestMirror.label, speed: formatSpeed(bestMirror.speedBps) })"
            type="success"
            :closable="false"
            show-icon
          />
        </div>
        <div class="speed-manual">
          <el-alert
            title="下载慢/失败？手动方案"
            type="info"
            :closable="false"
            show-icon
          >
            <template #default>
              <ol class="speed-manual-list">
                <li>点「复制直链」，用 <b>IDM / 迅雷 / aria2c</b> 拉到本地任意目录</li>
                <li>回这里点「<b>选择本地文件</b>」导入 .gguf，自动按 SHA-256 校验后落盘</li>
                <li>或者粘贴其他源 URL 到下面的输入框，用「<b>从此 URL 安装</b>」直接走本应用下载器</li>
              </ol>
              <div class="speed-manual-input">
                <el-input
                  v-model="manualInstallUrl"
                  placeholder="https://... 任意 .gguf 直链（HF / ModelScope / 群晖 / OSS …）"
                  clearable
                >
                  <template #append>
                    <el-button
                      :icon="Download"
                      :loading="manualInstalling"
                      :disabled="!manualInstallUrl.trim()"
                      @click="onInstallFromManualUrl"
                    >
                      从此 URL 安装
                    </el-button>
                  </template>
                </el-input>
              </div>
            </template>
          </el-alert>
        </div>
      </div>

      <el-alert
        v-if="llm.installError"
        :title="llm.installError"
        type="error"
        show-icon
        :closable="true"
        @close="llm.installError = null"
        style="margin-top: 8px"
      />
    </el-form>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  Download,
  VideoPlay,
  VideoPause,
  Delete,
  CircleClose,
  Connection,
  FolderOpened,
  DocumentCopy
} from '@element-plus/icons-vue'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import { useLlmStore } from '@/stores/llm'

const { t } = useI18n()
const auth = useAuthStore()
const llm = useLlmStore()

const disclaimerOpen = ref(false)
const selectedModelId = ref<string>('')

onMounted(async () => {
  if (!llm.disclaimerAccepted) {
    disclaimerOpen.value = true
  }
  await llm.refreshFallback(auth.token || '')
  if (!selectedModelId.value) {
    selectedModelId.value =
      llm.fallbackState?.selected_model_id ??
      llm.fallbackModels[0]?.id ??
      ''
  }
  refreshDiskFree()
})

// Re-check disk free whenever the user picks a different model (the
// "enough space" hint depends on the selected model's size).
watch(selectedModelId, () => {
  if (showInstallButton.value) refreshDiskFree()
})

watch(
  () => llm.fallbackState?.selected_model_id,
  (id) => {
    if (id && !selectedModelId.value) selectedModelId.value = id
  }
)

const disclaimerText = computed(() => {
  const model = llm.fallbackModels.find((m) => m.id === selectedModelId.value)
  const name = model?.display_name ?? 'the selected model'
  return t('settings.ai.fallback.disclaimerBody', { model: name })
})

function onAcceptDisclaimer() {
  llm.acceptDisclaimer()
  disclaimerOpen.value = false
}

const selectedModelSize = computed(() => {
  const m = llm.fallbackModels.find((mm) => mm.id === selectedModelId.value)
  return m?.size_bytes ?? 0
})

const isReady = computed(() => {
  const p = llm.fallbackState?.phase
  // Rust serializes Phase as either:
  //   - a bare string for unit variants: "not_downloaded" | "verifying"
  //   - an object for struct variants:    { ready: {...} } | { error: {...} }
  // We check both shapes here so the UI doesn't crash on either form.
  if (p == null) return false
  if (typeof p === 'object') return 'ready' in p
  return false
})

const serverRunning = computed(() => {
  const p = llm.fallbackState?.phase
  return isReady.value && llm.fallbackState?.llama_server_port != null
})

const baseUrl = computed(() => {
  const port = llm.fallbackState?.llama_server_port
  return port ? `http://127.0.0.1:${port}/v1` : ''
})

const isInstalling = computed(
  () => llm.installInFlight || (llm.installProgress != null)
)

const showInstallButton = computed(
  () => !isReady.value && !isInstalling.value
)

const statusLabel = computed(() => {
  const p = llm.fallbackState?.phase
  if (p == null) return t('settings.ai.fallback.statusUnknown')
  // Struct variants — check the object key first so TS narrows correctly.
  if (typeof p === 'object') {
    if ('downloading' in p) return t('settings.ai.fallback.statusDownloading')
    if ('ready' in p) {
      return serverRunning.value
        ? t('settings.ai.fallback.statusRunning')
        : t('settings.ai.fallback.statusReady')
    }
    if ('error' in p) return t('settings.ai.fallback.statusError')
    if ('hash_mismatch' in p) return t('settings.ai.fallback.statusHashMismatch')
  }
  // Bare-string unit variants:
  if (p === 'not_downloaded') return t('settings.ai.fallback.statusNotInstalled')
  if (p === 'verifying') return t('settings.ai.fallback.statusVerifying')
  return t('settings.ai.fallback.statusUnknown')
})

const statusTagType = computed(() => {
  const p = llm.fallbackState?.phase
  if (p == null) return 'info'
  if (p === 'verifying') return 'warning'
  if (typeof p === 'object') {
    if ('ready' in p) return serverRunning.value ? 'success' : 'info'
    if ('error' in p || 'hash_mismatch' in p) return 'danger'
    if ('downloading' in p) return 'warning'
  }
  return 'info'
})

const progressPct = computed(() => {
  const p = llm.installProgress
  if (!p || !p.totalBytes) return 0
  return Math.min(100, Math.round((p.bytesDone / p.totalBytes) * 100))
})

// Stall hint: when bytesDone hasn't moved for >10s while expected > bytesDone.
const stalledHint = computed(() => {
  const p = llm.installProgress
  if (!p || !isInstalling.value) return ''
  const pct = p.totalBytes ? p.bytesDone / p.totalBytes : 0
  if (pct > 0.02 && pct < 0.99 && p.speedBps === 0) {
    return t('settings.ai.fallback.stalledHint')
  }
  return ''
})

const diskFree = ref<number | null>(null)

const diskFreeHuman = computed(() => {
  if (diskFree.value == null) return t('settings.ai.fallback.diskFreeChecking')
  return formatBytes(diskFree.value)
})

async function refreshDiskFree() {
  diskFree.value = await llm.fetchDiskFree()
}

// ---- v0.6.5: speed-test mirrors ----
const speedResults = ref<import('@/api/llm').SpeedTestResult[]>([])
const speedTesting = ref(false)

async function runSpeedTest() {
  if (!selectedModelId.value) return
  speedTesting.value = true
  speedResults.value = []
  try {
    speedResults.value = await llm.speedTest(auth.token || '', selectedModelId.value)
  } finally {
    speedTesting.value = false
  }
}

const manualInstallUrl = ref('')
const manualInstalling = ref(false)

async function copyMirrorUrl(row: import('@/api/llm').SpeedTestResult) {
  try {
    await navigator.clipboard.writeText(row.url)
    ElMessage.success(`已复制 ${row.label} 直链`)
  } catch (e) {
    ElMessage.error('复制失败：' + (e instanceof Error ? e.message : String(e)))
  }
}

async function useMirrorInstall(row: import('@/api/llm').SpeedTestResult) {
  if (!selectedModelId.value) return
  if (!llm.disclaimerAccepted) {
    disclaimerOpen.value = true
    return
  }
  try {
    await ElMessageBox.confirm(
      `将使用 ${row.label}（${formatSpeed(row.speedBps)}）开始安装，确定？`,
      '确认使用此 Mirror',
      { type: 'info', confirmButtonText: '开始安装', cancelButtonText: '取消' }
    )
  } catch {
    return
  }
  speedResults.value = []
  await llm.installModel(auth.token || '', selectedModelId.value, row.url)
}

async function onInstallFromManualUrl() {
  const url = manualInstallUrl.value.trim()
  if (!url || !selectedModelId.value) return
  if (!llm.disclaimerAccepted) {
    disclaimerOpen.value = true
    return
  }
  manualInstalling.value = true
  try {
    await llm.installModel(auth.token || '', selectedModelId.value, url)
  } finally {
    manualInstalling.value = false
  }
}

const bestMirror = computed(() => {
  const reachable = speedResults.value.filter((r) => r.reachable && r.speedBps > 0)
  if (reachable.length === 0) return null
  return reachable.reduce((a, b) => (a.speedBps >= b.speedBps ? a : b))
})

function formatBytes(n: number): string {
  if (!Number.isFinite(n) || n < 0) return '—'
  if (n < 1024) return `${n} B`
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`
}

function formatSize(n: number): string {
  return formatBytes(n)
}

function formatSpeed(bps: number): string {
  if (!Number.isFinite(bps) || bps <= 0) return '—'
  return `${formatBytes(bps)}/s`
}

function formatEta(seconds: number): string {
  if (seconds < 60) return `${seconds}s`
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${seconds % 60}s`
  return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`
}

function formatProgress(percentage: number): string {
  return `${percentage}%`
}

async function onInstall() {
  if (!selectedModelId.value) return
  if (!llm.disclaimerAccepted) {
    disclaimerOpen.value = true
    return
  }
  try {
    await llm.installModel(auth.token || '', selectedModelId.value)
  } catch (e) {
    ElMessage.error(e instanceof Error ? e.message : String(e))
  }
}

async function onCancel() {
  await llm.cancelInstall(auth.token || '')
  ElMessage.info(t('settings.ai.fallback.cancelled'))
}

async function onPickLocalFile() {
  if (!selectedModelId.value) return
  // Use Tauri's open dialog so we get a real native file picker.
  const { open } = await import('@tauri-apps/api/dialog')
  const picked = await open({
    multiple: false,
    directory: false,
    filters: [{ name: 'GGUF', extensions: ['gguf'] }]
  })
  if (!picked || typeof picked !== 'string') return
  try {
    await llm.importLocal(auth.token || '', selectedModelId.value, picked)
    ElMessage.success(t('settings.ai.fallback.importSuccess'))
  } catch (e) {
    ElMessage.error(e instanceof Error ? e.message : String(e))
  }
}

async function onStartServer() {
  try {
    await llm.startServer(auth.token || '')
    ElMessage.success(t('settings.ai.fallback.serverStarted'))
  } catch (e) {
    ElMessage.error(e instanceof Error ? e.message : String(e))
  }
}

async function onStopServer() {
  try {
    await llm.stopServer(auth.token || '')
    ElMessage.success(t('settings.ai.fallback.serverStopped'))
  } catch (e) {
    ElMessage.error(e instanceof Error ? e.message : String(e))
  }
}

async function onRemove() {
  try {
    await ElMessageBox.confirm(
      t('settings.ai.fallback.removeConfirm'),
      t('common.confirm'),
      { type: 'warning' }
    )
  } catch {
    return
  }
  try {
    await llm.removeModel(auth.token || '')
    ElMessage.success(t('settings.ai.fallback.removed'))
  } catch (e) {
    ElMessage.error(e instanceof Error ? e.message : String(e))
  }
}
</script>

<style scoped lang="scss">
.local-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.row {
  display: flex;
  gap: 32px;
  flex-wrap: wrap;
  margin-bottom: 8px;
}
.meta {
  display: flex;
  flex-direction: column;
  gap: 2px;
  .label {
    font-size: 12px;
    color: var(--el-text-color-secondary);
  }
  .value {
    font-size: 14px;
  }
}
.progress-meta {
  display: flex;
  align-items: baseline;
  gap: 14px;
  margin-bottom: 6px;
  font-size: 13px;
  flex-wrap: wrap;
  .stage {
    color: var(--el-text-color-secondary);
    font-weight: 500;
  }
  .counter {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 14px;
    font-weight: 600;
    color: var(--el-text-color-primary);
    .counter-sep {
      color: var(--el-text-color-placeholder);
      margin: 0 4px;
    }
  }
  .speed {
    color: var(--el-color-primary);
    font-variant-numeric: tabular-nums;
  }
  .eta {
    color: var(--el-text-color-secondary);
    font-variant-numeric: tabular-nums;
  }
}
.stall-hint {
  margin-top: 8px;
}
.speed-results {
  margin-top: 12px;
}
.speed-hint {
  margin-top: 8px;
}
</style>