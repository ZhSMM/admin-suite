//! LLM commands — provider + model CRUD, chat (sync + stream), usage query.
//!
//! `decide_route()` picks the right provider/model for a chat call:
//!   1. explicit provider_id/model_id → use as-is
//!   2. no provider + fallback ready + enabled → use local fallback
//!   3. no provider + cloud configured → use user default
//!   4. otherwise → error
//!
//! All commands emit metrics via the v0.5.7 MetricsRegistry (one line per
//! command) and go through `require_permission` for authz.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::auth::session::SessionStore;
use crate::auth::session::AuthenticatedUser;
use crate::commands::metrics as m;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::llm::{
    self, fallback, http_client, ChatMessage, ChatRequest, ChatResponse, LlmError,
    LlmProvider, ProviderContext,
};
use crate::AppState;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Provider {
    pub id: String,
    pub code: String,
    pub name: String,
    pub kind: String,
    pub base_url: String,
    pub auth_type: String,
    pub auth_header: Option<String>,
    /// API key write-only: never returned by list/get.
    pub api_key: Option<String>,
    pub settings_json: String,
    pub default_model_id: Option<String>,
    pub enabled: bool,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ProviderCreate {
    pub code: String,
    pub name: String,
    pub kind: String,
    pub base_url: String,
    pub auth_type: String,
    pub auth_header: Option<String>,
    pub api_key: Option<String>,
    pub settings_json: Option<String>,
    pub default_model_id: Option<String>,
    pub enabled: Option<bool>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct ProviderUpdate {
    pub id: String,
    pub name: Option<String>,
    pub base_url: Option<String>,
    pub auth_type: Option<String>,
    pub auth_header: Option<String>,
    /// None or empty string means "leave unchanged".
    pub api_key: Option<String>,
    pub settings_json: Option<String>,
    pub default_model_id: Option<String>,
    pub enabled: Option<bool>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Model {
    pub id: String,
    pub provider_id: String,
    pub code: String,
    pub display_name: String,
    pub capabilities: String,
    pub context_window: i32,
    pub max_output: i32,
    pub pricing_json: String,
    pub enabled: bool,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ModelCreate {
    pub provider_id: String,
    pub code: String,
    pub display_name: String,
    pub capabilities: Option<String>,
    pub context_window: Option<i32>,
    pub max_output: Option<i32>,
    pub pricing_json: Option<String>,
    pub enabled: Option<bool>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct ModelUpdate {
    pub id: String,
    pub display_name: Option<String>,
    pub capabilities: Option<String>,
    pub context_window: Option<i32>,
    pub max_output: Option<i32>,
    pub pricing_json: Option<String>,
    pub enabled: Option<bool>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct UsageRow {
    pub id: String,
    pub ts_unix_ms: i64,
    pub user_id: String,
    pub provider_id: String,
    pub model_id: String,
    pub capability: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    pub cost_usd: f64,
    pub latency_ms: i32,
    pub success: bool,
    pub error: Option<String>,
    pub request_id: String,
}

// ---------------------------------------------------------------------------
// Providers
// ---------------------------------------------------------------------------

fn row_to_provider(row: &rusqlite::Row<'_>) -> rusqlite::Result<Provider> {
    Ok(Provider {
        id: row.get("id")?,
        code: row.get("code")?,
        name: row.get("name")?,
        kind: row.get("kind")?,
        base_url: row.get("base_url")?,
        auth_type: row.get("auth_type")?,
        auth_header: row.get("auth_header")?,
        // Never return the API key — the frontend only ever writes it.
        api_key: None,
        settings_json: row.get("settings_json")?,
        default_model_id: row.get("default_model_id")?,
        enabled: row.get::<_, i64>("enabled")? != 0,
        sort_order: row.get("sort_order")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

#[tauri::command]
pub fn llm_providers_list(
    state: State<AppState>,
    token: String,
) -> Result<Vec<Provider>, AppError> {
    let _t = m::time(&state.metrics, "llm_providers_list");
    let _user = require_perm(&state, &token, "llm:manage")?;
    state
        .db
        .with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT id, code, name, kind, base_url, auth_type, auth_header,
                        settings_json, default_model_id, enabled, sort_order,
                        created_at, updated_at
                 FROM llm_providers ORDER BY sort_order, name",
            )?;
            let rows = stmt
                .query_map([], row_to_provider)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub fn llm_providers_get(
    state: State<AppState>,
    token: String,
    id: String,
) -> Result<Provider, AppError> {
    let _t = m::time(&state.metrics, "llm_providers_get");
    let _user = require_perm(&state, &token, "llm:manage")?;
    state
        .db
        .with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT id, code, name, kind, base_url, auth_type, auth_header,
                        settings_json, default_model_id, enabled, sort_order,
                        created_at, updated_at
                 FROM llm_providers WHERE id = ?1",
            )?;
            stmt.query_row([&id], row_to_provider)
                .map_err(|e| AppError::Internal(format!("provider not found: {e}")))
        })
}

#[tauri::command]
pub fn llm_providers_create(
    state: State<AppState>,
    token: String,
    payload: ProviderCreate,
) -> Result<Provider, AppError> {
    let _t = m::time(&state.metrics, "llm_providers_create");
    let _user = require_perm(&state, &token, "llm:manage")?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_iso();
    let encrypted = match &payload.api_key {
        Some(s) if !s.is_empty() => Some(
            state
                .master_key
                .encrypt(s)
                .map_err(|e| AppError::Internal(format!("encrypt api_key: {e}")))?,
        ),
        _ => None,
    };
    let api_key_bytes = encrypted.as_deref();
    state
        .db
        .with_tx(|tx| {
            tx.execute(
                "INSERT INTO llm_providers (id, code, name, kind, base_url, auth_type,
                                            auth_header, api_key_enc, settings_json,
                                            default_model_id, enabled, sort_order,
                                            created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                rusqlite::params![
                    id,
                    payload.code,
                    payload.name,
                    payload.kind,
                    payload.base_url,
                    payload.auth_type,
                    payload.auth_header,
                    api_key_bytes,
                    payload.settings_json.unwrap_or_else(|| "{}".to_string()),
                    payload.default_model_id,
                    payload.enabled.unwrap_or(true) as i64,
                    payload.sort_order.unwrap_or(0),
                    now,
                    now,
                ],
            )?;
            Ok(())
        })
        .map_err(|e| AppError::Internal(e.to_string()))?;
    llm_providers_get(state, token, id)
}

#[tauri::command]
pub fn llm_providers_update(
    state: State<AppState>,
    token: String,
    payload: ProviderUpdate,
) -> Result<Provider, AppError> {
    let _t = m::time(&state.metrics, "llm_providers_update");
    let _user = require_perm(&state, &token, "llm:manage")?;
    let now = now_iso();

    // Encrypt new key if provided.
    let new_encrypted: Option<Vec<u8>> = match &payload.api_key {
        Some(s) if !s.is_empty() => Some(
            state
                .master_key
                .encrypt(s)
                .map_err(|e| AppError::Internal(format!("encrypt api_key: {e}")))?,
        ),
        _ => None,
    };

    state
        .db
        .with_tx(|tx| {
            // Use COALESCE-style update: only overwrite fields the caller passed.
            tx.execute(
                "UPDATE llm_providers SET
                    name          = COALESCE(?2, name),
                    base_url      = COALESCE(?3, base_url),
                    auth_type     = COALESCE(?4, auth_type),
                    auth_header   = ?5,
                    settings_json = COALESCE(?6, settings_json),
                    default_model_id = ?7,
                    enabled       = COALESCE(?8, enabled),
                    sort_order    = COALESCE(?9, sort_order),
                    updated_at    = ?10
                 WHERE id = ?1",
                rusqlite::params![
                    payload.id,
                    payload.name,
                    payload.base_url,
                    payload.auth_type,
                    payload.auth_header,
                    payload.settings_json,
                    payload.default_model_id,
                    payload.enabled.map(|b| b as i64),
                    payload.sort_order,
                    now,
                ],
            )?;
            if let Some(ct) = new_encrypted.as_deref() {
                tx.execute(
                    "UPDATE llm_providers SET api_key_enc = ?2 WHERE id = ?1",
                    rusqlite::params![payload.id, ct],
                )?;
            }
            Ok(())
        })
        .map_err(|e| AppError::Internal(e.to_string()))?;
    llm_providers_get(state, token, payload.id)
}

#[tauri::command]
pub fn llm_providers_delete(
    state: State<AppState>,
    token: String,
    id: String,
) -> Result<(), AppError> {
    let _t = m::time(&state.metrics, "llm_providers_delete");
    let _user = require_perm(&state, &token, "llm:manage")?;
    state
        .db
        .with_conn(|c| {
            c.execute("DELETE FROM llm_providers WHERE id = ?1", [&id])?;
            Ok(())
        })
        .map_err(|e| AppError::Internal(e.to_string()))
}

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

fn row_to_model(row: &rusqlite::Row<'_>) -> rusqlite::Result<Model> {
    Ok(Model {
        id: row.get("id")?,
        provider_id: row.get("provider_id")?,
        code: row.get("code")?,
        display_name: row.get("display_name")?,
        capabilities: row.get("capabilities")?,
        context_window: row.get("context_window")?,
        max_output: row.get("max_output")?,
        pricing_json: row.get("pricing_json")?,
        enabled: row.get::<_, i64>("enabled")? != 0,
        sort_order: row.get("sort_order")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

#[tauri::command]
pub fn llm_models_list(
    state: State<AppState>,
    token: String,
    provider_id: Option<String>,
) -> Result<Vec<Model>, AppError> {
    let _t = m::time(&state.metrics, "llm_models_list");
    let _user = require_perm(&state, &token, "llm:manage")?;
    state
        .db
        .with_conn(|c| {
            let (sql, has_filter): (&str, bool) = if provider_id.is_some() {
                (
                    "SELECT id, provider_id, code, display_name, capabilities,
                            context_window, max_output, pricing_json, enabled,
                            sort_order, created_at, updated_at
                     FROM llm_models WHERE provider_id = ?1 ORDER BY sort_order, display_name",
                    true,
                )
            } else {
                (
                    "SELECT id, provider_id, code, display_name, capabilities,
                            context_window, max_output, pricing_json, enabled,
                            sort_order, created_at, updated_at
                     FROM llm_models ORDER BY sort_order, display_name",
                    false,
                )
            };
            let mut stmt = c.prepare(sql)?;
            let rows = if has_filter {
                stmt.query_map([provider_id.as_deref().unwrap()], row_to_model)?
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                stmt.query_map([], row_to_model)?
                    .collect::<Result<Vec<_>, _>>()?
            };
            Ok(rows)
        })
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub fn llm_models_create(
    state: State<AppState>,
    token: String,
    payload: ModelCreate,
) -> Result<Model, AppError> {
    let _t = m::time(&state.metrics, "llm_models_create");
    let _user = require_perm(&state, &token, "llm:manage")?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_iso();
    state
        .db
        .with_tx(|tx| {
            tx.execute(
                "INSERT INTO llm_models (id, provider_id, code, display_name,
                                          capabilities, context_window, max_output,
                                          pricing_json, enabled, sort_order,
                                          created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                rusqlite::params![
                    id,
                    payload.provider_id,
                    payload.code,
                    payload.display_name,
                    payload.capabilities.unwrap_or_else(|| "[\"chat\",\"stream\"]".to_string()),
                    payload.context_window.unwrap_or(4096),
                    payload.max_output.unwrap_or(2048),
                    payload.pricing_json.unwrap_or_else(|| "{}".to_string()),
                    payload.enabled.unwrap_or(true) as i64,
                    payload.sort_order.unwrap_or(0),
                    now,
                    now,
                ],
            )?;
            Ok(())
        })
        .map_err(|e| AppError::Internal(e.to_string()))?;
    state
        .db
        .with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT id, provider_id, code, display_name, capabilities,
                        context_window, max_output, pricing_json, enabled,
                        sort_order, created_at, updated_at
                 FROM llm_models WHERE id = ?1",
            )?;
            stmt.query_row([&id], row_to_model)
                .map_err(|e| AppError::Internal(format!("model not found: {e}")))
        })
}

#[tauri::command]
pub fn llm_models_update(
    state: State<AppState>,
    token: String,
    payload: ModelUpdate,
) -> Result<Model, AppError> {
    let _t = m::time(&state.metrics, "llm_models_update");
    let _user = require_perm(&state, &token, "llm:manage")?;
    let now = now_iso();
    state
        .db
        .with_tx(|tx| {
            tx.execute(
                "UPDATE llm_models SET
                    display_name   = COALESCE(?2, display_name),
                    capabilities   = COALESCE(?3, capabilities),
                    context_window = COALESCE(?4, context_window),
                    max_output     = COALESCE(?5, max_output),
                    pricing_json   = COALESCE(?6, pricing_json),
                    enabled        = COALESCE(?7, enabled),
                    sort_order     = COALESCE(?8, sort_order),
                    updated_at     = ?9
                 WHERE id = ?1",
                rusqlite::params![
                    payload.id,
                    payload.display_name,
                    payload.capabilities,
                    payload.context_window,
                    payload.max_output,
                    payload.pricing_json,
                    payload.enabled.map(|b| b as i64),
                    payload.sort_order,
                    now,
                ],
            )?;
            Ok(())
        })
        .map_err(|e| AppError::Internal(e.to_string()))?;
    state
        .db
        .with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT id, provider_id, code, display_name, capabilities,
                        context_window, max_output, pricing_json, enabled,
                        sort_order, created_at, updated_at
                 FROM llm_models WHERE id = ?1",
            )?;
            stmt.query_row([&payload.id], row_to_model)
                .map_err(|e| AppError::Internal(format!("model not found: {e}")))
        })
}

#[tauri::command]
pub fn llm_models_delete(
    state: State<AppState>,
    token: String,
    id: String,
) -> Result<(), AppError> {
    let _t = m::time(&state.metrics, "llm_models_delete");
    let _user = require_perm(&state, &token, "llm:manage")?;
    state
        .db
        .with_conn(|c| {
            c.execute("DELETE FROM llm_models WHERE id = ?1", [&id])?;
            Ok(())
        })
        .map_err(|e| AppError::Internal(e.to_string()))
}

// ---------------------------------------------------------------------------
// Chat
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ChatArgs {
    pub provider_id: String,
    pub model_id: String,
    pub messages: Vec<ChatMessage>,
    pub system: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct ChatResult {
    pub content: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub model: Option<String>,
    pub finish_reason: Option<String>,
    pub request_id: String,
}

#[tauri::command]
pub async fn llm_chat(
    state: State<'_, AppState>,
    app: AppHandle,
    token: String,
    args: ChatArgs,
) -> Result<ChatResult, AppError> {
    let _t = m::time(&state.metrics, "llm_chat");
    let _user = require_perm(&state, &token, "llm:use")?;
    let request_id = uuid::Uuid::new_v4().to_string();
    let started = std::time::Instant::now();
    let (provider_row, model_row, ctx) = resolve(&state, &args.provider_id, &args.model_id)?;
    let adapter = pick_adapter(&provider_row.kind);
    let req = ChatRequest {
        model: model_row.code.clone(),
        messages: args.messages.clone(),
        temperature: args.temperature,
        max_tokens: args.max_tokens,
        top_p: None,
        stop: None,
        system: args.system.clone(),
    };
    let result = adapter.chat(&ctx, req).await;
    record_usage(
        &state,
        &app,
        &_user,
        &provider_row,
        &model_row,
        &request_id,
        "chat",
        started,
        &result,
    );
    match result {
        Ok(r) => Ok(ChatResult {
            content: r.content,
            prompt_tokens: r.prompt_tokens,
            completion_tokens: r.completion_tokens,
            total_tokens: r.total_tokens,
            model: r.model,
            finish_reason: r.finish_reason,
            request_id,
        }),
        Err(e) => Err(map_llm_error(e)),
    }
}

#[tauri::command]
pub async fn llm_chat_stream(
    state: State<'_, AppState>,
    app: AppHandle,
    token: String,
    args: ChatArgs,
) -> Result<ChatResult, AppError> {
    let _t = m::time(&state.metrics, "llm_chat_stream");
    let _user = require_perm(&state, &token, "llm:use")?;
    let request_id = uuid::Uuid::new_v4().to_string();
    let started = std::time::Instant::now();
    let (provider_row, model_row, ctx) = resolve(&state, &args.provider_id, &args.model_id)?;
    let adapter = pick_adapter(&provider_row.kind);
    let req = ChatRequest {
        model: model_row.code.clone(),
        messages: args.messages.clone(),
        temperature: args.temperature,
        max_tokens: args.max_tokens,
        top_p: None,
        stop: None,
        system: args.system.clone(),
    };

    let request_id_for_event = request_id.clone();
    let app_for_chunk = app.clone();
    let on_chunk = move |chunk: llm::StreamChunk| {
        let _ = app_for_chunk.emit_all(
            "llm:chunk",
            serde_json::json!({
                "request_id": request_id_for_event,
                "delta": chunk.delta,
                "finish_reason": chunk.finish_reason,
            }),
        );
    };

    let result = adapter.chat_stream(&ctx, req, Box::new(on_chunk)).await;
    record_usage(
        &state,
        &app,
        &_user,
        &provider_row,
        &model_row,
        &request_id,
        "chat",
        started,
        &result,
    );
    match result {
        Ok(r) => Ok(ChatResult {
            content: r.content,
            prompt_tokens: r.prompt_tokens,
            completion_tokens: r.completion_tokens,
            total_tokens: r.total_tokens,
            model: r.model,
            finish_reason: r.finish_reason,
            request_id,
        }),
        Err(e) => Err(map_llm_error(e)),
    }
}

#[tauri::command]
pub fn llm_models_get(
    state: State<AppState>,
    token: String,
    id: String,
) -> Result<Model, AppError> {
    let _t = m::time(&state.metrics, "llm_models_get");
    let _user = require_perm(&state, &token, "llm:manage")?;
    state
        .db
        .with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT id, provider_id, code, display_name, capabilities,
                        context_window, max_output, pricing_json, enabled,
                        sort_order, created_at, updated_at
                 FROM llm_models WHERE id = ?1",
            )?;
            stmt.query_row([&id], row_to_model)
                .map_err(|e| AppError::Internal(format!("model not found: {e}")))
        })
}

#[tauri::command]
pub fn llm_usage_query(
    state: State<AppState>,
    token: String,
    from_unix_ms: Option<i64>,
    to_unix_ms: Option<i64>,
    user_id: Option<String>,
    provider_id: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<UsageRow>, AppError> {
    let _t = m::time(&state.metrics, "llm_usage_query");
    let _user = require_perm(&state, &token, "llm:usage:read")?;
    let limit = limit.unwrap_or(200).clamp(1, 5000);
    state
        .db
        .with_conn(|c| {
            // Build SQL dynamically (keep it simple; only optional WHERE clauses).
            let mut sql = String::from(
                "SELECT id, ts_unix_ms, user_id, provider_id, model_id, capability,
                        prompt_tokens, completion_tokens, total_tokens, cost_usd,
                        latency_ms, success, error, request_id
                 FROM llm_usage WHERE 1=1",
            );
            let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
            if let Some(f) = from_unix_ms {
                sql.push_str(" AND ts_unix_ms >= ?");
                params.push(Box::new(f));
            }
            if let Some(t) = to_unix_ms {
                sql.push_str(" AND ts_unix_ms <= ?");
                params.push(Box::new(t));
            }
            if let Some(u) = &user_id {
                sql.push_str(" AND user_id = ?");
                params.push(Box::new(u.clone()));
            }
            if let Some(p) = &provider_id {
                sql.push_str(" AND provider_id = ?");
                params.push(Box::new(p.clone()));
            }
            sql.push_str(" ORDER BY ts_unix_ms DESC LIMIT ?");
            params.push(Box::new(limit));

            let mut stmt = c.prepare(&sql)?;
            let param_refs: Vec<&dyn rusqlite::ToSql> =
                params.iter().map(|p| p.as_ref() as &dyn rusqlite::ToSql).collect();
            let rows = stmt
                .query_map(&param_refs[..], |row| {
                    Ok(UsageRow {
                        id: row.get("id")?,
                        ts_unix_ms: row.get("ts_unix_ms")?,
                        user_id: row.get("user_id")?,
                        provider_id: row.get("provider_id")?,
                        model_id: row.get("model_id")?,
                        capability: row.get("capability")?,
                        prompt_tokens: row.get("prompt_tokens")?,
                        completion_tokens: row.get("completion_tokens")?,
                        total_tokens: row.get("total_tokens")?,
                        cost_usd: row.get("cost_usd")?,
                        latency_ms: row.get("latency_ms")?,
                        success: row.get::<_, i64>("success")? != 0,
                        error: row.get("error")?,
                        request_id: row.get("request_id")?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })
        .map_err(|e| AppError::Internal(e.to_string()))
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

fn require_perm(state: &State<AppState>, token: &str, perm: &str) -> AppResult<AuthenticatedUser> {
    let user = state.sessions.lookup(token)?;
    crate::auth::rbac::require_permission(&user, perm)?;
    Ok(user)
}

fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%fZ").to_string()
}

fn map_llm_error(e: LlmError) -> AppError {
    AppError::Internal(e.to_string())
}

fn pick_adapter(kind: &str) -> Box<dyn LlmProvider> {
    match kind {
        "anthropic" => Box::new(llm::providers::anthropic::Anthropic),
        "custom" => Box::new(llm::providers::custom::CustomAdapter),
        // 'openai_compat' and 'fallback' both speak the OpenAI schema.
        _ => Box::new(llm::providers::openai_compat::OpenAiCompat),
    }
}

struct ProviderRow {
    id: String,
    kind: String,
}

struct ModelRow {
    id: String,
    code: String,
}

fn resolve(
    state: &State<AppState>,
    provider_id: &str,
    model_id: &str,
) -> Result<(ProviderRow, ModelRow, ProviderContext), AppError> {
    let db = &state.db;
    let master = &state.master_key;

    let (provider_kind, base_url, auth_type, auth_header, api_key_enc, timeout_ms) = db
        .with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT kind, base_url, auth_type, auth_header, api_key_enc, settings_json
                 FROM llm_providers WHERE id = ?1",
            )?;
            let row = stmt
                .query_row([provider_id], |r| {
                    Ok((
                        r.get::<_, String>("kind")?,
                        r.get::<_, String>("base_url")?,
                        r.get::<_, String>("auth_type")?,
                        r.get::<_, Option<String>>("auth_header")?,
                        r.get::<_, Option<Vec<u8>>>("api_key_enc")?,
                        r.get::<_, String>("settings_json")?,
                    ))
                })
                .map_err(|e| AppError::Internal(format!("provider not found: {e}")))?;
            let timeout_ms: u64 = serde_json::from_str::<serde_json::Value>(&row.5)
                .ok()
                .and_then(|v| v.get("timeout_ms").and_then(|t| t.as_u64()))
                .unwrap_or(60_000);
            Ok((
                row.0, row.1, row.2, row.3, row.4, timeout_ms,
            ))
        })
        .map_err(|e: crate::error::AppError| e)?;

    let model_code = db
        .with_conn(|c| {
            let mut stmt = c.prepare("SELECT code FROM llm_models WHERE id = ?1")?;
            stmt.query_row([model_id], |r| r.get::<_, String>(0))
                .map_err(|e| AppError::Internal(format!("model not found: {e}")))
        })?;

    let api_key = match llm::decrypt_api_key(api_key_enc.as_deref(), master) {
        Ok(s) => s,
        Err(e) => return Err(AppError::Internal(format!("api_key decrypt: {e}"))),
    };

    Ok((
        ProviderRow { id: provider_id.to_string(), kind: provider_kind.clone() },
        ModelRow { id: model_id.to_string(), code: model_code },
        ProviderContext {
            provider_id: provider_id.to_string(),
            base_url,
            auth_type,
            auth_header,
            api_key,
            timeout_ms,
            http: http_client(timeout_ms),
        },
    ))
}

fn record_usage(
    state: &State<AppState>,
    _app: &AppHandle,
    user: &AuthenticatedUser,
    provider: &ProviderRow,
    model: &ModelRow,
    request_id: &str,
    capability: &str,
    started: std::time::Instant,
    result: &Result<ChatResponse, LlmError>,
) {
    let latency_ms = started.elapsed().as_millis() as i64;
    let (success, error_msg, prompt_tokens, completion_tokens) = match result {
        Ok(r) => (true, None, r.prompt_tokens as i64, r.completion_tokens as i64),
        Err(e) => (false, Some(e.to_string()), 0, 0),
    };
    let id = uuid::Uuid::new_v4().to_string();
    let ts = fallback::unix_ms();
    let _ = state.db.with_conn(|c| {
        c.execute(
            "INSERT OR IGNORE INTO llm_usage
                (id, ts_unix_ms, user_id, provider_id, model_id, capability,
                 prompt_tokens, completion_tokens, total_tokens, cost_usd,
                 latency_ms, success, error, request_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            rusqlite::params![
                id,
                ts,
                user.user_id,
                provider.id,
                model.id,
                capability,
                prompt_tokens,
                completion_tokens,
                prompt_tokens + completion_tokens,
                0.0_f64,
                latency_ms,
                success as i64,
                error_msg,
                request_id,
            ],
        )?;
        Ok(())
    });
    // Best-effort audit log
    let _ = crate::commands::audit::write(
        &state.db,
        Some(&user.user_id),
        Some(&user.username),
        "llm_chat",
        Some(&provider.id),
        Some(&model.id),
        Some(&serde_json::json!({
            "request_id": request_id,
            "tokens": prompt_tokens + completion_tokens,
            "latency_ms": latency_ms,
            "success": success,
        }).to_string()),
    );
}

// ---------------------------------------------------------------------------
// Fallback state mirror for the frontend
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct FallbackMirror {
    pub state: fallback::FallbackState,
    pub models: Vec<FallbackModelMirror>,
}

#[derive(Debug, Serialize)]
pub struct FallbackModelMirror {
    pub id: &'static str,
    pub display_name: &'static str,
    pub size_bytes: u64,
    pub min_ram_gb: u32,
    pub primary_url: &'static str,
}

#[tauri::command]
pub fn llm_fallback_status(state: State<AppState>) -> Result<FallbackMirror, AppError> {
    let _t = m::time(&state.metrics, "llm_fallback_status");
    let mgr = state.fallback.clone();
    let state_clone = mgr.state();
    let models = fallback::MODELS
        .iter()
        .map(|m| FallbackModelMirror {
            id: m.id,
            display_name: m.display_name,
            size_bytes: m.size_bytes,
            min_ram_gb: m.min_ram_gb,
            primary_url: m.primary_url,
        })
        .collect();
    Ok(FallbackMirror { state: state_clone, models })
}

#[tauri::command]
pub fn llm_fallback_select_model(
    state: State<AppState>,
    token: String,
    model_id: String,
) -> Result<(), AppError> {
    let _t = m::time(&state.metrics, "llm_fallback_select_model");
    let _user = require_perm(&state, &token, "llm:manage")?;
    state
        .fallback
        .set_selected_model(Some(model_id))
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub fn llm_fallback_set_enabled(
    state: State<AppState>,
    token: String,
    enabled: bool,
) -> Result<(), AppError> {
    let _t = m::time(&state.metrics, "llm_fallback_set_enabled");
    let _user = require_perm(&state, &token, "llm:manage")?;
    state
        .fallback
        .set_enabled(enabled)
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub fn llm_fallback_dismiss_startup_prompt(
    state: State<AppState>,
    token: String,
) -> Result<(), AppError> {
    let _t = m::time(&state.metrics, "llm_fallback_dismiss_startup_prompt");
    let _user = require_perm(&state, &token, "llm:manage")?;
    state
        .db
        .with_conn(|c| {
            c.execute(
                "UPDATE llm_fallback_config SET notify_on_start = 0, updated_at = ?2 WHERE id = 1",
                rusqlite::params![1i64, now_iso()],
            )?;
            Ok(())
        })
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub fn llm_fallback_startup_prompt_needed(
    state: State<AppState>,
    token: String,
) -> Result<bool, AppError> {
    let _user = require_perm(&state, &token, "llm:use")?;
    let notify: bool = state
        .db
        .with_conn(|c| {
            let n: i64 = c
                .query_row(
                    "SELECT notify_on_start FROM llm_fallback_config WHERE id = 1",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or(1);
            Ok(n != 0)
        })
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let state_now = state.fallback.state();
    let any_provider = state
        .db
        .with_conn(|c| {
            let n: i64 = c.query_row(
                "SELECT COUNT(*) FROM llm_providers WHERE enabled = 1",
                [],
                |r| r.get(0),
            )?;
            Ok(n > 0)
        })
        .map_err(|e: crate::error::AppError| e)?;
    Ok(notify && !any_provider && matches!(state_now.phase, fallback::Phase::NotDownloaded))
}

// Suppress unused imports if SessionStore is moved elsewhere later.
#[allow(dead_code)]
fn _unused_session(_s: &SessionStore) {}