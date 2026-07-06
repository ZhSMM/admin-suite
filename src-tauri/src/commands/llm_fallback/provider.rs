//! DB helpers that wire the running llama-server into the regular
//! `llm_providers` / `llm_models` tables so the chat UI picks it up
//! like any other provider.
//!
//! These functions are private to `commands::llm_fallback::server`
//! — keep that surface small.

use std::sync::Arc;

use rusqlite::params;

use crate::db::Db;

/// Probe up to 64 consecutive free TCP ports starting from `start`.
pub fn probe_free_port(start: u16) -> Option<u16> {
    for p in start..start + 64 {
        if std::net::TcpListener::bind(("127.0.0.1", p)).is_ok() {
            return Some(p);
        }
    }
    None
}

/// Insert-or-update the `fallback-local` provider + model rows so the
/// rest of the system can route through the llama-server child process.
/// Always sets `enabled = 1`.
pub fn upsert_fallback_provider(db: &Arc<Db>, port: u16) -> Result<(), String> {
    let provider_id = "fallback-local";
    let provider_code = "fallback-local";
    let provider_name = "Local Fallback (llama-server)";
    let base_url = format!("http://127.0.0.1:{}/v1", port);
    // Use the selected model code from fallback state, or fall back to
    // a known sentinel so the row has usable `code` even before any
    // install completes.
    let model_id = "fallback-local-model";
    let model_code = db
        .with_conn(|c| {
            let s: Option<String> = c
                .query_row(
                    "SELECT selected_model_id FROM llm_fallback_config WHERE id = 1",
                    [],
                    |r| r.get(0),
                )
                .ok()
                .flatten();
            Ok(s.unwrap_or_else(|| "qwen2.5-1.5b-instruct-q4km".to_string()))
        })
        .map_err(|e| format!("read fallback config: {e}"))?;
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%fZ").to_string();
    db.with_tx(|tx| {
        tx.execute(
            "INSERT INTO llm_providers
                 (id, code, name, kind, base_url, auth_type, auth_header,
                  api_key_enc, settings_json, default_model_id, enabled,
                  sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'fallback', ?4, 'none', NULL,
                     NULL, '{}', ?5, 1, 100, ?6, ?6)
             ON CONFLICT(id) DO UPDATE SET
                 base_url = excluded.base_url,
                 default_model_id = excluded.default_model_id,
                 enabled = 1,
                 updated_at = excluded.updated_at",
            params![provider_id, provider_code, provider_name, base_url, model_id, now],
        )?;
        tx.execute(
            "INSERT INTO llm_models
                 (id, provider_id, code, display_name, context_window,
                  capabilities_json, enabled, sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 8192,
                     '{\"chat\":true}', 1, 100, ?5, ?5)
             ON CONFLICT(id) DO UPDATE SET
                 provider_id = excluded.provider_id,
                 enabled = 1,
                 updated_at = excluded.updated_at",
            params![model_id, provider_id, model_code, model_code, now],
        )?;
        Ok(())
    })
    .map_err(|e| format!("upsert fallback provider: {e}"))?;
    Ok(())
}

/// Flip the `enabled` flag on the `fallback-local` provider + model
/// rows. Used when the llama-server stops, or when the model is
/// removed.
pub fn set_fallback_provider_enabled(db: &Arc<Db>, enabled: bool) -> Result<(), String> {
    db.with_conn(|c| {
        c.execute(
            "UPDATE llm_providers SET enabled = ?1, updated_at = ?2
             WHERE id = 'fallback-local'",
            params![enabled as i64, chrono::Utc::now().format("%Y-%m-%dT%H:%M:%fZ").to_string()],
        )?;
        c.execute(
            "UPDATE llm_models SET enabled = ?1, updated_at = ?2
             WHERE id = 'fallback-local-model'",
            params![enabled as i64, chrono::Utc::now().format("%Y-%m-%dT%H:%M:%fZ").to_string()],
        )?;
        Ok(())
    })
    .map_err(|e| format!("set fallback provider enabled: {e}"))
}
