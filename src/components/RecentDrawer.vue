<template>
  <el-drawer
    v-model="open"
    :title="t('recents.title')"
    direction="rtl"
    size="420"
    :with-header="true"
  >
    <!-- Tabs -->
    <el-tabs v-model="tab" class="recents-tabs">
      <el-tab-pane :label="t('recents.tab.recent')" name="recent">
        <div class="toolbar">
          <span class="hint">{{ t('recents.recentHint') }}</span>
          <el-button
            v-if="Object.keys(store.recent).length"
            text
            type="danger"
            @click="store.clearAll()"
          >
            {{ t('recents.clearAll') }}
          </el-button>
        </div>

        <div v-if="!Object.keys(store.recent).length" class="empty">
          {{ t('recents.empty') }}
        </div>

        <div
          v-for="(entries, toolPath) in store.recent"
          :key="toolPath"
          class="group"
        >
          <div class="group-header">
            <strong>{{ labelFor(toolPath) }}</strong>
            <el-button text size="small" @click="store.clearTool(toolPath)">
              {{ t('common.delete') }}
            </el-button>
          </div>
          <div
            v-for="entry in entries"
            :key="entry.ts"
            class="entry"
            @click="resume(toolPath, entry)"
          >
            <div class="entry-meta">
              <span>{{ formatTime(entry.ts) }}</span>
              <el-button
                text
                size="small"
                @click.stop="store.removeRecent(toolPath, entry.ts)"
              >
                ×
              </el-button>
            </div>
            <div class="entry-preview">
              {{ previewFor(entry) }}
            </div>
          </div>
        </div>
      </el-tab-pane>

      <el-tab-pane :label="t('recents.tab.favorites')" name="favorites">
        <div v-if="!store.favorites.length" class="empty">
          {{ t('recents.favoritesEmpty') }}
        </div>
        <div
          v-for="path in store.favorites"
          :key="path"
          class="entry"
          @click="go(path)"
        >
          <div class="entry-meta">
            <strong>{{ labelFor(path) }}</strong>
            <el-button
              text
              size="small"
              type="danger"
              @click.stop="store.toggleFavorite(path)"
            >
              {{ t('recents.unpin') }}
            </el-button>
          </div>
          <div class="entry-preview">{{ path }}</div>
        </div>
      </el-tab-pane>
    </el-tabs>
  </el-drawer>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useRecentStore } from '@/stores/recent'

const { t } = useI18n()
const router = useRouter()
const store = useRecentStore()

const open = defineModel<boolean>({ default: false })
const tab = ref<'recent' | 'favorites'>('recent')

const routes = computed(() => router.getRoutes())

function labelFor(path: string): string {
  const r = routes.value.find((x) => x.path === path || x.path === path + '/')
  if (r?.meta?.title) {
    const key = String(r.meta.title)
    return t(key)
  }
  return path
}

function previewFor(entry: { inputs: Record<string, unknown>; label?: string }): string {
  if (entry.label) return entry.label
  const values = Object.values(entry.inputs).filter((v) => v != null && v !== '')
  if (!values.length) return '—'
  return values
    .slice(0, 2)
    .map((v) => {
      const s = typeof v === 'string' ? v : JSON.stringify(v)
      return s.length > 60 ? s.slice(0, 60) + '…' : s
    })
    .join(' · ')
}

function formatTime(ts: number): string {
  const d = new Date(ts)
  const now = new Date()
  const sameDay = d.toDateString() === now.toDateString()
  if (sameDay) {
    return d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })
  }
  return d.toLocaleDateString(undefined, { month: '2-digit', day: '2-digit' }) +
    ' ' + d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })
}

function go(path: string) {
  open.value = false
  router.push(path)
}

function resume(toolPath: string, entry: { inputs: Record<string, unknown> }) {
  // Re-opening the same route won't reload the component, so the snapshot
  // would never be re-applied.  We broadcast a custom event that tool pages
  // can listen to (and re-hydrate from `entry.inputs`).
  window.dispatchEvent(
    new CustomEvent('admin-suite:restore-snapshot', {
      detail: { path: toolPath, inputs: entry.inputs }
    })
  )
  open.value = false
  router.push(toolPath)
}
</script>

<style scoped lang="scss">
.recents-tabs {
  margin-top: -16px;
}
.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}
.hint {
  color: var(--text-secondary);
  font-size: 12px;
}
.empty {
  text-align: center;
  color: var(--text-secondary);
  padding: 32px;
}
.group {
  margin-bottom: 16px;
  border: 1px solid var(--border-color, #e5e6eb);
  border-radius: 6px;
  overflow: hidden;
}
.group-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 12px;
  background: var(--bg-secondary, #f5f7fa);
}
.entry {
  padding: 8px 12px;
  border-top: 1px solid var(--border-color, #e5e6eb);
  cursor: pointer;
  user-select: none;
  &:first-child { border-top: none; }
  &:hover {
    background: var(--el-color-primary-light-9, #ecf5ff);
  }
}
.entry-meta {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 12px;
  color: var(--text-secondary);
}
.entry-preview {
  font-family: 'JetBrains Mono', Consolas, monospace;
  font-size: 12px;
  margin-top: 4px;
  word-break: break-all;
  white-space: pre-wrap;
}
</style>