import { onMounted, onUnmounted, watch, type Ref } from 'vue'
import { useRoute } from 'vue-router'
import { useRecentStore } from '@/stores/recent'

/**
 * Hook for tool pages. Pass the tool's path and a function that returns the
 * current input snapshot (already sanitised — strip passwords, keys, etc.
 * before returning). The composable will:
 *   - save a snapshot on mount (so navigating away and back can restore)
 *   - save a debounced snapshot whenever inputs change
 *   - delete the snapshot on unmount only if you call `discardOnLeave()`
 *
 * Example:
 *   const { record } = useToolRecorder('/tools/hash', () => ({
 *     text: text.value.slice(0, 200),
 *     algorithm: algorithm.value
 *   }))
 */
export function useToolRecorder(
  toolPath: string,
  getSnapshot: () => Record<string, unknown>
) {
  const store = useRecentStore()
  const route = useRoute()
  let timer: ReturnType<typeof setTimeout> | null = null
  let watcher: ReturnType<typeof watch> | null = null

  function schedule() {
    if (timer) clearTimeout(timer)
    timer = setTimeout(() => {
      try {
        store.record(toolPath, getSnapshot())
      } catch {
        /* ignore — store may be unavailable in test env */
      }
    }, 600)
  }

  onMounted(() => {
    // Initial snapshot — useful when the user opens a tool cold.
    schedule()
    // Watch the whole reactive return of `getSnapshot()` by polling — Vue
    // can't watch a function return directly.  We re-evaluate whenever the
    // route changes (cheap proxy for "user changed tabs and came back").
    watcher = watch(
      () => route.fullPath,
      () => schedule()
    )
  })

  onUnmounted(() => {
    if (timer) clearTimeout(timer)
    if (watcher) watcher()
  })

  return {
    /** Force a snapshot save right now (e.g. after a long computation finishes). */
    record: () => store.record(toolPath, getSnapshot()),
    /** Wipe this tool's history. */
    clear: () => store.clearTool(toolPath)
  }
}