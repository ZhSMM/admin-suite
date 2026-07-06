import { call } from './index'

// ---------------------------------------------------------------------------
// Wire shapes — mirror `commands::chat_history` in Rust.
// ---------------------------------------------------------------------------

export interface ChatSession {
  id: number
  user_id: number
  title: string
  provider_id: string
  model_id: string
  created_at: string
  updated_at: string
  archived: boolean
  message_count: number
}

export interface ChatMessageNode {
  id: number
  session_id: number
  parent_id: number | null
  role: 'user' | 'assistant' | 'system' | string
  content: string
  provider_id: string
  model_id: string
  status: 'done' | 'streaming' | 'error' | string
  error: string | null
  created_at: string
  children: ChatMessageNode[]
}

export interface ChatSessionWithTree {
  session: ChatSession
  tree: ChatMessageNode[]
}

export type ExportFormat = 'json' | 'markdown' | 'html'

export interface ExportResult {
  filename: string
  mime: string
  content: string
}

// ---------------------------------------------------------------------------
// API
// ---------------------------------------------------------------------------

export const chatHistoryApi = {
  list(token: string, args: { archived?: boolean; search?: string } = {}): Promise<ChatSession[]> {
    return call<ChatSession[]>('chat_session_list', { token, args })
  },
  create(token: string, args: { title?: string; providerId?: string; modelId?: string } = {}): Promise<ChatSession> {
    return call<ChatSession>('chat_session_create', { token, args })
  },
  update(token: string, args: { id: number; title?: string; archived?: boolean }): Promise<ChatSession> {
    return call<ChatSession>('chat_session_update', { token, args })
  },
  delete(token: string, id: number): Promise<boolean> {
    return call<boolean>('chat_session_delete', { token, args: { id } })
  },
  get(token: string, id: number): Promise<ChatSessionWithTree> {
    return call<ChatSessionWithTree>('chat_session_get', { token, args: { id } })
  },
  append(
    token: string,
    args: {
      sessionId: number
      parentId: number | null
      role: 'user' | 'assistant' | 'system'
      content: string
      providerId?: string
      modelId?: string
      status?: 'done' | 'streaming' | 'error'
      error?: string
    }
  ): Promise<ChatMessageNode> {
    return call<ChatMessageNode>('chat_message_append', {
      token,
      args: {
        session_id: args.sessionId,
        parent_id: args.parentId,
        role: args.role,
        content: args.content,
        provider_id: args.providerId ?? null,
        model_id: args.modelId ?? null,
        status: args.status ?? null,
        error: args.error ?? null
      }
    })
  },
  updateMessage(
    token: string,
    args: { id: number; content?: string; status?: 'done' | 'streaming' | 'error'; error?: string }
  ): Promise<ChatMessageNode> {
    return call<ChatMessageNode>('chat_message_update', {
      token,
      args: {
        id: args.id,
        content: args.content ?? null,
        status: args.status ?? null,
        error: args.error ?? null
      }
    })
  },
  deleteMessage(token: string, id: number): Promise<boolean> {
    return call<boolean>('chat_message_delete', { token, args: { id } })
  },
  export(token: string, args: { id: number; format: ExportFormat; activePathIds?: number[] | null }): Promise<ExportResult> {
    return call<ExportResult>('chat_session_export', {
      token,
      args: {
        id: args.id,
        format: args.format,
        active_path_ids: args.activePathIds ?? null
      }
    })
  }
}
