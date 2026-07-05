# v0.6.2 Design — LLM Local Install + Conversation Multi-Level Management

Author: Mavis · Status: **DRAFT, awaiting approval** · Target release: v0.6.2

## 0. TL;DR

Two new features for v0.6.2:

1. **Local-model one-click install** — surface the existing `FallbackManager`
   machinery (already shipped in v0.6.0: model registry, state machine,
   download infra, llama-server spawn) through a dedicated UI panel +
   missing Tauri commands. After this lands, a fresh install with zero
   cloud config can click "Install local AI" and have a working offline
   chat in 2–5 minutes.

2. **Conversation multi-level management (问答多级管理)** — extend the
   flat Chat page into a tree-structured conversation system with
   sessions → threads → branches, plus templates, search, archive, export,
   and RBAC. This is a multi-release scope; v0.6.2 ships the data model
   and the tree UI, v0.6.3 ships templates/export/sharing.

The two features share infrastructure: a new `llm_conversations` table,
LLM streaming (`llm_chat_stream`) finally wired into a real UI, and a
provider-routing layer that respects the user's "local-first" toggle.

---

## 1. Feature A — One-click Local Install

### 1.1 UX flow (top to bottom)

```
┌─ Settings → AI ─────────────────────────────────────────────┐
│                                                             │
│  ☁️  Cloud providers                                       │
│  (existing list, unchanged)                                │
│                                                             │
│  📦  Local offline model                                   │
│  ──────────────────────                                     │
│  Status: ● Not installed                                   │
│                                                             │
│  Model:    [Qwen2.5 1.5B Instruct (Q4_K_M) ▾]            │
│  Size:     ~1.0 GB · Min RAM: 4 GB                        │
│  Server:   llama.cpp b3900 · ~30 MB                       │
│  Disk free: 42.3 GB ✓                                      │
│                                                             │
│  [ Install offline AI ]   ← single button                 │
│                                                             │
│  When server is up:                                        │
│  Status: ● Running on http://127.0.0.1:39135              │
│  [ Stop server ]  [ Remove model ]                        │
└─────────────────────────────────────────────────────────────┘
```

The "Install" button starts a **two-stage download**: GGUF model
(~1–2 GB) + `llama-server.exe` zip (~30 MB). Progress emits via Tauri
events (`llm:fallback:progress`) so the UI shows a single progress bar
weighted by total bytes.

When the server comes up, the backend auto-creates an `llm_providers`
row of `kind = "fallback"` (or reuses one) so all existing AI tools
(Chat, Translate, Explain, Summarize) immediately route through it. The
"Local-first" toggle in Settings → AI then becomes functional (today
it's a checkbox with no wiring — see §1.4).

### 1.2 Backend — missing Tauri commands

The `FallbackManager` already has `set_selected_model`, `set_enabled`,
`ensure_server`, `kill_server`, `update_phase`, etc. What's missing:

| Command | Purpose |
|---|---|
| `llm_fallback_list_models` | Return UI-friendly model metadata (id, name, size, min_ram, primary_url, mirrors) |
| `llm_fallback_download_start(model_id)` | Spawn a tokio task that downloads model + llama-server, emits `llm:fallback:progress` events |
| `llm_fallback_download_cancel()` | Stop the in-flight download, mark `Phase::Error` with "cancelled" |
| `llm_fallback_progress_stream` (event) | Emitted every 250ms with `{phase, bytes_done, total_bytes, speed_bps, eta_seconds, current_stage: "model" \| "server"}` |
| `llm_fallback_start_server()` | Wrapper around `ensure_server()` with port probe |
| `llm_fallback_stop_server()` | Wrapper around `kill_server()` |
| `llm_fallback_remove_model()` | Delete the GGUF file, reset phase to `NotDownloaded` |
| `llm_fallback_ensure_provider_row()` | After server is up, INSERT/UPSERT an `llm_providers` row so the rest of the system sees it |

Concurrency:
- The manager's `parking_lot::Mutex` already serializes state mutations.
- Add a separate `AtomicU8` cancel flag (or a `tokio::sync::Notify`) for
  download cancellation — `reqwest` doesn't natively support cancellation,
  so we drop the future and let the connection drain in the background.

### 1.3 Frontend

New component `LocalModelPanel.vue` (used inside `Settings.vue` AI
section, OR a dedicated `/system/llm/local` page — **decide in §1.5**).

State:
- `useLlmStore.fallback` — fetches `list_models`, `state`, subscribes to
  `llm:fallback:progress` event via `listen()`.
- Actions: `installModel()`, `cancelDownload()`, `startServer()`,
  `stopServer()`, `removeModel()`.

UI states:
1. **Not installed**: model picker + Install button.
2. **Downloading**: progress bar + cancel button.
3. **Verifying**: spinner.
4. **Ready (server stopped)**: Start server + Remove model buttons.
5. **Running**: green status + Stop server + port URL.

Error states: `HashMismatch`, network errors → friendly toast + retry.

### 1.4 Wiring "Local-first" toggle

Today `ai.local_first` is stored in `app_state` but no code reads it.
Fix:

```rust
// In commands/llm.rs::llm_chat:
fn decide_route(req: &ChatRequest, fallback_mgr: &FallbackManager, settings: &Db) -> ProviderDecision {
    let local_first = settings::get_or(db, "ai.local_first", "false")? == "true";
    let fallback_ready = matches!(fallback_mgr.state().phase, Phase::Ready { .. }) && fallback_mgr.local_base_url().is_some();
    if local_first && fallback_ready {
        return ProviderDecision::UseLocal;
    }
    // ... existing cloud routing
}
```

When `UseLocal`, the backend rewrites `provider_id` to the auto-created
fallback provider row before calling `resolve()`. UI sees no difference.

### 1.5 Open questions (need your call)

- **Q1.1**: Settings inline panel vs. dedicated `/system/llm/local` page?
  - Inline: discoverable, but Settings page gets long.
  - Page: cleaner Settings, but adds a sidebar entry.
  - **My recommendation**: inline. The "AI" section is already where
    admins expect this; another menu item is noise.
- **Q1.2**: Auto-create the `llm_providers` row, or make the user click
  "Use as default" to register it?
  - **My recommendation**: auto-create (off by default, `enabled=false`).
    When server starts, set `enabled=true`. When stopped, `enabled=false`.
    Keeps the providers list truthful.
- **Q1.3**: License/disclaimer dialog before download?
  - **My recommendation**: yes, one-time, with the model card showing
    license (Apache-2.0 for Qwen, Llama-3 community license for Llama).
    Stored as `app_state: llm.fallback.disclaimer_accepted_v1=true`.

---

## 2. Feature B — Conversation Multi-Level Management (问答多级管理)

### 2.1 What "multi-level" means here

Three things, in order of user value:

1. **Session list** — left rail showing all past conversations,
   grouped by date / pinned / archived. This alone is a 10× UX win over
   the current single-buffer Chat page.
2. **Tree of branches** — within a session, you can **fork from any
   message** to explore alternative answers without losing the original.
   This is the actual "multi-level" part — the message log becomes a tree,
   not a list.
3. **Templates / agents** — pre-canned system prompts + tool presets
   ("SQL expert", "Code reviewer", "Translator v2") that spawn new
   sessions with one click.

I considered "threaded replies" (Slack-style) but rejected it — LLM
responses are atomic, not threaded. The branching model (2.2) is the
right abstraction.

### 2.2 Data model — V12 migration

```sql
-- Conversation = a top-level chat session. Has a tree of messages.
CREATE TABLE llm_conversations (
    id              TEXT PRIMARY KEY,                 -- uuid v4
    title           TEXT NOT NULL DEFAULT 'New chat',
    owner_user_id   INTEGER NOT NULL,                 -- users.id
    provider_id     TEXT,                             -- last-used provider
    model_id        TEXT,                             -- last-used model
    system_prompt   TEXT NOT NULL DEFAULT '',
    template_id     TEXT,                             -- optional, see 2.5
    visibility      TEXT NOT NULL DEFAULT 'private',  -- 'private'|'team'|'public'
    pinned          INTEGER NOT NULL DEFAULT 0,
    archived        INTEGER NOT NULL DEFAULT 0,
    total_tokens    INTEGER NOT NULL DEFAULT 0,
    total_cost_usd  REAL NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    FOREIGN KEY (owner_user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (provider_id)   REFERENCES llm_providers(id) ON DELETE SET NULL
);

-- A single message in a conversation. Forms a tree via parent_message_id.
CREATE TABLE llm_messages (
    id                    TEXT PRIMARY KEY,
    conversation_id       TEXT NOT NULL,
    parent_message_id     TEXT,                       -- NULL = root of conversation
    role                  TEXT NOT NULL,              -- 'system'|'user'|'assistant'|'tool'
    content               TEXT NOT NULL,
    -- The path from root to this node, slash-separated message ids.
    -- Lets us reconstruct ancestor chain without recursive SQL.
    path                  TEXT NOT NULL,              -- e.g. 'root_id/abc/def'
    depth                 INTEGER NOT NULL,
    branch_index          INTEGER NOT NULL DEFAULT 0, -- sibling order under same parent
    provider_id           TEXT,
    model_id              TEXT,
    prompt_tokens         INTEGER NOT NULL DEFAULT 0,
    completion_tokens     INTEGER NOT NULL DEFAULT 0,
    cost_usd              REAL NOT NULL DEFAULT 0,
    latency_ms            INTEGER NOT NULL DEFAULT 0,
    request_id            TEXT,                       -- ties to llm_usage row
    error                 TEXT,                       -- non-null = failed turn
    created_at            TEXT NOT NULL,
    FOREIGN KEY (conversation_id) REFERENCES llm_conversations(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_message_id) REFERENCES llm_messages(id) ON DELETE CASCADE
);

CREATE INDEX idx_llm_messages_conv_created ON llm_messages(conversation_id, created_at);
CREATE INDEX idx_llm_messages_parent ON llm_messages(parent_message_id);

-- Saved prompt templates / agents.
CREATE TABLE llm_templates (
    id              TEXT PRIMARY KEY,
    code            TEXT UNIQUE NOT NULL,             -- 'sql-expert', 'code-reviewer', ...
    name            TEXT NOT NULL,
    description     TEXT NOT NULL DEFAULT '',
    system_prompt   TEXT NOT NULL,
    -- JSON: { provider_id?, model_id?, temperature?, max_tokens?, tools?: [...] }
    default_config  TEXT NOT NULL DEFAULT '{}',
    built_in        INTEGER NOT NULL DEFAULT 0,       -- protected from delete when 1
    sort_order      INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL
);

-- Optional: shared visibility grants for team/public conversations.
CREATE TABLE llm_conversation_shares (
    conversation_id TEXT NOT NULL,
    user_id         INTEGER,                          -- NULL row = role-wide grant
    role_id         INTEGER,
    can_edit        INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL,
    PRIMARY KEY (conversation_id, user_id, role_id),
    FOREIGN KEY (conversation_id) REFERENCES llm_conversations(id) ON DELETE CASCADE
);

-- Conversation tags (many-to-many).
CREATE TABLE llm_conversation_tags (
    conversation_id TEXT NOT NULL,
    tag             TEXT NOT NULL,
    PRIMARY KEY (conversation_id, tag),
    FOREIGN KEY (conversation_id) REFERENCES llm_conversations(id) ON DELETE CASCADE
);
```

**Why a tree, not a flat list + parent_id?**

- Forks are the dominant operation (the user said "问答多级" → "ask
  another question about the answer"). A flat list loses the fork
  relationship; a tree preserves it natively.
- The `path` column gives O(1) ancestry lookup (`path LIKE 'root/%'`)
  without recursive CTEs. Important because SQLite doesn't support
  `WITH RECURSIVE` in the same way and we want chat history to load fast.
- Sibling order via `branch_index` keeps UI stable when forking.

### 2.3 UI layout

```
┌──────────────────────────────────────────────────────────────────────┐
│ [≡] AI Chat                              [💬 New]  [🔍 Search]  [⚙] │
├─────────────┬────────────────────────────────────────────────────────┤
│ Sessions    │  Conversation: "Why is Vite slow in CI?"     [⋯]       │
│ ────────    │  ─────────────────────────────────────────────────     │
│ 📌 Pinned   │  Path:  root → "Why slow" → "It's the dep prebundle"   │
│ • SQL opt   │                                                        │
│             │  ┌─ Branch A ─────────────────────────────────────┐   │
│ Today       │  │  👤 you: Why is Vite slow in CI?              │   │
│ • Vite slow │  │  🤖 assistant: It's likely the dep prebundle…  │   │
│ • Postgres  │  │       [↳ Fork]  [📋 Copy]  [↻ Regenerate]    │   │
│ • JS regex  │  └───────────────────────────────────────────────┘   │
│             │                                                        │
│ Yesterday   │  ┌─ Branch B (forked from "Why slow") ───────────┐   │
│ • Translate │  │  👤 you: What if I disable prebundle?         │   │
│ • Tauri bug │  │  🤖 assistant: That would help, but…         │   │
│             │  │       [↳ Fork]  [📋 Copy]                    │   │
│ Older       │  └───────────────────────────────────────────────┘   │
│ ...         │                                                        │
│             │  [Ask a follow-up → creates a new message under the  │
│ Templates   │   currently-focused node]                            │
│ • SQL exp.  │                                                        │
│ • Code rev. │  [ Type message...                            ] [↑] │
│ • Trans. v2 │                                                        │
└─────────────┴────────────────────────────────────────────────────────┘
```

Key UI affordances:

- **Focus indicator** on the active branch (highlighted node in the path
  breadcrumb at the top).
- **Fork button** on any message: spawns a new sibling branch with that
  message's content as the parent, lets the user ask a divergent question.
- **Regenerate** on assistant messages: creates a new sibling branch with
  the same user prompt, side-by-side comparison.
- **Branch switcher** at the top of each turn: ◀ A · B · C ▶ (when a
  node has multiple children).
- **Search** (header icon): full-text over message content + titles.
- **Templates** (sidebar bottom): click → new session with pre-filled
  system prompt + config.
- **Right-click context menu**: rename, archive, delete, export.

### 2.4 Streaming UX

We already have `llm_chat_stream` emitting `llm:chunk` events but no UI
uses it. v0.6.2 wires it up:

- During generation, the assistant message starts as a placeholder
  (`...`) and gets `content` appended in real time as chunks arrive.
- The user can **stop generation** (interrupt button) → cancel the
  in-flight request, save what we got so far (or discard, user choice).
- Errors mid-stream show inline in red, with a "Retry" button that
  generates a sibling branch.

### 2.5 Templates / agents

The `llm_templates` table seeds a few built-ins on V12:

| code | name | system prompt |
|---|---|---|
| `general` | General Assistant | (empty — neutral) |
| `sql-expert` | SQL Expert | "You are a senior DBA. When given a schema, write efficient, parameterized SQL. Explain the plan." |
| `code-reviewer` | Code Reviewer | "You are a meticulous reviewer. List concrete issues + fixes; never refactor without asking." |
| `translator-v2` | Translator v2 | Same as Translate page's system prompt |
| `summarizer-v2` | Summarizer v2 | Same as Summarize page's system prompt |
| `regex-builder` | Regex Builder | "Build a regex from a natural-language description. Test it against examples I provide. Explain the parts." |

The existing Translate/Explain/Summarize pages stay as quick tools, but
they can be re-skinned as "spawn from template" shortcuts in v0.6.3.

### 2.6 RBAC

Reuse existing permissions:

- `llm:use` — create sessions, send messages.
- `llm:manage` — manage templates, view others' sessions in `team`
  visibility mode.

New permissions:

- `llm:share:public` — mark a conversation `public` (visible to all
  authenticated users, read-only).
- `llm:templates:manage` — create/edit non-built-in templates.

Visibility rules:

- `private` — only owner sees it.
- `team` — anyone with `llm:use` can `SELECT` (read-only).
- `public` — anyone authenticated, read-only.

Sharing grants (`llm_conversation_shares`) override visibility for
specific users/roles (can grant edit rights).

### 2.7 Export

For v0.6.3 (not v0.6.2):

- **Markdown**: thread flattened along the focused branch, with sidebar
  notes for sibling branches. Standard chat-archive format.
- **JSON**: full tree dump (for re-import later).
- **PDF**: via the existing PDF skill (write a markdown intermediate,
  then `pdf` skill renders it).

### 2.8 Out of scope for v0.6.2

To keep the release shippable, defer:

- [ ] Streaming tool calls (function-calling — not in providers yet).
- [ ] Multi-user real-time collaboration (websocket + CRDT — too much).
- [ ] Conversation import (only export for now).
- [ ] Mobile / Tauri-mobile adaptation.
- [ ] Branch merge (combining two branches into one).
- [ ] Auto-summary of long conversations (LLM calling itself).

### 2.9 v0.6.2 vs v0.6.3 split

| Feature | v0.6.2 | v0.6.3 |
|---|---|---|
| Session list (left rail) | ✅ |  |
| Tree branching (fork, regenerate) | ✅ |  |
| Streaming UX | ✅ |  |
| Built-in templates (6 seeded) | ✅ |  |
| RBAC + visibility | ✅ |  |
| Search (title + content) | ✅ |  |
| Tags |  | ✅ |
| Markdown export |  | ✅ |
| JSON export / re-import |  | ✅ |
| PDF export |  | ✅ |
| Conversation sharing UI |  | ✅ |
| Custom templates CRUD (frontend) |  | ✅ |
| Auto-title from first message |  | ✅ |

### 2.10 Migration risks

V12 introduces a hard dependency on `llm_messages.path` — every insert
must compute the correct path. Mitigation:

- Single helper `messages::insert_with_path(conn, msg)` that resolves
  parent path + appends new id.
- 5 unit tests covering: root insert, nested insert, sibling ordering,
  fork from mid-tree, deep path (>5 levels).

---

## 3. Decisions needed before code

| # | Question | My recommendation |
|---|---|---|
| Q1.1 | Local model panel: inline in Settings vs. dedicated page? | **Inline** |
| Q1.2 | Auto-create `llm_providers` row when server starts? | **Yes, with enabled toggle synced** |
| Q1.3 | License disclaimer before download? | **Yes, one-time, with license info per model** |
| Q2.1 | Tree depth limit? | **No hard limit; UI collapses beyond 4** |
| Q2.2 | Branch visibility: hidden until focused, or always shown? | **Show all; collapse inactive to a single line** |
| Q2.3 | "Send to Translate/Explain" cross-tool handoff? | **Defer to v0.6.3** — keep scope tight |
| Q2.4 | Conversation-level system prompt override vs always inherit? | **Always override (per-conversation), ignore parents** |

If you're good with all my recommendations above, I'll start with Q1.1
+ Q1.2 + the §1.2 commands + the panel UI, then move to V12 migration
and the tree UI. Roughly 1500–2000 LoC across Rust + Vue, 6–8 days of
work if uninterrupted.

---

## 4. File-level plan (when we go)

### New backend files
- `src-tauri/src/llm/fallback/download.rs` — extend with `cancel`
  (already has stream + sha256).
- `src-tauri/src/commands/llm_fallback.rs` — new command module for
  the §1.2 surface.
- `src-tauri/src/commands/llm_conversations.rs` — CRUD for sessions
  + messages + templates + tags.
- `src-tauri/migrations/V12__seed_conversations.sql`.

### Modified backend files
- `src-tauri/src/commands/llm.rs` — `pick_adapter` already has
  Google; add routing for `decide_route()` per §1.4.
- `src-tauri/src/lib.rs` — register new commands + new AppState
  fields if needed.

### New frontend files
- `src/views/ai/LocalModelPanel.vue`.
- `src/views/ai/Conversations.vue` (new sidebar layout replacing
  current Chat.vue's flat page) — actually keep Chat.vue as the
  "single-session quick view" but redirect /ai/chat to /ai/conversations.
- `src/views/ai/ConversationView.vue` (the right pane with tree).
- `src/api/conversations.ts` (typed wrappers).
- `src/stores/conversations.ts` (pinia store).

### Modified frontend files
- `src/router/index.ts` — `/ai/conversations` + `/ai/conversations/:id`.
- `src/stores/llm.ts` — fallback state subscription.
- `src/views/admin/Settings.vue` — embed `LocalModelPanel.vue`.
- `src/i18n/locales/{en-US,zh-CN}.ts` — ~30 new keys per locale.

---

## 5. What I want from you

Either:

1. **Approve the design as-is** ("ship it, go with all your
   recommendations") → I start §1.2 commands + LocalModelPanel, then
   V12 + tree UI.
2. **Pick a different option on one or more questions** (Q1.1–Q2.4) →
   I'll patch the doc + scope and start.
3. **"Just do §1 (local install) this release, defer conversations to
   v0.6.3"** → Tighter scope, faster ship. The conversation design
   above still serves as the spec for next release.

Which way?