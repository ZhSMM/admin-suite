import { defineStore } from 'pinia'
import { ElMessage } from 'element-plus'
import { chatHistoryApi, type ChatMessageNode, type ChatSession, type ChatSessionWithTree, type ExportFormat } from '@/api/chat-history'
import { useAuthStore } from './auth'

interface State {
  sessions: ChatSession[]
  archivedExpanded: boolean
  currentSessionId: number | null
  /** Tree of messages for the current session; forest of roots. */
  tree: ChatMessageNode[]
  /** Active path: array of node ids from root → current leaf. */
  activePath: number[]
  /** Stream state — when present, the corresponding node's content is
   * being mutated in-place. */
  streamingId: number | null
  loading: boolean
  loadingDetail: boolean
  sending: boolean
  error: string | null
  /** Free-text search query for sidebar. */
  search: string
}

export const useChatHistoryStore = defineStore('chatHistory', {
  state: (): State => ({
    sessions: [],
    archivedExpanded: false,
    currentSessionId: null,
    tree: [],
    activePath: [],
    streamingId: null,
    loading: false,
    loadingDetail: false,
    sending: false,
    error: null,
    search: ''
  }),
  getters: {
    currentSession(state): ChatSession | null {
      return state.sessions.find((s) => s.id === state.currentSessionId) ?? null
    },
    /** Flatten tree along `activePath` into [{id, role, content}, ...] —
     * what the LLM needs as context. */
    activeMessages(state): { id: number; role: string; content: string }[] {
      const out: { id: number; role: string; content: string }[] = []
      let level: ChatMessageNode[] | undefined = state.tree
      for (const id of state.activePath) {
        const node: ChatMessageNode | undefined = level?.find((n) => n.id === id)
        if (!node) break
        out.push({ id: node.id, role: node.role, content: node.content })
        level = node.children
      }
      return out
    }
  },
  actions: {
    /** Recompute the active path to "newest sibling at every level". */
    defaultActivePath() {
      const path: number[] = []
      let level: ChatMessageNode[] | undefined = this.tree
      while (level && level.length > 0) {
        const node: ChatMessageNode = level.reduce((a, b) => (a.created_at > b.created_at ? a : b))
        path.push(node.id)
        level = node.children
      }
      this.activePath = path
    },
    async fetchSessions() {
      const auth = useAuthStore()
      this.loading = true
      this.error = null
      try {
        this.sessions = await chatHistoryApi.list(auth.token || '', {
          archived: this.archivedExpanded,
          search: this.search
        })
      } catch (e) {
        this.error = e instanceof Error ? e.message : String(e)
      } finally {
        this.loading = false
      }
    },
    async createSession(opts: { title?: string; providerId?: string; modelId?: string } = {}): Promise<ChatSession | null> {
      const auth = useAuthStore()
      try {
        const s = await chatHistoryApi.create(auth.token || '', opts)
        this.sessions.unshift(s)
        return s
      } catch (e) {
        ElMessage.error(e instanceof Error ? e.message : String(e))
        return null
      }
    },
    async openSession(id: number) {
      const auth = useAuthStore()
      this.loadingDetail = true
      this.error = null
      try {
        const data: ChatSessionWithTree = await chatHistoryApi.get(auth.token || '', id)
        this.currentSessionId = data.session.id
        this.tree = data.tree
        this.defaultActivePath()
        // Update the sidebar row's updated_at lazily; cheaper than refetch.
        const row = this.sessions.find((s) => s.id === id)
        if (row) {
          row.updated_at = data.session.updated_at
          row.message_count = data.session.message_count
        }
      } catch (e) {
        ElMessage.error(e instanceof Error ? e.message : String(e))
      } finally {
        this.loadingDetail = false
      }
    },
    async deleteSession(id: number) {
      const auth = useAuthStore()
      try {
        await chatHistoryApi.delete(auth.token || '', id)
        this.sessions = this.sessions.filter((s) => s.id !== id)
        if (this.currentSessionId === id) {
          this.currentSessionId = null
          this.tree = []
          this.activePath = []
        }
      } catch (e) {
        ElMessage.error(e instanceof Error ? e.message : String(e))
      }
    },
    async renameSession(id: number, title: string) {
      const auth = useAuthStore()
      try {
        const s = await chatHistoryApi.update(auth.token || '', { id, title })
        const row = this.sessions.find((r) => r.id === id)
        if (row) {
          row.title = s.title
          row.updated_at = s.updated_at
        }
      } catch (e) {
        ElMessage.error(e instanceof Error ? e.message : String(e))
      }
    },
    async toggleArchived(id: number, archived: boolean) {
      const auth = useAuthStore()
      try {
        const s = await chatHistoryApi.update(auth.token || '', { id, archived })
        const idx = this.sessions.findIndex((r) => r.id === id)
        if (idx >= 0) this.sessions.splice(idx, 1, s)
      } catch (e) {
        ElMessage.error(e instanceof Error ? e.message : String(e))
      }
    },
    /** Navigate to a sibling node id. `nodeId` must be a child of the
     * node currently at `level` in `activePath`. */
    async setActiveToNode(nodeId: number) {
      // Walk tree looking for nodeId, build prefix.
      const prefix: number[] = []
      const find = (level: ChatMessageNode[]): boolean => {
        for (const n of level) {
          prefix.push(n.id)
          if (n.id === nodeId) return true
          if (find(n.children)) return true
          prefix.pop()
        }
        return false
      }
      if (!find(this.tree)) return
      this.activePath = prefix
    },
    /** Append a user message to the active leaf, then send the LLM call.
     * Streams → appends an empty assistant node first, sets streamingId,
     * caller polls `streamingId` node and updates content.
     *
     * Returns the user message id (parent_id for the assistant node).
     */
    async appendUserAndSend(opts: {
      content: string
      role?: 'user' | 'system'
      providerId: string
      modelId: string
      runAssistant: (parentId: number, onDone: (assistantId: number, full: string, error?: string) => Promise<void>) => Promise<void>
    }): Promise<number | null> {
      if (!this.currentSessionId) return null
      const auth = useAuthStore()
      this.sending = true
      const parentId = this.activePath.length === 0 ? null : this.activePath[this.activePath.length - 1]
      try {
        // 1. persist user
        const user = await chatHistoryApi.append(auth.token || '', {
          sessionId: this.currentSessionId,
          parentId,
          role: opts.role ?? 'user',
          content: opts.content,
          providerId: opts.providerId,
          modelId: opts.modelId,
          status: 'done'
        })
        // 2. attach to local tree
        this.attachNodeLocally(user)
        this.activePath = [...this.activePath, user.id]
        // 3. create empty assistant and stream
        await opts.runAssistant(user.id, async (assistantId, full, error) => {
          await chatHistoryApi.updateMessage(auth.token || '', {
            id: assistantId,
            content: full,
            status: error ? 'error' : 'done',
            error: error ?? undefined
          })
        })
        return user.id
      } catch (e) {
        ElMessage.error(e instanceof Error ? e.message : String(e))
        return null
      } finally {
        this.sending = false
        this.streamingId = null
      }
    },
    /** Persist an empty assistant node as a child of `parentId`, mark it
     * streaming locally, return its id. The caller updates its content
     * via `updateMessage`. */
    async appendAssistantPlaceholder(parentId: number, providerId: string, modelId: string): Promise<ChatMessageNode | null> {
      if (!this.currentSessionId) return null
      const auth = useAuthStore()
      try {
        const node = await chatHistoryApi.append(auth.token || '', {
          sessionId: this.currentSessionId,
          parentId,
          role: 'assistant',
          content: '',
          providerId,
          modelId,
          status: 'streaming'
        })
        this.attachNodeLocally(node)
        this.activePath = [...this.activePath, node.id]
        this.streamingId = node.id
        return node
      } catch (e) {
        ElMessage.error(e instanceof Error ? e.message : String(e))
        return null
      }
    },
    /** Update the content of a node in-place while streaming. */
    patchNodeContent(nodeId: number, content: string) {
      const find = (level: ChatMessageNode[]): boolean => {
        for (const n of level) {
          if (n.id === nodeId) {
            n.content = content
            return true
          }
          if (find(n.children)) return true
        }
        return false
      }
      find(this.tree)
    },
    async deleteMessage(id: number) {
      const auth = useAuthStore()
      try {
        await chatHistoryApi.deleteMessage(auth.token || '', id)
        // Remove from tree
        const remove = (level: ChatMessageNode[]): boolean => {
          for (let i = 0; i < level.length; i++) {
            if (level[i].id === id) {
              level.splice(i, 1)
              return true
            }
            if (remove(level[i].children)) return true
          }
          return false
        }
        remove(this.tree)
        // Drop from activePath if it was there
        this.activePath = this.activePath.filter((x) => x !== id)
      } catch (e) {
        ElMessage.error(e instanceof Error ? e.message : String(e))
      }
    },
    async regenerate(assistantId: number, runAgain: (parentId: number, onDone: (id: number, full: string, error?: string) => Promise<void>) => Promise<void>) {
      const auth = useAuthStore()
      // Find this assistant's parent id
      let parentId: number | null = null
      const find = (level: ChatMessageNode[]): boolean => {
        for (const n of level) {
          if (n.id === assistantId) {
            parentId = n.parent_id
            return true
          }
          if (find(n.children)) return true
        }
        return false
      }
      find(this.tree)
      if (parentId == null) return
      // Position activePath to `parentId`, drop the assistant and any descendants.
      const idx = this.activePath.indexOf(parentId)
      if (idx >= 0) {
        this.activePath = this.activePath.slice(0, idx + 1)
      }
      this.sending = true
      try {
        const node = await chatHistoryApi.append(auth.token || '', {
          sessionId: this.currentSessionId!,
          parentId,
          role: 'assistant',
          content: '',
          providerId: '',
          modelId: '',
          status: 'streaming'
        })
        this.attachNodeLocally(node)
        this.activePath = [...this.activePath, node.id]
        this.streamingId = node.id
        await runAgain(parentId, async (assistantId, full, error) => {
          await chatHistoryApi.updateMessage(auth.token || '', {
            id: assistantId,
            content: full,
            status: error ? 'error' : 'done',
            error: error ?? undefined
          })
        })
      } catch (e) {
        ElMessage.error(e instanceof Error ? e.message : String(e))
      } finally {
        this.sending = false
        this.streamingId = null
      }
    },
    async exportSession(format: ExportFormat): Promise<{ filename: string; mime: string; content: string } | null> {
      if (!this.currentSessionId) return null
      const auth = useAuthStore()
      try {
        const res = await chatHistoryApi.export(auth.token || '', {
          id: this.currentSessionId,
          format,
          activePathIds: this.activePath
        })
        return res
      } catch (e) {
        ElMessage.error(e instanceof Error ? e.message : String(e))
        return null
      }
    },
    /** Insert a `node` into the local tree at the correct parent. */
    attachNodeLocally(node: ChatMessageNode) {
      if (node.parent_id == null) {
        this.tree.push({ ...node, children: [] })
        return
      }
      const insert = (level: ChatMessageNode[]): boolean => {
        for (const n of level) {
          if (n.id === node.parent_id) {
            n.children.push({ ...node, children: [] })
            return true
          }
          if (insert(n.children)) return true
        }
        return false
      }
      insert(this.tree)
    },
    /** Find siblings of `nodeId` at the same parent. */
    siblingsOf(nodeId: number): ChatMessageNode[] {
      let parentChildren: ChatMessageNode[] | null = null
      const findParent = (level: ChatMessageNode[]): boolean => {
        for (const n of level) {
          if (n.children.some((c) => c.id === nodeId)) {
            parentChildren = n.children
            return true
          }
          if (findParent(n.children)) return true
        }
        return false
      }
      // root-level
      if (this.tree.some((n) => n.id === nodeId)) {
        return this.tree
      }
      findParent(this.tree)
      return parentChildren ?? []
    }
  }
})
