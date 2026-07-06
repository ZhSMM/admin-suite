//! Local-first routing.
//!
//! Called from `commands::llm::llm_chat` and `llm_chat_stream` BEFORE
//! the resolver runs. If `app_state.ai.local_first == "true"` and the
//! fallback is `Phase::Ready` AND has the `fallback-local` provider
//! row enabled, the call's `(provider_id, model_id)` are rewritten to
//! point at the local engine. Idempotent.

use std::sync::Arc;

use tauri::State;

use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::llm::fallback::{self, FallbackManager, Phase};
use crate::AppState;

/// Mutates `provider_id` and `model_id` to `("fallback-local",
/// "fallback-local-model")` when:
///   - the user has enabled `ai.local_first`;
///   - the local fallback is fully installed (`Phase::Ready`);
///   - the fallback-local provider row exists and is enabled;
///   - the call isn't already going through the fallback.
///
/// Returns `true` if a rewrite happened.
pub fn maybe_reroute_to_local(
    db: &Arc<Db>,
    fallback: &FallbackManager,
    provider_id: &mut String,
    model_id: &mut String,
) -> Result<bool, AppError> {
    if !local_first_enabled(db)? {
        return Ok(false);
    }
    if !matches!(fallback.state().phase, Phase::Ready { .. }) {
        return Ok(false);
    }
    if !fallback_local_enabled(db)? {
        return Ok(false);
    }
    if *provider_id == "fallback-local" && *model_id == "fallback-local-model" {
        return Ok(false);
    }
    *provider_id = "fallback-local".to_string();
    *model_id = "fallback-local-model".to_string();
    Ok(true)
}

fn local_first_enabled(db: &Arc<Db>) -> Result<bool, AppError> {
    db.with_conn(|c| {
        c.query_row(
            "SELECT value FROM app_state WHERE key = 'ai.local_first'",
            [],
            |r| {
                let v: Option<String> = r.get(0).ok();
                Ok(v.unwrap_or_default() == "true")
            },
        )
        .map_err(AppError::from)
    })
    .map_err(|e| AppError::Internal(format!("read local_first: {e}")))
}

fn fallback_local_enabled(db: &Arc<Db>) -> Result<bool, AppError> {
    db.with_conn(|c| {
        c.query_row(
            "SELECT EXISTS(SELECT 1 FROM llm_providers WHERE id = 'fallback-local' AND enabled = 1)",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map_err(AppError::from)
    })
    .map(|n| n > 0)
    .map_err(|e| AppError::Internal(format!("check fallback provider: {e}")))
}

/// Convenience wrapper for callers that don't already hold a
/// `&FallbackManager` (e.g. the streaming variant).
pub fn maybe_reroute_from_state(
    state: &State<'_, AppState>,
    provider_id: &mut String,
    model_id: &mut String,
) -> Result<bool, AppError> {
    maybe_reroute_to_local(&state.db, &state.fallback, provider_id, model_id)
}

#[allow(unused_imports)]
const _UNUSED_FALLBACK_RE_EXPORTS: () = () = {
    use fallback::unix_ms; // keep fallback re-export path live for tests
    let _ = unix_ms;
};
