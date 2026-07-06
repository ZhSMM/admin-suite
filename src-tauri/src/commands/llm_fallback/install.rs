//! Stage 1: install / cancel / run_install.
//!
//! `llm_fallback_install_start` returns immediately with `InstallStartResult`;
//! the actual download work happens in a tokio task spawned by
//! `run_install`. Two stages: download llama-server.exe, then the
//! GGUF model file. Progress is reported through `events::emit_progress`
//! and `events::emit_done` on the `llm:fallback:*` channels.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;
use tauri::AppHandle;

use crate::commands::llm as llm_cmd;
use crate::commands::metrics as m;
use crate::error::{AppError, AppResult};
use crate::llm::fallback::{self, DownloadProgress, FallbackManager, Phase};
use crate::AppState;

use super::events::{emit_done, emit_progress};

/// Result of `llm_fallback_install_start`. Returns immediately; the actual
/// download runs in a tokio task and reports via events.
#[derive(Debug, Serialize)]
pub struct InstallStartResult {
    pub model_id: String,
    pub model_size_bytes: u64,
    pub server_size_bytes: u64,
    pub already_installed: bool,
    /// If non-null, the install will use this URL as primary (the user
    /// picked a specific mirror via the speed-test UI).
    pub preferred_url: Option<String>,
}

/// Tauri command: kick off the two-stage download in a background task.
///
/// If `preferred_url` is set, the resolver uses it as primary (skipping
/// the auto-reroute via speed-test). Otherwise it runs the standard
/// `resolve_spec_with_speedtest` flow.
#[tauri::command]
pub async fn llm_fallback_install_start(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
    token: String,
    model_id: String,
    preferred_url: Option<String>,
) -> Result<InstallStartResult, AppError> {
    let _t = m::time(&state.metrics, "llm_fallback_install_start");
    let _user = llm_cmd::require_perm(&state, &token, "llm:manage")?;
    let model_spec = fallback::find_spec(&model_id)
        .ok_or_else(|| AppError::Validation(format!("unknown fallback model: {}", model_id)))?;
    let mgr = state.fallback.clone();

    let snapshot = mgr.state();
    if matches!(snapshot.phase, Phase::Ready { .. }) {
        let server_size = std::fs::metadata(mgr.llama_server_binary_path())
            .map(|m| m.len())
            .unwrap_or(0);
        let model_size = std::fs::metadata(mgr.model_file_path(&model_id))
            .map(|m| m.len())
            .unwrap_or(model_spec.size_estimate_bytes);
        return Ok(InstallStartResult {
            model_id,
            model_size_bytes: model_size,
            server_size_bytes: server_size,
            already_installed: true,
            preferred_url,
        });
    }

    let app_clone = app.clone();
    let model_id_owned = model_id.clone();
    let model_id_for_event = model_id.clone();
    let mgr_for_task = mgr.clone();
    let cache_dir_for_task = state.fallback.llm_dir();
    let preferred_url_for_task = preferred_url.clone();

    tokio::spawn(async move {
        run_install(
            app_clone,
            mgr_for_task,
            cache_dir_for_task,
            model_id_for_event,
            model_id_owned,
            preferred_url_for_task,
        )
        .await;
    });

    // 35 MiB is a conservative upper bound for llama-server b3900 zip.
    Ok(InstallStartResult {
        model_id,
        model_size_bytes: model_spec.size_estimate_bytes,
        server_size_bytes: 35_000_000,
        already_installed: false,
        preferred_url,
    })
}

/// Tauri command: flip the cancel flag for the in-flight install.
#[tauri::command]
pub fn llm_fallback_install_cancel(
    state: tauri::State<'_, AppState>,
    token: String,
) -> Result<bool, AppError> {
    let _t = m::time(&state.metrics, "llm_fallback_install_cancel");
    let _user = llm_cmd::require_perm(&state, &token, "llm:manage")?;
    Ok(state.fallback.cancel_download())
}

/// Resolve the spec — using the user-picked URL if provided, otherwise
/// running the auto-reroute speedtest. User-picked URLs that fail HEAD
/// validation fall through to the auto-rerouted result.
async fn resolve_model(
    spec: &fallback::FallbackModelSpec,
    cache_dir: &Path,
    preferred_url: Option<&str>,
) -> Result<fallback::ResolvedModel, String> {
    if let Some(url) = preferred_url {
        if fallback::resolve_spec_with_preferred(spec, cache_dir, url)
            .await
            .is_ok()
        {
            return fallback::resolve_spec_with_preferred(spec, cache_dir, url).await;
        }
        // user URL failed validation; fall through to speedtest
    }
    fallback::resolve_spec_with_speedtest(spec, cache_dir).await
}

#[allow(clippy::too_many_arguments)]
async fn run_install(
    app: AppHandle,
    mgr: FallbackManager,
    cache_dir: PathBuf,
    model_id: String,
    model_id_for_log: String,
    preferred_url: Option<String>,
) {
    // Stage 1: ensure llama-server.exe (download + extract).
    let server_path = mgr.llama_server_binary_path();
    if !server_path.exists() {
        let stage = "server".to_string();
        let guard = mgr.begin_download(&stage);
        let app_for_progress = app.clone();
        let model_id_for_progress = model_id_for_log.clone();
        let progress_cb: Arc<dyn Fn(DownloadProgress) + Send + Sync> =
            Arc::new(move |p: DownloadProgress| {
                emit_progress(&app_for_progress, &model_id_for_progress, "server", &p);
            });
        let res = mgr.ensure_llama_server(&guard, progress_cb).await;
        if let Err(e) = res {
            let _ = mgr.update_phase(Phase::Error {
                message: format!("server download: {}", e),
            });
            let _ = mgr.cancel_download();
            emit_done(&app, &model_id, false, &format!("{}", e));
            return;
        }
        drop(guard); // clears manager download slot
    }

    // Stage 2: download the GGUF model file.
    //
    // v0.6.5: resolve against HF, probe mirrors, promote fastest to
    // primary. v0.7.1: user can pin a specific URL via the speed-test UI.
    // v0.8: this stage will switch to `multi_download::download` when
    // the GGUF origin advertises `Accept-Ranges: bytes`.
    let spec_for_resolve = fallback::find_spec(&model_id).expect("checked in install_start");
    let resolved = match resolve_model(spec_for_resolve, &cache_dir, preferred_url.as_deref()).await
    {
        Ok(r) => r,
        Err(e) => {
            let _ = mgr.update_phase(Phase::Error {
                message: format!("resolve {}: {}", model_id, e),
            });
            emit_done(&app, &model_id, false, &format!("resolve: {}", e));
            return;
        }
    };
    let stage = "model".to_string();
    let guard = mgr.begin_download(&stage);
    let app_for_progress = app.clone();
    let model_id_for_progress = model_id.clone();
    let progress_cb: Arc<dyn Fn(DownloadProgress) + Send + Sync> =
        Arc::new(move |p: DownloadProgress| {
            emit_progress(&app_for_progress, &model_id_for_progress, "model", &p);
        });
    let model_path = mgr.model_file_path(&model_id);
    // v0.8: prefer Range-aware parallel download — `multi::download`
    // returns immediately if the server doesn't honour Range, so
    // this is always safe.
    let cancel_for_dl = mgr.cancel_flag();
    let res = fallback::multi::download(
        &resolved.primary_url,
        &model_path,
        cancel_for_dl,
        progress_cb,
    )
    .await;
    let final_path = match res {
        Ok(_) => Ok(model_path),
        Err(e) => Err(e),
    };
    match final_path {
        Ok(path) => {
            let _ = mgr.update_phase(Phase::Ready {
                path: path.clone(),
                downloaded_at_unix_ms: fallback::unix_ms(),
            });
            let _ = mgr.set_selected_model(Some(model_id.clone()));
            drop(guard);
            emit_done(&app, &model_id, true, "");
        }
        Err(e) => {
            let _ = mgr.update_phase(Phase::Error {
                message: format!("{}", e),
            });
            drop(guard);
            emit_done(&app, &model_id, false, &format!("{}", e));
        }
    }
}
