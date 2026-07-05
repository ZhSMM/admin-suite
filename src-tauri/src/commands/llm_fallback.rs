//! v0.6.2 — One-click local install: download + run the local fallback model.
//!
//! Frontend calls into these commands to drive the full lifecycle:
//!
//!   1. `llm_fallback_install_start`  — kick off the two-stage download
//!      (server binary, then GGUF model). Emits `llm:fallback:progress`
//!      events for the UI to render a single weighted progress bar.
//!   2. `llm_fallback_install_cancel` — flip the cancel flag.
//!   3. `llm_fallback_server_start`   — spawn llama-server, upsert the
//!      `llm_providers` row of `kind = "fallback"` so the rest of the
//!      system sees it.
//!   4. `llm_fallback_server_stop`    — kill llama-server, mark provider
//!      row `enabled = 0`.
//!   5. `llm_fallback_remove`         — delete model + reset state.
//!
//! The full `status` snapshot is returned by the existing
//! `llm_fallback_status` command (no change). Frontend re-fetches it
//! whenever `llm:fallback:progress` fires.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::commands::llm as llm_cmd;
use crate::commands::metrics as m;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::llm::fallback::{
    self, DownloadProgress, FallbackManager, Phase,
};
use crate::AppState;

/// Event payload emitted every 500ms during download.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FallbackProgressEvent {
    pub stage: String, // "model" | "server"
    pub bytes_done: u64,
    pub total_bytes: u64,
    pub speed_bps: u64,
    pub eta_seconds: u64,
    pub current_stage: String, // mirrors `stage` for convenience
    pub model_id: String,
}

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
    let model_spec = fallback::find_spec(&model_id).ok_or_else(|| {
        AppError::Validation(format!("unknown fallback model: {}", model_id))
    })?;
    let mgr = state.fallback.clone();

    // If already ready, return early.
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
    // Spawn the actual download work on a separate task so the Tauri
    // command returns immediately and the UI can subscribe to events.
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

    // Return the planned sizes so the UI can show "downloading X MB total".
    // llama-server b3900 zip is roughly 30MB; use a conservative upper bound.
    Ok(InstallStartResult {
        model_id,
        model_size_bytes: model_spec.size_estimate_bytes,
        server_size_bytes: 35_000_000,
        already_installed: false,
        preferred_url,
    })
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
        match fallback::resolve_spec_with_preferred(spec, cache_dir, url).await {
            Ok(r) => return Ok(r),
            Err(_) => {
                // user URL failed validation; fall through to speedtest
            }
        }
    }
    fallback::resolve_spec_with_speedtest(spec, cache_dir).await
}

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
        let progress_cb: Arc<dyn Fn(DownloadProgress) + Send + Sync> = Arc::new(move |p: DownloadProgress| {
            emit_progress(
                &app_for_progress,
                &model_id_for_progress,
                "server",
                &p,
            );
        });
        let res = mgr
            .ensure_llama_server(&guard, progress_cb)
            .await;
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
    // v0.6.5: resolve the model spec against HuggingFace, then probe
    // every mirror's actual download speed and promote the fastest to
    // primary. For users in CN / regions where HF is throttled, this
    // reroutes the install to a fast mirror without any user action.
    let spec_for_resolve = fallback::find_spec(&model_id).expect("checked in install_start");
    let resolved = match resolve_model(spec_for_resolve, &cache_dir, preferred_url.as_deref()).await {
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
    let progress_cb: Arc<dyn Fn(DownloadProgress) + Send + Sync> = Arc::new(move |p: DownloadProgress| {
        emit_progress(
            &app_for_progress,
            &model_id_for_progress,
            "model",
            &p,
        );
    });
    let res = mgr
        .download_model(&resolved, &guard, progress_cb)
        .await;
    match res {
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

fn emit_progress(app: &AppHandle, model_id: &str, stage: &str, p: &DownloadProgress) {
    // Tauri's event-name validator rejects `.` (and other characters), but
    // model_id is user-provided and may contain dots (e.g. the Qwen GGUF
    // id "qwen2.5-1.5b-instruct-q4km"). Emit a SINGLE event with the
    // model_id in the payload; the frontend filters by that field.
    let payload = FallbackProgressEvent {
        stage: stage.to_string(),
        bytes_done: p.bytes_done,
        total_bytes: p.total_bytes,
        speed_bps: p.speed_bps,
        eta_seconds: p.eta_seconds,
        current_stage: stage.to_string(),
        model_id: model_id.to_string(),
    };
    let _ = app.emit_all("llm:fallback:progress", payload);
}

fn emit_done(app: &AppHandle, model_id: &str, success: bool, error: &str) {
    #[derive(Serialize, Clone)]
    #[serde(rename_all = "camelCase")]
    struct DonePayload {
        model_id: String,
        success: bool,
        error: String,
    }
    let _ = app.emit_all(
        "llm:fallback:done",
        DonePayload {
            model_id: model_id.to_string(),
            success,
            error: error.to_string(),
        },
    );
}

#[tauri::command]
pub fn llm_fallback_install_cancel(
    state: tauri::State<'_, AppState>,
    token: String,
) -> Result<bool, AppError> {
    let _t = m::time(&state.metrics, "llm_fallback_install_cancel");
    let _user = llm_cmd::require_perm(&state, &token, "llm:manage")?;
    Ok(state.fallback.cancel_download())
}

#[tauri::command]
pub async fn llm_fallback_server_start(
    state: tauri::State<'_, AppState>,
    token: String,
) -> Result<u16, AppError> {
    let _t = m::time(&state.metrics, "llm_fallback_server_start");
    let _user = llm_cmd::require_perm(&state, &token, "llm:manage")?;
    let mgr = state.fallback.clone();
    // Probe a free port starting from 39135.
    let port = probe_free_port(39135).unwrap_or(39135);
    let _exe = mgr.ensure_server(port).map_err(AppError::Internal)?;
    // Upsert the fallback provider row so the rest of the system can route
    // through it. Set enabled=1.
    upsert_fallback_provider(&state.db, port).map_err(AppError::Internal)?;
    Ok(port)
}

#[tauri::command]
pub async fn llm_fallback_server_stop(
    state: tauri::State<'_, AppState>,
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
    state: tauri::State<'_, AppState>,
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
    state: tauri::State<'_, AppState>,
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

    // Optional SHA256 verify — keeps the contract the auto-install uses.
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

    // Copy to the canonical location.
    let mgr = state.fallback.clone();
    mgr.kill_server();
    let dest = mgr.model_file_path(&model_id);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::Internal(format!("mkdir: {e}")))?;
    }
    std::fs::copy(&src, &dest)
        .map_err(|e| AppError::Internal(format!("copy {} -> {}: {e}", src.display(), dest.display())))?;

    let _ = mgr.update_phase(fallback::Phase::Ready {
        path: dest.clone(),
        downloaded_at_unix_ms: fallback::unix_ms(),
    });
    let _ = mgr.set_selected_model(Some(model_id.clone()));

    Ok(size)
}

/// Returns the free disk space (in bytes) on the volume holding `<data_dir>`.
/// Used by the Settings → AI panel to warn before kicking off a 1GB+ download.
#[tauri::command]
pub fn llm_fallback_disk_free(state: tauri::State<'_, AppState>) -> Result<u64, AppError> {
    let _t = m::time(&state.metrics, "llm_fallback_disk_free");
    let dir = state.fallback.data_dir();
    fs2::available_space(&dir).map_err(|e| AppError::Internal(format!("disk_free: {e}")))
}

/// Discover top-N trending GGUF-Instruct models from HuggingFace.
/// Used by the "popular models" recommendation panel.
#[tauri::command]
pub async fn llm_fallback_discover_trending(
    state: tauri::State<'_, AppState>,
    _token: String,
    limit: Option<usize>,
) -> Result<Vec<fallback::TrendingModel>, AppError> {
    let _t = m::time(&state.metrics, "llm_fallback_discover_trending");
    let _user = llm_cmd::require_perm(&state, &_token, "llm:use")?;
    fallback::discover_trending(limit.unwrap_or(12))
        .await
        .map_err(AppError::Internal)
}

/// Speed-test every mirror for the given model_id (or for a single URL
/// if `manual_url` is set).
///
/// Behaviour:
///   - `model_id`: resolve the spec → real GGUF URLs → test each (kind='download')
///   - `manual_url`: just test that one URL (kind='download', label='manual')
///
/// When `model_id` is provided but resolution fails (HF API unreachable),
/// returns an empty Vec — the UI should then surface a "paste a URL"
/// input so the user can supply a known mirror. We DO NOT fallback to
/// probing root URLs anymore — that produced misleading "URLs" the user
/// couldn't actually paste into IDM.
#[tauri::command]
pub async fn llm_fallback_speed_test(
    state: tauri::State<'_, AppState>,
    _token: String,
    model_id: Option<String>,
    manual_url: Option<String>,
) -> Result<Vec<fallback::SpeedTestResult>, AppError> {
    let _t = m::time(&state.metrics, "llm_fallback_speed_test");
    let _user = llm_cmd::require_perm(&state, &_token, "llm:use")?;

    // User-provided URL: just probe that one.
    if let Some(u) = manual_url.as_deref() {
        if !u.is_empty() {
            return Ok(vec![fallback::speed_test_url(u, "manual", "download").await]);
        }
    }

    // Need a model_id to discover mirrors.
    let model_id = match model_id.as_deref() {
        Some(id) if !id.is_empty() => id,
        _ => {
            return Err(AppError::Validation(
                "either model_id or manual_url required".into(),
            ))
        }
    };
    let spec = fallback::find_spec(model_id)
        .ok_or_else(|| AppError::Validation(format!("unknown model: {model_id}")))?;
    let cache_dir = state.fallback.llm_dir();

    // Resolve spec → real .gguf URLs. If this fails (HF API down) the
    // user should be told to paste a URL — we don't pretend to know
    // their mirror configuration.
    let resolved = fallback::resolve_spec(spec, &cache_dir).await.ok();
    let Some(r) = resolved else {
        return Ok(vec![]);
    };

    let mut urls: Vec<(String, &'static str)> = vec![(r.primary_url, "primary")];
    for m in &r.mirrors {
        let label: &'static str = if m.contains("hf-mirror") {
            "hf-mirror"
        } else if m.contains("modelscope") {
            "modelscope"
        } else {
            "mirror"
        };
        urls.push((m.clone(), label));
    }

    let mut results = Vec::with_capacity(urls.len());
    for (url, label) in urls {
        let u = Box::leak(url.into_boxed_str());
        results.push(fallback::speed_test_url(u, label, "download").await);
    }
    Ok(results)
}

// ---------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------

fn probe_free_port(start: u16) -> Option<u16> {
    for p in start..start + 64 {
        if std::net::TcpListener::bind(("127.0.0.1", p)).is_ok() {
            return Some(p);
        }
    }
    None
}

fn upsert_fallback_provider(db: &Arc<Db>, port: u16) -> Result<(), String> {
    use rusqlite::params;
    let provider_id = "fallback-local";
    let provider_code = "fallback-local";
    let provider_name = "Local Fallback (llama-server)";
    let base_url = format!("http://127.0.0.1:{}/v1", port);
    // We need an llm_models row too — use the selected model code from
    // fallback state, or fall back to a known sentinel.
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

fn set_fallback_provider_enabled(db: &Arc<Db>, enabled: bool) -> Result<(), String> {
    use rusqlite::params;
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

// ---------------------------------------------------------------------
// Local-first routing (called from llm_chat / llm_chat_stream)
// ---------------------------------------------------------------------

/// If `ai.local_first` is on AND the fallback is ready AND has a provider
/// row, rewrite `args.provider_id` / `args.model_id` to the fallback row.
/// Returns true if a reroute happened.
pub fn maybe_reroute_to_local(
    db: &Arc<Db>,
    fallback: &FallbackManager,
    provider_id: &mut String,
    model_id: &mut String,
) -> Result<bool, AppError> {
    let local_first = db
        .with_conn(|c| {
            let v: Option<String> = c
                .query_row(
                    "SELECT value FROM app_state WHERE key = 'ai.local_first'",
                    [],
                    |r| r.get(0),
                )
                .ok();
            Ok(v.unwrap_or_default() == "true")
        })
        .map_err(|e| AppError::Internal(format!("read local_first: {e}")))?;
    if !local_first {
        return Ok(false);
    }
    let snap = fallback.state();
    if !matches!(snap.phase, Phase::Ready { .. }) {
        return Ok(false);
    }
    let has_provider = db
        .with_conn(|c| {
            let n: i64 = c
                .query_row(
                    "SELECT COUNT(*) FROM llm_providers WHERE id = 'fallback-local' AND enabled = 1",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            Ok(n > 0)
        })
        .map_err(|e| AppError::Internal(format!("check fallback provider: {e}")))?;
    if !has_provider {
        return Ok(false);
    }
    if *provider_id == "fallback-local" && *model_id == "fallback-local-model" {
        return Ok(false); // already local
    }
    *provider_id = "fallback-local".to_string();
    *model_id = "fallback-local-model".to_string();
    Ok(true)
}