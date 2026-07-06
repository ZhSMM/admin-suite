# Architecture

This document describes how the Admin Suite project is laid out —
what runs where, who calls whom, and how to make sense of the codebase
when you come in cold.

## 1. Two-process layout

```
┌───────────────────────────────────────────────────────────────┐
│                       Electron-like shell                     │
│                                                               │
│  ┌─────────────────────────────┐    ┌──────────────────────┐ │
│  │   Vue 3 + Pinia + Element   │    │   Tauri runtime      │ │
│  │   (TypeScript)              │ ◀▶ │   + WebView          │ │
│  │   src/                      │ IPC│   src-tauri/src/      │ │
│  └─────────────────────────────┘    └──────────────────────┘ │
│                                          ▲                   │
│                                          │                   │
│                                  Local SQLite + plugins       │
└───────────────────────────────────────────────────────────────┘
```

- The frontend is a single-page Vue 3 app. It owns no business
  logic — every privileged action goes through a Tauri IPC call.
- The backend is a Rust binary. It owns the database, file system,
  credential vault, and external HTTP calls (LLM providers,
  HuggingFace, model mirrors, GitHub release API).
- IPC arguments and return values cross the boundary as JSON; every
  call goes through `AppError` → serialised to `{ code, message }`
  on the wire.

## 2. Database

- SQLite, single file under the OS-config dir.
- Migrations live in `src-tauri/migrations/V*.sql` (Flyway-style),
  applied on every boot by `db::migrate`. The current test asserts
  `>= 13` migrations so an early V13 typo can't silently pass CI.
- Schema lives in two epochs:
  - V1 — bootstrap (users, roles, menus, audit, resources)
  - V7–V13 — features layered on (app_state, monitoring, LLM,
    chat history, admin perms)

## 3. Frontend layout

```
src/
├── main.ts                    # Vue + Pinia + vue-i18n + router
├── App.vue
├── router.ts                  # /system/* + /ai/* + /tools/*
├── stores/                    # Pinia stores (one per domain)
│   ├── auth.ts                # login / token / perm checks
│   ├── llm.ts                 # providers / models / fallback LLM state
│   ├── chat-history.ts        # multi-level chat sessions
│   ├── recent.ts              # recent-items dropdown
│   └── ... (theme, locale, menu, …)
├── api/                       # thin wrappers around `invoke`
│   ├── llm.ts                 # TypeScript interfaces for every LLM RPC
│   ├── chat-history.ts        # 8 commands (list/create/.../export)
│   └── ... (one file per domain)
├── views/                     # routed pages
│   ├── admin/                 # everything under /system/*
│   ├── ai/                    # chat, explain, summarize, translate
│   └── tools/                  # json, base, sql, hash, …
├── views/ai/LocalModelPanel.vue   # offline GGUF installer (separate flow)
├── components/                # shared widgets
├── i18n/                      # vue-i18n locales
└── styles/                    # element-plus overrides + tokens
```

### Frontend conventions

- **One store per domain** — avoid glue stores; cross-domain data
  should cross via emitted events.
- **API surface mirrors backend commands** — `api/llm.ts` exposes
  one TS function per `#[tauri::command]`.
- **No raw `invoke` in components** — always wrap in `api/*.ts` so
  TypeScript types stay tight.
- **i18n in components** uses `t('namespace.key')`; new keys go in
  both `zh-CN.ts` and `en-US.ts` in the same commit.

## 4. Backend layout

```
src-tauri/src/
├── main.rs                    # binary entry, calls run() in lib.rs
├── lib.rs                     # builder, AppState, command registration
├── auth/
│   ├── session.rs             # token store, AuthenticatedUser
│   ├── rbac.rs                # permission checks (e.g. `llm:manage`)
│   └── …
├── commands/                  # Tauri command handlers — one
│   ├── mod.rs                 #   domain per module
│   ├── auth.rs                #   (login/logout/me)
│   ├── users.rs               #
│   ├── audit.rs               #
│   ├── llm.rs                 #   provider/model CRUD + chat
│   ├── llm_fallback/          # offline GGUF installer
│   │   ├── mod.rs             #   install / cancel / server / remove
│   │   ├── speed_test.rs      #   streaming mirror probe
│   │   ├── provider.rs        #   upserts llm_providers row
│   │   └── reroute.rs         #   ai.local_first → fallback
│   ├── chat_history.rs        #   9 commands (sessions + messages)
│   └── …
├── db/
│   ├── mod.rs                 # Db wrapper (with_conn / with_tx)
│   └── migrate.rs             # Flyway-like runner + tests
├── llm/
│   ├── mod.rs                 # ChatRequest/Response, ProviderContext,
│   │                         #   LlmProvider trait (incl. list_models)
│   └── providers/
│       ├── openai_compat.rs   # GET /models + chat
│       ├── anthropic.rs       # GET /v1/models + chat
│       ├── google.rs          # GET /v1beta/models + chat
│       ├── custom.rs          # echo / passthrough (legacy)
│       └── fallback.rs        # proxy to local llama-server
├── llm/fallback/              # offline installer engine
│   ├── mod.rs                 # public surface — re-exports
│   ├── manager.rs             # lifecycle (NotDownloaded → Ready)
│   ├── registry.rs            # spec table (MODELS), mirror resolver
│   ├── download.rs            # single-conn stream + SHA-256
│   └── multi_download.rs      # Range-aware parallel downloader
└── error.rs                   # AppError + AppResult + serialise
```

### Backend conventions

- **One domain per command module** — keep `commands/foo.rs`
  under ~600 lines; split into `commands/foo/{mod.rs,part.rs}` if
  it grows.
- **All commands require an `llm:use` or `llm:manage` token**
  via `commands::llm::require_perm`; never trust `token == ""`.
- **All DB access goes through `state.db.with_conn` /
  `state.db.with_tx`** — those wrap `Arc<Connection>` and
  serialise writes into transactions cleanly.
- **No `unwrap()` in app code** — tests may use it; hot paths
  bubble `AppError`.

## 5. IPC error contract

```
Rust                          IPC                           TypeScript
─────────                      ────                          ──────────
AppError::Validation(_)   -->  { code: "VALIDATION", msg } --> try { call<…>() } catch { … }
AppError::Db(_)           -->  { code: "DB_ERROR",    msg } -->  ↑
…                          <--  ────                       <-- thrown as JS Error
```

The frontend converts the structured response to `ElMessage.error`
or surfaces it on the relevant form field. The `code` field is
stable across versions; the `message` is human-readable and may
change.

## 6. Lifecycle of a Tauri command

1. Frontend calls `await llmApi.chat(token, args)`.
2. Tauri resolves the registered handler — one of the
   `#[tauri::command]` functions in `commands/*.rs`.
3. Handler calls `require_perm(&state, &token, "llm:use")` →
   `state.sessions.lookup(token)` then
   `auth::rbac::require_permission(&user, "llm:use")?`.
4. Handler does business work, possibly emitting events via
   `app.emit_all("namespace:event", payload)`.
5. Handler returns `Result<T, AppError>`; Tauri serialises the
   `Ok(T)` payload to JSON or the `Err(AppError)` to its
   `{code, message}` shape.

## 7. Where to start reading

| If you want to…                                       | Read…                              |
|------------------------------------------------------|------------------------------------|
| Understand the offline model installer              | `src-tauri/src/llm/fallback.rs`    |
| Add a new LLM provider                               | `src-tauri/src/llm/providers/`     |
| Tweak the chat UI                                    | `src/views/ai/Chat.vue`            |
| Add a new admin page                                 | `src/views/admin/Settings.vue`     |
| Cut a release                                        | `docs/release-process.md`          |
