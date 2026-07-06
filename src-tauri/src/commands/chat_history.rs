//! v0.7.0 ??Persistent multi-level chat history.
//!
//! Two tables (see `migrations/V12__add_chat_history.sql`):
//! - `chat_sessions` ??one per chat "session" (a tree root)
//! - `chat_messages` ??tree nodes. `parent_id` NULL = root. Same parent
//!   with multiple rows = sibling branches.
//!
//! Branching is implicit: pick `activePath` (root?leaf ids) on the
//! client and rebuild `ChatMessage[]` for the LLM call from it.

use rusqlite::{params, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

use crate::auth::rbac::require_permission;
use crate::auth::session::AuthenticatedUser;
use crate::error::{AppError, AppResult};
use crate::AppState;

// ---------------------------------------------------------------------------
// Wire types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatSession {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub provider_id: String,
    pub model_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub archived: bool,
    /// Number of messages on the *active path* (root?leaf of latest).
    /// Filled by `chat_session_list` for sidebar sorting/display.
    pub message_count: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessageNode {
    pub id: i64,
    pub session_id: i64,
    pub parent_id: Option<i64>,
    pub role: String, // 'user' | 'assistant' | 'system'
    pub content: String,
    pub provider_id: String,
    pub model_id: String,
    pub status: String, // 'done' | 'streaming' | 'error'
    pub error: Option<String>,
    pub created_at: String,
    pub children: Vec<ChatMessageNode>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatSessionWithTree {
    pub session: ChatSession,
    pub tree: Vec<ChatMessageNode>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ListArgs {
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub search: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct CreateSessionArgs {
    pub title: Option<String>,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateSessionArgs {
    pub id: i64,
    pub title: Option<String>,
    #[serde(default)]
    pub archived: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
pub struct GetSessionArgs {
    pub id: i64,
}

#[derive(Debug, Deserialize, Default)]
pub struct DeleteArgs {
    pub id: i64,
}

#[derive(Debug, Deserialize, Default)]
pub struct AppendMessageArgs {
    pub session_id: i64,
    pub parent_id: Option<i64>,
    pub role: String,
    pub content: String,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub status: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateMessageArgs {
    pub id: i64,
    pub content: Option<String>,
    pub status: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExportArgs {
    pub id: i64,
    /// 'json' | 'markdown' | 'html'
    pub format: String,
    /// Optional root?leaf path; if absent we auto-pick "latest sibling"
    /// at every level (the natural newest-branch path).
    pub active_path_ids: Option<Vec<i64>>,
}

#[derive(Debug, Serialize)]
pub struct ExportResult {
    pub filename: String,
    pub mime: String,
    /// utf-8 content; frontend writes it to disk.
    pub content: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn now_ms_str() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%fZ").to_string()
}

fn require_user(state: &State<AppState>, token: &str) -> AppResult<AuthenticatedUser> {
    let user = state.sessions.lookup(token)?;
    require_permission(&user, "llm:use")?;
    Ok(user)
}

fn row_to_session(row: &Row<'_>) -> rusqlite::Result<ChatSession> {
    let archived: i64 = row.get(7)?;
    Ok(ChatSession {
        id: row.get(0)?,
        user_id: row.get(1)?,
        title: row.get(2)?,
        provider_id: row.get(3)?,
        model_id: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
        archived: archived != 0,
        message_count: row.get::<_, i64>(8).unwrap_or(0),
    })
}

fn row_to_message(row: &Row<'_>) -> rusqlite::Result<ChatMessageNode> {
    Ok(ChatMessageNode {
        id: row.get(0)?,
        session_id: row.get(1)?,
        parent_id: row.get(2)?,
        role: row.get(3)?,
        content: row.get(4)?,
        provider_id: row.get(5)?,
        model_id: row.get(6)?,
        status: row.get(7)?,
        error: row.get(8)?,
        created_at: row.get(9)?,
        children: Vec::new(),
    })
}

const SESSION_BASE_COLS: &str =
    "id, user_id, title, provider_id, model_id, created_at, updated_at, archived";

const MESSAGE_BASE_COLS: &str = "id, session_id, parent_id, role, content, \
    provider_id, model_id, status, error, created_at";

/// Build a `Vec<ChatMessageNode>` (forest of roots) from a flat list of
/// message rows. Two-pass: first index, then attach.
fn build_tree(rows: Vec<ChatMessageNode>) -> Vec<ChatMessageNode> {
    let mut children_by_parent: HashMap<i64, Vec<usize>> = HashMap::new();
    for (i, n) in rows.iter().enumerate() {
        if let Some(p) = n.parent_id {
            children_by_parent.entry(p).or_default().push(i);
        }
    }
    fn attach(
        all: &[ChatMessageNode],
        children_by_parent: &HashMap<i64, Vec<usize>>,
        parent: Option<i64>,
        out: &mut Vec<ChatMessageNode>,
    ) {
        if let Some(pid) = parent {
            if let Some(idxs) = children_by_parent.get(&pid) {
                for &i in idxs {
                    let mut node = ChatMessageNode {
                        children: Vec::new(),
                        ..all[i].clone()
                    };
                    attach(all, children_by_parent, Some(node.id), &mut node.children);
                    out.push(node);
                }
            }
        }
    }
    let mut roots = Vec::new();
    attach(&rows, &children_by_parent, None, &mut roots);
    roots
}

fn sanitize_filename(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else if c.is_whitespace() {
                '_'
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

/// Render an active-path `Vec<ChatMessageNode>` as Markdown.
fn render_markdown(title: &str, path: &[ChatMessageNode]) -> String {
    let mut s = format!(
        "# {}\n\n",
        if title.is_empty() { "(untitled)" } else { title }
    );
    for m in path {
        let role = match m.role.as_str() {
            "user" => "## ?? You",
            "assistant" => "## ?? Assistant",
            _ => "## ??System",
        };
        s.push_str(role);
        s.push_str("\n\n");
        s.push_str(m.content.trim());
        s.push_str("\n\n");
        if !m.model_id.is_empty() {
            s.push_str(&format!(
                "\n<sub>{} - {}</sub>\n\n",
                m.model_id, m.created_at
            ));
        }
    }
    s
}

/// Render an active-path as a self-contained HTML (imports none, safe to
/// paste anywhere ??we keep CSS inline and tiny).
fn render_html(title: &str, path: &[ChatMessageNode]) -> String {
    let escape = |s: &str| -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('\n', "<br/>")
    };
    let mut body = String::new();
    for m in path {
        let cls = match m.role.as_str() {
            "user" => "u",
            "assistant" => "a",
            _ => "s",
        };
        body.push_str(&format!(
            "<div class=\"m m-{}\"><div class=\"r\">{}</div><div class=\"c\">{}</div></div>",
            cls,
            escape(&format!(
                "{}{}{}",
                m.role,
                if m.model_id.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", m.model_id)
                },
                ""
            )),
            escape(&m.content)
        ));
    }
    format!(
        "<!doctype html><meta charset=\"utf-8\"><title>{}</title><style>\
         body{{font:14px/1.5 -apple-system,Segoe UI,sans-serif;max-width:780px;\
         margin:24px auto;padding:0 16px;color:#1f2937}}\
         h1{{font-size:18px;border-bottom:1px solid #e5e7eb;padding-bottom:8px}}\
         .m{{margin:12px 0;padding:10px 14px;border-radius:8px}}\
         .m-u{{background:#eef2ff}}\
         .m-a{{background:#f3f4f6}}\
         .m-s{{background:#fef3c7;font-style:italic}}\
         .r{{font-size:11px;color:#6b7280;margin-bottom:4px;text-transform:uppercase;\
         letter-spacing:.04em}}\
         pre,code{{white-space:pre-wrap;word-break:break-word}}\
         </style><h1>{}</h1>{}",
        escape(if title.is_empty() { "Chat" } else { title }),
        escape(if title.is_empty() { "Chat" } else { title }),
        body
    )
}

/// Default active path: "newest sibling at every level" from the roots.
/// Falls back to empty if there are no messages.
fn default_active_path(tree: &[ChatMessageNode], out: &mut Vec<i64>) {
    if let Some(root) = tree.iter().max_by_key(|n| n.created_at.clone()) {
        out.push(root.id);
        let mut current = root;
        while !current.children.is_empty() {
            current = current
                .children
                .iter()
                .max_by_key(|n| n.created_at.clone())
                .expect("non-empty");
            out.push(current.id);
        }
    }
}

/// Walk the tree to collect actual node clones for the active path.
fn active_path_nodes(tree: &[ChatMessageNode], ids: &[i64]) -> Vec<ChatMessageNode> {
    fn collect_at(tree: &[ChatMessageNode], target: i64, out: &mut Vec<ChatMessageNode>) {
        for n in tree {
            if n.id == target {
                out.push(ChatMessageNode {
                    children: Vec::new(),
                    ..n.clone()
                });
                return;
            }
            collect_at(&n.children, target, out);
        }
    }
    let mut out = Vec::new();
    for id in ids {
        collect_at(tree, *id, &mut out);
    }
    out
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn chat_session_list(
    state: State<'_, AppState>,
    token: String,
    args: Option<ListArgs>,
) -> AppResult<Vec<ChatSession>> {
    let _t = crate::commands::metrics::time(&state.metrics, "chat_session_list");
    let user = require_user(&state, &token)?;
    let args = args.unwrap_or_default();
    state.db.with_conn(|c| {
        let sql = format!(
            "SELECT {cols}, (SELECT COUNT(*) FROM chat_messages m WHERE m.session_id = s.id) AS cnt
             FROM chat_sessions s
             WHERE s.user_id = ?1 AND s.archived = ?2
               AND (?3 = '' OR s.title LIKE '%' || ?3 || '%')
             ORDER BY s.updated_at DESC
             LIMIT 200",
            cols = SESSION_BASE_COLS
        );
        let items: Vec<ChatSession> = c
            .prepare(&sql)?
            .query_map(
                params![user.user_id, args.archived as i64, args.search],
                row_to_session,
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(items)
    })
}

#[tauri::command]
pub async fn chat_session_create(
    state: State<'_, AppState>,
    token: String,
    args: Option<CreateSessionArgs>,
) -> AppResult<ChatSession> {
    let _t = crate::commands::metrics::time(&state.metrics, "chat_session_create");
    let user = require_user(&state, &token)?;
    let args = args.unwrap_or_default();
    let now = now_ms_str();
    state.db.with_conn(|c| {
        c.execute(
            "INSERT INTO chat_sessions (user_id, title, provider_id, model_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![
                user.user_id,
                args.title.unwrap_or_default(),
                args.provider_id.unwrap_or_default(),
                args.model_id.unwrap_or_default(),
                now,
            ],
        )?;
        let id = c.last_insert_rowid();
        let row = c
            .query_row(
                &format!(
                    "SELECT {cols}, 0 AS cnt FROM chat_sessions WHERE id = ?1",
                    cols = SESSION_BASE_COLS
                ),
                params![id],
                row_to_session,
            )
            .optional()?;
        row.ok_or_else(|| AppError::Internal("session insert vanished".into()))
    })
}

#[tauri::command]
pub async fn chat_session_update(
    state: State<'_, AppState>,
    token: String,
    args: UpdateSessionArgs,
) -> AppResult<ChatSession> {
    let _t = crate::commands::metrics::time(&state.metrics, "chat_session_update");
    let user = require_user(&state, &token)?;
    let now = now_ms_str();
    state.db.with_conn(|c| {
        // Scope to this user so cross-tenant writes can't happen.
        let owned: i64 = c
            .query_row(
                "SELECT user_id FROM chat_sessions WHERE id = ?1",
                params![args.id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| AppError::Validation("session not found".into()))?;
        if owned.to_string() != user.user_id {
            return Err(AppError::Forbidden("not your session".into()));
        }
        if let Some(t) = &args.title {
            c.execute(
                "UPDATE chat_sessions SET title = ?1, updated_at = ?2 WHERE id = ?3",
                params![t, now, args.id],
            )?;
        }
        if let Some(a) = args.archived {
            c.execute(
                "UPDATE chat_sessions SET archived = ?1, updated_at = ?2 WHERE id = ?3",
                params![a as i64, now, args.id],
            )?;
        }
        c.query_row(
            &format!(
                "SELECT {cols}, (SELECT COUNT(*) FROM chat_messages WHERE session_id = ?1) AS cnt
                 FROM chat_sessions WHERE id = ?1",
                cols = SESSION_BASE_COLS
            ),
            params![args.id],
            row_to_session,
        )
        .map_err(AppError::from)
    })
}

#[tauri::command]
pub async fn chat_session_delete(
    state: State<'_, AppState>,
    token: String,
    args: DeleteArgs,
) -> AppResult<bool> {
    let _t = crate::commands::metrics::time(&state.metrics, "chat_session_delete");
    let user = require_user(&state, &token)?;
    state.db.with_conn(|c| {
        let n = c.execute(
            "DELETE FROM chat_sessions WHERE id = ?1 AND user_id = ?2",
            params![args.id, user.user_id],
        )?;
        Ok(n > 0)
    })
}

#[tauri::command]
pub async fn chat_session_get(
    state: State<'_, AppState>,
    token: String,
    args: GetSessionArgs,
) -> AppResult<ChatSessionWithTree> {
    let _t = crate::commands::metrics::time(&state.metrics, "chat_session_get");
    let user = require_user(&state, &token)?;
    state.db.with_conn(|c| {
        let session = c
            .query_row(
                &format!(
                    "SELECT {cols}, (SELECT COUNT(*) FROM chat_messages WHERE session_id = ?1) AS cnt
                     FROM chat_sessions WHERE id = ?1 AND user_id = ?2",
                    cols = SESSION_BASE_COLS
                ),
                params![args.id, user.user_id],
                row_to_session,
            )
            .optional()?
            .ok_or_else(|| AppError::Validation("session not found".into()))?;

        let mut stmt = c.prepare(&format!(
            "SELECT {cols} FROM chat_messages WHERE session_id = ?1 ORDER BY id ASC",
            cols = MESSAGE_BASE_COLS
        ))?;
        let rows: Vec<ChatMessageNode> = stmt
            .query_map(params![args.id], row_to_message)?
            .collect::<Result<Vec<_>, _>>()?;
        let tree = build_tree(rows);
        Ok(ChatSessionWithTree { session, tree })
    })
}

#[tauri::command]
pub async fn chat_message_append(
    state: State<'_, AppState>,
    token: String,
    args: AppendMessageArgs,
) -> AppResult<ChatMessageNode> {
    let _t = crate::commands::metrics::time(&state.metrics, "chat_message_append");
    let user = require_user(&state, &token)?;
    let now = now_ms_str();
    state.db.with_tx(|tx| {
        let owner: i64 = tx
            .query_row(
                "SELECT user_id FROM chat_sessions WHERE id = ?1",
                params![args.session_id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| AppError::Validation("session not found".into()))?;
        if owner.to_string() != user.user_id {
            return Err(AppError::Forbidden("not your session".into()));
        }
        tx.execute(
            "INSERT INTO chat_messages
               (session_id, parent_id, role, content, provider_id, model_id, status, error, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                args.session_id,
                args.parent_id,
                args.role,
                args.content,
                args.provider_id.unwrap_or_default(),
                args.model_id.unwrap_or_default(),
                args.status.unwrap_or_else(|| "done".into()),
                args.error,
                now,
            ],
        )?;
        let id = tx.last_insert_rowid();
        tx.execute(
            "UPDATE chat_sessions SET updated_at = ?1 WHERE id = ?2",
            params![now, args.session_id],
        )?;
        let node = tx
            .query_row(
                &format!(
                    "SELECT {cols} FROM chat_messages WHERE id = ?1",
                    cols = MESSAGE_BASE_COLS
                ),
                params![id],
                row_to_message,
            )
            .map_err(AppError::from)?;
        Ok(node)
    })
}

#[tauri::command]
pub async fn chat_message_update(
    state: State<'_, AppState>,
    token: String,
    args: UpdateMessageArgs,
) -> AppResult<ChatMessageNode> {
    let _t = crate::commands::metrics::time(&state.metrics, "chat_message_update");
    let user = require_user(&state, &token)?;
    let now = now_ms_str();
    state.db.with_conn(|c| {
        // owner check via join ??guards cross-tenant writes via direct id.
        let allowed: bool = c
            .query_row(
                "SELECT 1 FROM chat_messages m
                 JOIN chat_sessions s ON s.id = m.session_id
                 WHERE m.id = ?1 AND s.user_id = ?2",
                params![args.id, user.user_id],
                |r| r.get::<_, i64>(0),
            )
            .optional()?
            .is_some();
        if !allowed {
            return Err(AppError::Forbidden("not your message".into()));
        }
        if let Some(content) = &args.content {
            c.execute(
                "UPDATE chat_messages SET content = ?1 WHERE id = ?2",
                params![content, args.id],
            )?;
        }
        if let Some(status) = &args.status {
            c.execute(
                "UPDATE chat_messages SET status = ?1 WHERE id = ?2",
                params![status, args.id],
            )?;
        }
        if let Some(err) = &args.error {
            c.execute(
                "UPDATE chat_messages SET error = ?1 WHERE id = ?2",
                params![err, args.id],
            )?;
        }
        c.execute(
            "UPDATE chat_sessions SET updated_at = ?1
             WHERE id = (SELECT session_id FROM chat_messages WHERE id = ?2)",
            params![now, args.id],
        )?;
        c.query_row(
            &format!(
                "SELECT {cols} FROM chat_messages WHERE id = ?1",
                cols = MESSAGE_BASE_COLS
            ),
            params![args.id],
            row_to_message,
        )
        .map_err(AppError::from)
    })
}

#[tauri::command]
pub async fn chat_message_delete(
    state: State<'_, AppState>,
    token: String,
    args: DeleteArgs,
) -> AppResult<bool> {
    let _t = crate::commands::metrics::time(&state.metrics, "chat_message_delete");
    let user = require_user(&state, &token)?;
    state.db.with_conn(|c| {
        let n = c.execute(
            "DELETE FROM chat_messages WHERE id = ?1
               AND session_id IN (SELECT id FROM chat_sessions WHERE user_id = ?2)",
            params![args.id, user.user_id],
        )?;
        Ok(n > 0)
    })
}

#[tauri::command]
pub async fn chat_session_export(
    state: State<'_, AppState>,
    token: String,
    args: ExportArgs,
) -> AppResult<ExportResult> {
    let _t = crate::commands::metrics::time(&state.metrics, "chat_session_export");
    let user = require_user(&state, &token)?;
    state.db.with_conn(|c| {
        let session = c
            .query_row(
                &format!(
                    "SELECT {cols}, (SELECT COUNT(*) FROM chat_messages WHERE session_id = ?1) AS cnt
                     FROM chat_sessions WHERE id = ?1 AND user_id = ?2",
                    cols = SESSION_BASE_COLS
                ),
                params![args.id, user.user_id],
                row_to_session,
            )
            .optional()?
            .ok_or_else(|| AppError::Validation("session not found".into()))?;

        let mut stmt = c.prepare(&format!(
            "SELECT {cols} FROM chat_messages WHERE session_id = ?1 ORDER BY id ASC",
            cols = MESSAGE_BASE_COLS
        ))?;
        let rows: Vec<ChatMessageNode> = stmt
            .query_map(params![args.id], row_to_message)?
            .collect::<Result<Vec<_>, _>>()?;
        let tree = build_tree(rows);

        let active_ids: Vec<i64> = match args.active_path_ids {
            Some(v) if !v.is_empty() => v,
            _ => {
                let mut p = Vec::new();
                default_active_path(&tree, &mut p);
                p
            }
        };
        let path_nodes = active_path_nodes(&tree, &active_ids);

        let safe_title = sanitize_filename(&session.title);
        let stem = if safe_title.is_empty() {
            format!("chat-{}", session.id)
        } else {
            safe_title
        };
        let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");

        let (content, ext, mime) = match args.format.as_str() {
            "json" => {
                let v = serde_json::json!({
                    "id": session.id,
                    "title": session.title,
                    "provider_id": session.provider_id,
                    "model_id": session.model_id,
                    "created_at": session.created_at,
                    "updated_at": session.updated_at,
                    "active_path_ids": &active_ids,
                    "tree": tree,
                });
                (
                    serde_json::to_string_pretty(&v)?,
                    "json",
                    "application/json",
                )
            }
            "markdown" => (
                render_markdown(&session.title, &path_nodes),
                "md",
                "text/markdown",
            ),
            "html" => (
                render_html(&session.title, &path_nodes),
                "html",
                "text/html",
            ),
            other => {
                return Err(AppError::Validation(format!(
                    "unknown export format: {}",
                    other
                )));
            }
        };

        Ok(ExportResult {
            filename: format!("{}-{}.{}", stem, ts, ext),
            mime: mime.to_string(),
            content,
        })
    })
}




