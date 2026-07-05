<template>
  <el-dialog
    v-model="store.open"
    :title="t('palette.title')"
    width="640"
    align-center
    :show-close="false"
    :modal-class="'palette-modal'"
    :append-to-body="true"
  >
    <el-input
      ref="inputRef"
      v-model="query"
      :placeholder="t('palette.placeholder')"
      size="large"
      clearable
      :prefix-icon="Search"
    />
    <div class="palette-results">
      <template v-if="filtered.length">
        <div
          v-for="(item, i) in filtered"
          :key="item.path + i"
          class="palette-item"
          :class="{ active: i === activeIndex }"
          @mouseenter="activeIndex = i"
          @click="go(item)"
        >
          <el-icon v-if="item.icon" class="icon"><component :is="item.icon" /></el-icon>
          <div class="meta">
            <div class="label">{{ item.label }}</div>
            <div class="hint">{{ item.hint }}</div>
          </div>
          <el-tag v-if="item.kind === 'tool'" size="small" type="info">{{ t('palette.kind.tool') }}</el-tag>
          <el-tag v-else-if="item.kind === 'settings'" size="small" type="warning">{{ t('palette.kind.settings') }}</el-tag>
          <el-tag v-else size="small">{{ t('palette.kind.page') }}</el-tag>
        </div>
      </template>
      <div v-else class="palette-empty">
        {{ t('palette.empty') }}
      </div>
    </div>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { Search, ChatLineRound } from '@element-plus/icons-vue'
import { usePaletteStore } from '@/stores/palette'
import { useAuthStore } from '@/stores/auth'
import { useRecentStore } from '@/stores/recent'

const { t } = useI18n()
const router = useRouter()
const route = useRoute()
const store = usePaletteStore()
const auth = useAuthStore()
const recent = useRecentStore()

interface PaletteItem {
  label: string
  hint: string
  path: string
  icon?: any
  kind: 'page' | 'tool' | 'settings'
}

const query = ref('')
const activeIndex = ref(0)
const inputRef = ref<{ focus: () => void } | null>(null)

const items = computed<PaletteItem[]>(() => {
  // Build from the route table — every route with a `meta.title` is searchable.
  // icon comes from Element Plus icons-vue; we map by icon string for the common ones.
  const list: PaletteItem[] = []
  for (const r of router.getRoutes()) {
    if (!r.meta?.title || (r.meta as any).publicRoute) continue
    list.push({
      label: t(String(r.meta.title)),
      hint: r.path,
      path: r.path,
      kind: r.path.startsWith('/tools/') ? 'tool' : (r.path === '/system/settings' ? 'settings' : 'page')
    })
  }
  // Inject a couple of shortcut entries users almost always want.
  if (auth.hasPermission('backup:manage')) {
    list.push({
      label: t('backups.title'),
      hint: '/system/backups',
      path: '/system/backups',
      kind: 'settings'
    })
  }
  return list
})

const filtered = computed(() => {
  const q = query.value.trim()
  // "Ask AI" — type `? ` (question mark + space) to route the rest of the
  // query to the AI chat tool as a pre-filled message.
  if (q.startsWith('?') && q.length > 1) {
    const askText = q.slice(1).trim()
    return [{
      label: askText ? `${t('palette.askAi')}: ${askText}` : t('palette.askAi'),
      hint: '/ai/chat',
      path: '/ai/chat',
      icon: ChatLineRound,
      kind: 'tool' as const
    }]
  }
  if (!q) {
    // Empty query: surface favorites first, then the rest.  This makes the
    // palette feel like a "quick launcher" without typing.
    const favs = recent.favoritesList
      .map((p) => items.value.find((it) => it.path === p))
      .filter((x): x is PaletteItem => !!x)
    const rest = items.value.filter((it) => !recent.isFavorite(it.path))
    return [...favs, ...rest].slice(0, 12)
  }
  return items.value
    .filter((it) => it.label.toLowerCase().includes(q.toLowerCase()) || it.path.toLowerCase().includes(q.toLowerCase()))
    .slice(0, 20)
})

watch(filtered, () => { activeIndex.value = 0 })

function go(item: PaletteItem) {
  const q = query.value.trim()
  // vue-router's HistoryState is `Record<string, any>`; we only set `prefill`.
  const state: { prefill?: string } = {}
  if (q.startsWith('?') && q.length > 1) {
    state.prefill = q.slice(1).trim()
  }
  router.push({ path: item.path, state }).then(() => {
    store.hide()
    query.value = ''
  }).catch(() => {
    ElMessage.warning(t('palette.permissionDenied'))
  })
}

function onKey(e: KeyboardEvent) {
  // Ctrl+K / Cmd+K toggles the palette globally.  Even if our backend
  // stores `ui.command_palette=false` later, the binding lives here so the
  // store can simply choose not to call store.show().
  if ((e.ctrlKey || e.metaKey) && (e.key === 'k' || e.key === 'K')) {
    e.preventDefault()
    store.toggle()
    nextTick(() => inputRef.value?.focus())
  }
  // Escape closes.
  if (e.key === 'Escape' && store.open) {
    store.hide()
  }
}

function onPaletteKey(e: KeyboardEvent) {
  if (!store.open) return
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    activeIndex.value = Math.min(filtered.value.length - 1, activeIndex.value + 1)
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    activeIndex.value = Math.max(0, activeIndex.value - 1)
  } else if (e.key === 'Enter') {
    e.preventDefault()
    const it = filtered.value[activeIndex.value]
    if (it) go(it)
  }
}

onMounted(() => {
  window.addEventListener('keydown', onKey)
  window.addEventListener('keydown', onPaletteKey)
})
onUnmounted(() => {
  window.removeEventListener('keydown', onKey)
  window.removeEventListener('keydown', onPaletteKey)
})

watch(() => store.open, (v) => {
  if (v) nextTick(() => inputRef.value?.focus())
  else query.value = ''
})
</script>

<style scoped lang="scss">
.palette-results {
  margin-top: 12px;
  max-height: 420px;
  overflow-y: auto;
  border: 1px solid var(--border-color, #e5e6eb);
  border-radius: 6px;
}
.palette-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 14px;
  cursor: pointer;
  user-select: none;
  border-bottom: 1px solid var(--border-color, #e5e6eb);
  &:last-child { border-bottom: none; }
  &.active {
    background: var(--el-color-primary-light-9, #ecf5ff);
  }
  .icon { color: var(--text-secondary); }
  .meta { flex: 1; min-width: 0; }
  .label { font-size: 14px; font-weight: 500; }
  .hint { font-size: 12px; color: var(--text-secondary); margin-top: 2px; }
}
.palette-empty {
  padding: 24px;
  text-align: center;
  color: var(--text-secondary);
}
</style>