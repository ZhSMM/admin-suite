<template>
  <div class="page-container chat-page">
    <div class="page-header">
      <h2>{{ t('ai.chat.title') }} <el-tag size="small" type="info" style="margin-left:8px">v0.7</el-tag></h2>
      <el-select v-model="providerId" :placeholder="t('ai.chat.provider')" style="width: 200px" @change="onProviderChange">
        <el-option
          v-for="p in llm.enabledProviders"
          :key="p.id"
          :label="p.name"
          :value="p.id"
        />
      </el-select>
      <el-select v-model="modelId" :placeholder="t('ai.chat.model')" style="width: 240px; margin-left: 8px" :disabled="!providerId">
        <el-option
          v-for="m in modelsForProvider"
          :key="m.id"
          :label="m.display_name"
          :value="m.id"
        />
      </el-select>
      <el-button
        :icon="Folder"
        :title="t('ai.chat.sessions.title')"
        style="margin-left:8px"
        @click="sidebarOpen = !sidebarOpen"
      />
    </div>

    <el-alert
      v-if="llm.providers.length === 0"
      :title="t('ai.chat.noProvidersTitle')"
      type="info"
      show-icon
      :closable="false"
    >
      <p>{{ t('ai.chat.noProvidersDesc') }}</p>
      <el-button type="primary" size="small" @click="goSettings">
        {{ t('ai.chat.openSettings') }}
      </el-button>
    </el-alert>

    <div v-else class="chat-grid">
      <!-- Sidebar -->
      <aside v-if="sidebarOpen" class="sidebar">
        <div class="sidebar-head">
          <el-button type="primary" :icon="Plus" size="small" @click="onNewSession">
            {{ t('ai.chat.sessions.new') }}
          </el-button>
        </div>
        <el-input
          v-model="search"
          :placeholder="t('ai.chat.sessions.search')"
          clearable
          size="small"
          @input="onSearch"
        />
        <div class="session-list" v-loading="chat.loading">
          <div
            v-for="s in chat.sessions"
            :key="s.id"
            :class="['session-item', { active: chat.currentSessionId === s.id }]"
            @click="onOpen(s.id)"
          >
            <div class="session-title" @dblclick.stop="onRename(s)">
              {{ s.title || t('ai.chat.sessions.untitled') }}
            </div>
            <div class="session-meta">
              <span>{{ s.message_count }} {{ t('ai.chat.sessions.msgs') }}</span>
              <span v-if="s.provider_id || s.model_id" class="muted">
                {{ s.provider_id || '?' }} / {{ s.model_id || '?' }}
              </span>
              <el-dropdown trigger="click" @command="(c: string) => onSessionCmd(s, c)">
                <el-button :icon="MoreFilled" link size="small" @click.stop />
                <template #dropdown>
                  <el-dropdown-menu>
                    <el-dropdown-item command="rename">{{ t('ai.chat.sessions.rename') }}</el-dropdown-item>
                    <el-dropdown-item command="export">
                      {{ t('ai.chat.export.title') }}
                    </el-dropdown-item>
                    <el-dropdown-item command="archive">
                      {{ s.archived ? t('ai.chat.sessions.unarchive') : t('ai.chat.sessions.archive') }}
                    </el-dropdown-item>
                    <el-dropdown-item command="delete" divided>
                      {{ t('ai.chat.sessions.delete') }}
                    </el-dropdown-item>
                  </el-dropdown-menu>
                </template>
              </el-dropdown>
            </div>
          </div>
          <div v-if="chat.sessions.length === 0" class="empty">
            {{ t('ai.chat.sessions.empty') }}
          </div>
        </div>
        <div class="sidebar-foot">
          <el-checkbox v-model="chat.archivedExpanded" @change="chat.fetchSessions()">
            {{ t('ai.chat.sessions.showArchived') }}
          </el-checkbox>
        </div>
      </aside>

      <!-- Main -->
      <section class="main">
        <div class="active-indicator muted" v-if="chat.activePath.length > 0">
          {{ t('ai.chat.node.pathIndicator', { n: chat.activePath.length }) }}
        </div>

        <div v-loading="chat.loadingDetail" class="chat-history">
          <template v-for="(node, idx) in visibleMessages" :key="node.id">
            <!-- Per-message render with sibling navigation -->
            <div :class="['msg-row', `msg-row-${node.role}`, { active: chat.activePath.includes(node.id) }]">
              <div class="branch-spine" v-if="hasSiblings(node)" />
              <div :class="['msg', `msg-${node.role}`, { branching: hasSiblings(node) }]">
                <div class="msg-head">
                  <span class="role">{{ roleLabel(node.role) }}</span>
                  <span class="msg-meta muted">
                    {{ node.model_id || '' }}
                  </span>
                  <span v-if="node.status === 'streaming'" class="streaming">
                    {{ t('ai.chat.node.streaming') }}
                  </span>
                </div>
                <div class="msg-body">
                  <pre v-if="node.role === 'system'">{{ node.content }}</pre>
                  <template v-else>
                    <pre v-if="node.content">{{ node.content }}</pre>
                    <pre v-else class="placeholder">{{ t('ai.chat.thinking') }}</pre>
                  </template>
                  <div v-if="node.status === 'error' && node.error" class="err muted">
                    {{ node.error }}
                  </div>
                </div>
                <!-- action bar -->
                <div class="msg-actions">
                  <el-button-group>
                    <el-button
                      v-if="hasPrevSibling(node)"
                      size="small"
                      :icon="ArrowLeft"
                      @click="onSiblingStep(node, -1)"
                    />
                    <el-button
                      v-if="hasSiblings(node)"
                      size="small"
                      text
                      @click="onSiblingNavOpen(node)"
                    >
                      {{ t('ai.chat.node.siblingNav', { n: siblingIndex(node) + 1, total: siblingsOf(node).length }) }}
                    </el-button>
                    <el-button
                      v-if="hasNextSibling(node)"
                      size="small"
                      :icon="ArrowRight"
                      @click="onSiblingStep(node, 1)"
                    />
                  </el-button-group>
                  <el-button size="small" :icon="CopyDocument" text @click="copyNode(node)">
                    {{ t('ai.chat.node.copy') }}
                  </el-button>
                  <el-button
                    v-if="node.role === 'assistant' && !chat.sending"
                    size="small"
                    :icon="Refresh"
                    text
                    @click="onRegenerate(node)"
                  >
                    {{ t('ai.chat.node.regenerate') }}
                  </el-button>
                  <el-button size="small" :icon="Delete" text @click="chat.deleteMessage(node.id)">
                    {{ t('ai.chat.node.delete') }}
                  </el-button>
                </div>
              </div>
            </div>
          </template>
        </div>

        <div class="chat-input">
          <el-input
            v-model="input"
            type="textarea"
            :rows="3"
            :placeholder="t('ai.chat.inputPlaceholder')"
            :disabled="!providerId || !modelId || chat.sending"
            @keydown.enter.exact.prevent="onSend"
          />
          <div class="actions">
            <el-button :disabled="!providerId || !modelId" @click="onNewSession">
              {{ t('ai.chat.sessions.new') }}
            </el-button>
            <el-button
              type="primary"
              :loading="chat.sending"
              :disabled="!input.trim() || !providerId || !modelId"
              @click="onSend"
            >
              {{ t('ai.chat.send') }}
            </el-button>
          </div>
        </div>
      </section>
    </div>

    <!-- Sibling navigator (popover-style for now; anchored row above the action bar) -->
    <el-dialog v-model="siblingPickerOpen" :title="t('ai.chat.node.siblingNav', { n: 1, total: 1 })" width="540px">
      <div class="sibling-list">
        <div
          v-for="sib in siblingPickerList"
          :key="sib.id"
          :class="['sibling-item', { active: chat.activePath.includes(sib.id) }]"
          @click="onPickSibling(sib)"
        >
          <span class="sibling-role muted">{{ roleLabel(sib.role) }}</span>
          <span class="sibling-snippet">{{ sib.content.slice(0, 120) || '…' }}</span>
        </div>
      </div>
    </el-dialog>

    <!-- Export preview modal: showing content + Copy + Close -->
    <el-dialog
      v-model="exportModal.open"
      :title="t('ai.chat.export.title') + ' · ' + exportModal.format"
      width="720px"
    >
      <el-input
        v-model="exportModal.content"
        type="textarea"
        :rows="20"
        :readonly="true"
        style="font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace; font-size: 12px"
      />
      <template #footer>
        <el-button @click="exportModal.open = false">{{ t('common.close') }}</el-button>
        <el-button type="primary" :icon="CopyDocument" @click="copyExport">
          {{ t('ai.chat.export.copy') }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onActivated, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import { useRouter } from 'vue-router'
import {
  ArrowLeft,
  ArrowRight,
  CopyDocument,
  Delete,
  Folder,
  MoreFilled,
  Plus,
  Refresh
} from '@element-plus/icons-vue'
import { useAuthStore } from '@/stores/auth'
import { useLlmStore } from '@/stores/llm'
import { useChatHistoryStore } from '@/stores/chat-history'
import { llmApi, type ChatMessage } from '@/api/llm'
import type { ChatMessageNode } from '@/api/chat-history'

const { t } = useI18n()
const router = useRouter()
const auth = useAuthStore()
const llm = useLlmStore()
const chat = useChatHistoryStore()

const sidebarOpen = ref(true)
const providerId = ref<string>(llm.effectiveProviderId ?? '')
const modelId = ref<string>(llm.effectiveModelId ?? '')
const input = ref<string>(
  (history.state && typeof history.state.prefill === 'string') ? history.state.prefill : ''
)
const search = ref('')

const modelsForProvider = computed(() =>
  providerId.value ? llm.modelsFor(providerId.value) : []
)

watch(providerId, () => {
  modelId.value = ''
})

const onProviderChange = () => {
  if (modelsForProvider.value.length > 0 && !modelId.value) {
    modelId.value = modelsForProvider.value[0].id
  }
}

const roleLabel = (r: string) => {
  if (r === 'user') return t('ai.chat.user')
  if (r === 'assistant') return t('ai.chat.assistant')
  if (r === 'system') return t('ai.chat.node.system')
  return r
}

const goSettings = () => router.push('/system/llm/providers')

// ----- tree walk helpers -----
/** Linear message list along the active path (branch off-path siblings not shown here). */
const visibleMessages = computed<ChatMessageNode[]>(() => {
  const out: ChatMessageNode[] = []
  const walk = (level: ChatMessageNode[] | undefined, depth: number, wantId: number | null): ChatMessageNode | null => {
    if (!level || level.length === 0) return null
    for (const n of level) {
      if (depth === 0) {
        if (chat.activePath[0] === n.id) {
          out.push(n)
          return walk(n.children, 1, chat.activePath[1] ?? null)
        }
      } else {
        if (wantId == null) return n
        if (n.id === wantId) {
          out.push(n)
          return walk(n.children, depth + 1, chat.activePath[depth + 1] ?? null)
        }
      }
    }
    return null
  }
  walk(chat.tree, 0, null)
  return out
})

function siblingsOf(node: ChatMessageNode): ChatMessageNode[] {
  return chat.siblingsOf(node.id)
}
function hasSiblings(node: ChatMessageNode): boolean {
  return siblingsOf(node).length > 1
}
function hasPrevSibling(node: ChatMessageNode): boolean {
  const xs = siblingsOf(node)
  const i = xs.findIndex((x) => x.id === node.id)
  return i > 0
}
function hasNextSibling(node: ChatMessageNode): boolean {
  const xs = siblingsOf(node)
  const i = xs.findIndex((x) => x.id === node.id)
  return i >= 0 && i < xs.length - 1
}
function siblingIndex(node: ChatMessageNode): number {
  const xs = siblingsOf(node)
  return xs.findIndex((x) => x.id === node.id)
}

const siblingPickerOpen = ref(false)
const siblingPickerList = ref<ChatMessageNode[]>([])
const siblingPickerTarget = ref<ChatMessageNode | null>(null)

function onSiblingNavOpen(node: ChatMessageNode) {
  siblingPickerTarget.value = node
  siblingPickerList.value = siblingsOf(node)
  siblingPickerOpen.value = true
}
async function onPickSibling(node: ChatMessageNode) {
  siblingPickerOpen.value = false
  await chat.setActiveToNode(node.id)
}

async function onSiblingStep(node: ChatMessageNode, dir: -1 | 1) {
  const xs = siblingsOf(node)
  const i = xs.findIndex((x) => x.id === node.id)
  const target = xs[i + dir]
  if (target) await chat.setActiveToNode(target.id)
}

// ----- mutations -----
async function copyNode(node: ChatMessageNode) {
  try {
    await navigator.clipboard.writeText(node.content)
    ElMessage.success(t('common.copySuccess'))
  } catch {
    ElMessage.error(t('common.copyFailed'))
  }
}

async function onSend() {
  if (!providerId.value || !modelId.value || !input.value.trim()) return
  // create session lazily if needed
  let sessionId = chat.currentSessionId
  if (!sessionId) {
    const s = await chat.createSession({
      title: input.value.trim().slice(0, 60),
      providerId: providerId.value,
      modelId: modelId.value
    })
    if (!s) return
    sessionId = s.id
    await chat.openSession(sessionId)
  } else {
    // sync provider/model if changed on top bar; persist them on session
    if (providerId.value !== chat.currentSession?.provider_id || modelId.value !== chat.currentSession?.model_id) {
      // Cheaper than re-creating: silently update.
      chat.renameSession(sessionId, chat.currentSession?.title ?? '').catch(() => undefined)
    }
  }
  const userMsg = input.value.trim()
  input.value = ''
  await chat.appendUserAndSend({
    content: userMsg,
    providerId: providerId.value,
    modelId: modelId.value,
    runAssistant: async (parentId, onDone) => {
      // Synchronous LLM call (v0.7.0 doesn't yet poll streaming).
      const placeholder = await chat.appendAssistantPlaceholder(parentId, providerId.value, modelId.value)
      if (!placeholder) return
      try {
        const llmMsgs: ChatMessage[] = chat.activeMessages.map((m) => ({
          role: m.role as ChatMessage['role'],
          content: m.content
        }))
        const result = await llmApi.chat(auth.token || '', {
          provider_id: providerId.value,
          model_id: modelId.value,
          messages: llmMsgs
        })
        chat.patchNodeContent(placeholder.id, result.content)
        await onDone(placeholder.id, result.content)
        llm.setDefault(providerId.value, modelId.value)
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e)
        chat.patchNodeContent(placeholder.id, msg)
        await onDone(placeholder.id, msg, msg)
      }
    }
  })
}

async function onRegenerate(node: ChatMessageNode) {
  if (node.role !== 'assistant' || !providerId.value || !modelId.value) return
  await chat.regenerate(node.id, async (parentId, onDone) => {
    const placeholder = await chat.appendAssistantPlaceholder(parentId, providerId.value, modelId.value)
    if (!placeholder) return
    try {
      const llmMsgs: ChatMessage[] = chat.activeMessages.map((m) => ({
        role: m.role as ChatMessage['role'],
        content: m.content
      }))
      const result = await llmApi.chat(auth.token || '', {
        provider_id: providerId.value,
        model_id: modelId.value,
        messages: llmMsgs
      })
      chat.patchNodeContent(placeholder.id, result.content)
      await onDone(placeholder.id, result.content)
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      chat.patchNodeContent(placeholder.id, msg)
      await onDone(placeholder.id, msg, msg)
    }
  })
}

// ----- sidebar -----
async function onNewSession() {
  const s = await chat.createSession({
    providerId: providerId.value,
    modelId: modelId.value
  })
  if (s) await chat.openSession(s.id)
}
async function onOpen(id: number) {
  await chat.openSession(id)
}
async function onSearch() {
  await chat.fetchSessions()
}
async function onRename(s: { id: number; title: string }) {
  try {
    const { value } = await ElMessageBox.prompt(t('ai.chat.sessions.rename'), t('ai.chat.sessions.title'), {
      inputValue: s.title || '',
      confirmButtonText: t('common.save'),
      cancelButtonText: t('common.cancel')
    })
    if (typeof value === 'string') {
      await chat.renameSession(s.id, value.trim() || t('ai.chat.sessions.untitled'))
    }
  } catch {
    // user dismissed
  }
}
async function onSessionCmd(s: { id: number; title: string }, cmd: string) {
  if (cmd === 'rename') return onRename(s)
  if (cmd === 'archive') return chat.toggleArchived(s.id, !chat.sessions.find((x) => x.id === s.id)?.archived)
  if (cmd === 'delete') {
    try {
      await ElMessageBox.confirm(t('ai.chat.sessions.deleteConfirm'), t('common.confirm'), {
        type: 'warning'
      })
      await chat.deleteSession(s.id)
    } catch { /* dismissed */ }
  }
  if (cmd === 'export') {
    if (chat.currentSessionId !== s.id) await chat.openSession(s.id)
    exportModal.value.open = true
    exportModal.value.format = 'markdown'
    const r = await chat.exportSession('markdown')
    if (r) {
      exportModal.value.content = r.content
      exportModal.value.filename = r.filename
    } else {
      exportModal.value.content = ''
    }
  }
}

// ----- export modal -----
const exportModal = ref<{ open: boolean; format: 'json' | 'markdown' | 'html'; content: string; filename: string }>({
  open: false,
  format: 'markdown',
  content: '',
  filename: ''
})
async function copyExport() {
  try {
    await navigator.clipboard.writeText(exportModal.value.content)
    ElMessage.success(t('common.copySuccess'))
  } catch {
    ElMessage.error(t('common.copyFailed'))
  }
}

onMounted(async () => {
  await llm.loadAll(auth.token || '')
  if (!providerId.value && llm.enabledProviders.length > 0) {
    providerId.value = llm.enabledProviders[0].id
    onProviderChange()
  }
  await chat.fetchSessions()
})

// v0.7.1 — re-pull providers+models every time the user re-enters the page.
// Without this, configuring a new provider/model in /system/llm/models
// then navigating back would leave the dropdowns showing stale empty rows.
onActivated(async () => {
  await llm.loadAll(auth.token || '')
  // If the previously selected provider disappeared, fall back to the
  // first available one and refresh the model default.
  if (providerId.value && !llm.enabledProviders.find((p) => p.id === providerId.value)) {
    providerId.value = llm.enabledProviders[0]?.id ?? ''
    modelId.value = ''
  }
  if (providerId.value && !modelId.value && modelsForProvider.value.length > 0) {
    modelId.value = modelsForProvider.value[0].id
  }
  await chat.fetchSessions()
})
</script>

<style scoped>
.chat-page {
  display: flex;
  flex-direction: column;
  height: calc(100vh - var(--header-height));
}
.chat-grid {
  display: flex;
  flex: 1;
  background: var(--el-bg-color);
  border-radius: 6px;
  overflow: hidden;
}

/* sidebar */
.sidebar {
  width: 280px;
  border-right: 1px solid var(--border-color, #e5e6eb);
  display: flex;
  flex-direction: column;
  background: var(--bg-primary, #fff);
}
.sidebar-head {
  padding: 12px;
  border-bottom: 1px solid var(--border-color, #e5e6eb);
}
.sidebar :deep(.el-input) {
  margin: 8px 12px;
  width: auto;
}
.session-list {
  flex: 1;
  overflow-y: auto;
  padding: 0 8px 8px;
}
.session-item {
  padding: 8px 10px;
  border-radius: 6px;
  cursor: pointer;
  margin-bottom: 4px;
}
.session-item:hover {
  background: var(--el-fill-color-light);
}
.session-item.active {
  background: var(--el-color-primary-light-9);
}
.session-title {
  font-size: 13px;
  font-weight: 600;
  margin-bottom: 4px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.session-meta {
  display: flex;
  gap: 8px;
  align-items: center;
  font-size: 11px;
  color: var(--el-text-color-secondary);
}
.session-meta > :first-child { flex: 0 0 auto; }
.session-meta > :nth-child(2) {
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.session-meta > :last-child {
  flex: 0 0 auto;
}
.empty {
  text-align: center;
  color: var(--el-text-color-secondary);
  padding: 32px 12px;
}
.sidebar-foot {
  border-top: 1px solid var(--border-color, #e5e6eb);
  padding: 8px 12px;
  font-size: 12px;
}

/* main */
.main {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.active-indicator {
  font-size: 11px;
  padding: 6px 16px;
  border-bottom: 1px solid var(--border-color, #e5e6eb);
}
.chat-history {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.msg-row {
  display: flex;
  align-items: stretch;
  gap: 8px;
}
.msg-row-user { justify-content: flex-end; }
.msg-row-assistant,
.msg-row-system { justify-content: flex-start; }
.msg-row .branch-spine {
  width: 8px;
  border-left: 2px solid var(--el-color-primary-light-5);
  margin-left: 22px;
  border-radius: 2px;
  opacity: 0.5;
}
.msg {
  border-radius: 6px;
  padding: 8px 12px;
  max-width: 80%;
}
.msg-user {
  background: var(--el-color-primary-light-9);
}
.msg-assistant {
  background: var(--el-fill-color-light);
}
.msg-system {
  align-self: center;
  background: var(--el-color-warning-light-9);
  font-style: italic;
}
.msg.placeholder pre {
  color: var(--el-text-color-placeholder);
  font-style: italic;
}
.msg-head {
  font-size: 11px;
  color: var(--el-text-color-secondary);
  margin-bottom: 4px;
  display: flex;
  gap: 8px;
  align-items: center;
}
.role { font-weight: 600; }
.streaming { color: var(--el-color-primary); font-style: italic; }
.msg-body pre {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 13px;
}
.err {
  margin-top: 4px;
  font-size: 12px;
}
.msg-actions {
  margin-top: 6px;
  display: flex;
  gap: 6px;
  align-items: center;
  flex-wrap: wrap;
}
.msg.placeholder {
  color: var(--el-text-color-placeholder);
}

.chat-input {
  border-top: 1px solid var(--border-color, #e5e6eb);
  padding: 12px;
  background: var(--bg-primary, #fff);
}
.actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 8px;
}

.sibling-list {
  max-height: 360px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.sibling-item {
  display: flex;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 6px;
  cursor: pointer;
  border: 1px solid var(--border-color, #e5e6eb);
  align-items: flex-start;
}
.sibling-item:hover {
  background: var(--el-fill-color-light);
}
.sibling-item.active {
  border-color: var(--el-color-primary);
  background: var(--el-color-primary-light-9);
}
.sibling-role {
  flex: 0 0 80px;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
.sibling-snippet {
  flex: 1;
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}
</style>
