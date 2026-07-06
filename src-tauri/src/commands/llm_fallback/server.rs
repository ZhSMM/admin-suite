//! Stage 2: start/stop the local llama-server child process, plus
//! local-file import ("I already downloaded it via IDM") and the
//! disk-free / trending-model discovery commands.

use tauri::State;

use crate::commands::llm as llm_cmd;
use crate::commands::metrics as m;
use crate::error::{AppError, AppResult};
use crate::llm::fallback;
use crate::AppState;

use super::provider::{probe_free_port, set_fallback_provider_enabled, upsert_fallback_provider};

#[tauri::command]
pub async fn llm_fallback_server_start(
    state: State<'_, AppState>,
    token: String,
) -> Result<u16, AppError> {
    let _t = m::time(&state.metrics, "llm_fallback_server_start");
    let _user = llm_cmd::require_perm(&state, &token, "llm:manage")?;
    let mgr = state.fallback.clone();
    let port = probe_free_port(39135).unwrap_or(39135);
    let _exe = mgr.ensure_server(port).map_err(AppError::Internal)?;
    upsert_fallback_provider(&state.db, port).map_err(AppError::Internal)?;
    Ok(port)
}

#[tauri::command]
pub async fn llm_fallback_server_stop(
    state: State<'_, AppState>,
    token: String,
) -> Result<(), AppError> {
    let _t = m::time(&state.metrics, "llm_fallback_server_stop");
    let _user = llm_cmd::require_perm(&state, &token, "llm:manage")?;
    state.fallback.kill_server();
    set_fallback_provider_enabled(&state.db, false).map_err(AppError::Internal)?;
    Ok(())
}

#[tauri::command]
pub async fn llm_fallback_remove(
    state: State<'_, AppState>,
    token: String,
) -> Result<(), AppError> {
    let _t = m::time(&state.metrics, "llm_fallback_remove");
    let _user = llm_cmd::require_perm(&state, &token, "llm:manage")?;
    state.fallback.kill_server();
    state
        .fallback
        .remove_model()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    set_fallback_provider_enabled(&state.db, false).map_err(AppError::Internal)?;
    Ok(())
}

/// Register an externally-downloaded model file. User picks the file via
/// a native file dialog (or pastes a URL they downloaded themselves with
/// IDM / Aria2 / browser etc.) and we verify it then mark the model
/// Ready without re-downloading. Useful when the user has a faster path
/// to the file than our Rust HTTP client (e.g. cross-border throttling).
///
/// Behaviour:
///   - `source_path`: absolute path to the .gguf file
///   - `expected_sha256`: optional; if provided, must match (otherwise error)
///   - The file is COPIED to `<data_dir>/llm/models/<id>.gguf` so the
///     path is consistent with what the llama-server spawn expects.
///   - Phase transitions to Ready and the fallback provider row is
///     upserted so the rest of the app can route through it.
#[tauri::command]
pub async fn llm_fallback_import_local(
    state: State<'_, AppState>,
    token: String,
    model_id: String,
    source_path: String,
    expected_sha256: Option<String>,
) -> Result<u64, AppError> {
    let _t = m::time(&state.metrics, "llm_fallback_import_local");
    let _user = llm_cmd::require_perm(&state, &token, "llm:manage")?;
    let spec = fallback::find_spec(&model_id)
        .ok_or_else(|| AppError::Validation(format!("unknown model: {model_id}")))?;

    let src = std::path::PathBuf::from(&source_path);
    if !src.is_file() {
        return Err(AppError::Validation(format!("not a file: {source_path}")));
    }
    let size = std::fs::metadata(&src)
        .map_err(|e| AppError::Internal(format!("stat: {e}")))?
        .len();

    // Sanity check: size should be within 50% of the spec estimate.
    let est = spec.size_estimate_bytes;
    if est > 0 {
        let ratio = size as f64 / est as f64;
        if !(0.5..=1.5).contains(&ratio) {
            return Err(AppError::Validation(format!(
                "size {} bytes is too far from expected ~{} bytes (ratio {:.2}); wrong file?",
                size, est, ratio
            )));
        }
    }

    if let Some(expected) = expected_sha256.as_deref() {
        if !expected.is_empty() {
            let actual = fallback::download::sha256_file(&src)
                .map_err(|e| AppError::Internal(format!("sha256: {e}")))?;
            if actual != expected {
                return Err(AppError::Validation(format!(
                    "sha256 mismatch: actual={} expected={}",
                    actual, expected
                )));
            }
        }
    }

    let mgr = state.fallback.clone();
    mgr.kill_server();
    let dest = mgr.model_file_path(&model_id);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::Internal(format!("mkdir: {e}")))?;
    }
    std::fs::copy(&src, &dest).map_err(|e| {
        AppError::Internal(format!(
            "copy {} -> {}: {e}",
            src.display(),
            dest.display()
        ))
    })?;

    let _ = mgr.update_phase(fallback::Phase::Ready {
        path: dest.clone(),
        downloaded_at_unix_ms: fallback::unix_ms(),
    });
    let _ = mgr.set_selected_model(Some(model_id.clone()));

    Ok(size)
}

/// Free disk space (in bytes) on the volume holding `<data_dir>`.
/// Used by the Settings → AI panel to warn before kicking off a 1GB+
/// download.
#[tauri::command]
pub fn llm_fallback_disk_free(state: State<'_, AppState>) -> Result<u64, AppError> {
    let _t = m::time(&state.metrics, "llm_fallback_disk_free");
    let dir = state.fallback.data_dir();
    fs2::available_space(&dir).map_err(|e| AppError::Internal(format!("disk_free: {e}")))
}

/// Discover top-N trending GGUF-Instruct models from HuggingFace.
/// Used by the "popular models" recommendation panel.
#[tauri::command]
pub async fn llm_fallback_discover_trending(
    state: State<'_, AppState>,
    _token: String,
    limit: Option<usize>,
) -> Result<Vec<fallback::TrendingModel>, AppError> {
    let _t = m::time(&state.metrics, "llm_fallback_discover_trending");
    let _user = llm_cmd::require_perm(&state, &_token, "llm:use")?;
    fallback::discover_trending(limit.unwrap_or(12))
        .await
        .map_err(AppError::Internal)
}
