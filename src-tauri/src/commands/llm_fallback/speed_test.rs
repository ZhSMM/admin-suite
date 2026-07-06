//! Streaming mirror speed-test (v0.7.2).
//!
//! Returns immediately; results stream via the
//! `llm:fallback:speed-test` event channel so the UI can paint one
//! row per mirror as soon as its probe finishes.
//!
//! ## Event payload (camelCase on the wire)
//!
//! ```text
//! { kind:    "started" | "result" | "done" | "cancelled",
//!   modelId: string,
//!   index:   usize,                 // row index when applicable
//!   total:   usize,
//!   queue?:  [ { url, label, kind } ],  // sent on `started` only
//!   result?:  SpeedTestResult,
//!   error?:   string }
//! ```
//!
//! The `queue` field carries the (url, label, kind) list we already
//! know up front so the UI can render the panel even while the
//! probes are running — addresses the "no URL, no actions" complaint.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use once_cell::sync::Lazy;
use tauri::{AppHandle, Manager};

use crate::commands::llm as llm_cmd;
use crate::commands::metrics as m;
use crate::error::{AppError, AppResult};
use crate::llm::fallback;
use crate::AppState;

/// One registry slot for the in-flight speed-test. New tests
/// overwrite the previous cancel flag (deterministic: only one
/// runner at a time). `Once` would be wrong here because tests are
/// expected to repeat.
static SPEED_TEST_STATE: Lazy<Mutex<SpeedTestState>> = Lazy::new(|| {
    Mutex::new(SpeedTestState {
        cancel: AtomicBool::new(false),
        started_at: None,
    })
});

struct SpeedTestState {
    cancel: AtomicBool,
    /// When the run started — used as a soft expiry so a cancel
    /// from a previous pane doesn't accidentally stop a newer
    /// run that overlaps.
    started_at: Option<Instant>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SpeedTestQueueEntry {
    url: String,
    label: String,
    /// "download" | "probe" — the planner originally passes &'static str.
    kind: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SpeedTestEvent {
    kind: String,
    model_id: String,
    index: usize,
    total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<fallback::SpeedTestResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    /// Sent on `started` only — the URL queue so the FE can render
    /// the panel before the first probe completes.
    #[serde(skip_serializing_if = "Option::is_none")]
    queue: Option<Vec<SpeedTestQueueEntry>>,
}

fn emit(app: &AppHandle, payload: SpeedTestEvent) {
    let _ = app.emit_all("llm:fallback:speed-test", payload);
}

fn to_queue_entry(url: &str, label: &str, kind: &'static str) -> SpeedTestQueueEntry {
    SpeedTestQueueEntry {
        url: url.to_string(),
        label: label.to_string(),
        kind: kind.to_string(),
    }
}

/// Tauri command: kick off the speed-test pipeline.
///
/// Behaviour:
///   - `model_id`: resolve the spec → real GGUF URLs → probe each
///     (kind='download'). When resolution fails (HF API unreachable
///     from CN), probe a few well-known mirrors with a guessed
///     filename so the user still gets diagnostics.
///   - `manual_url`: probe exactly that URL (kind='download',
///     label='manual'); resolution + URL queue are skipped.
#[tauri::command]
pub async fn llm_fallback_speed_test(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
    token: String,
    model_id: Option<String>,
    manual_url: Option<String>,
) -> Result<(), AppError> {
    let _t = m::time(&state.metrics, "llm_fallback_speed_test");
    let _user = llm_cmd::require_perm(&state, &token, "llm:use")?;

    // Reset the cancel flag for the new run. Old flag values from a
    // previous test would otherwise poison the new run.
    {
        let mut g = SPEED_TEST_STATE.lock().unwrap();
        g.cancel.store(false, Ordering::Relaxed);
        g.started_at = Some(Instant::now());
    }

    // User-provided URL — single probe. Still streams via the event
    // channel so the UI has one code path.
    if let Some(u) = manual_url.as_deref() {
        if !u.is_empty() {
            let m_id = model_id.clone().unwrap_or_else(|| "manual".into());
            let u_owned = u.to_string();
            let app_c = app.clone();
            tokio::spawn(async move {
                run_manual(&app_c, &u_owned, m_id).await
            });
            return Ok(());
        }
    }

    let model_id = match model_id.as_deref() {
        Some(id) if !id.is_empty() => id.to_string(),
        _ => {
            return Err(AppError::Validation(
                "either model_id or manual_url required".into(),
            ))
        }
    };
    let spec = fallback::find_spec(&model_id)
        .ok_or_else(|| AppError::Validation(format!("unknown fallback model: {model_id}")))?;
    let cache_dir = state.fallback.llm_dir();

    // Resolve spec → real .gguf URLs. If this fails (HF API unreachable
    // from CN), probe the candidate mirrors directly using a guessed
    // filename so the user still gets useful diagnostics.
    let resolved = fallback::resolve_spec(spec, &cache_dir).await.ok();
    let urls: Vec<(String, String, &'static str)> = if let Some(r) = resolved {
        let mut v: Vec<(String, String, &'static str)> =
            vec![(r.primary_url, "primary".to_string(), "download")];
        for m in &r.mirrors {
            let label = if m.contains("hf-mirror") {
                "hf-mirror"
            } else if m.contains("modelscope") {
                "modelscope"
            } else {
                "mirror"
            };
            v.push((m.clone(), label.to_string(), "download"));
        }
        v
    } else {
        probe_known_mirrors_for_spec(spec)
            .into_iter()
            .map(|(u, l)| (u, l, "probe"))
            .collect()
    };

    // Pre-compute the queue we ship in the `started` event so the UI
    // can paint the URL column before the first probe finishes.
    let queue: Vec<SpeedTestQueueEntry> = urls
        .iter()
        .map(|(u, l, k)| to_queue_entry(u, l, *k))
        .collect();
    let total = urls.len();
    let app_clone = app.clone();
    let mid = model_id.clone();
    tokio::spawn(async move { run_queue(&app_clone, mid, urls, queue).await });
    Ok(())
}

/// Tauri command: cancel an in-flight speed-test run by flipping the
/// shared cancel flag.
#[tauri::command]
pub fn llm_fallback_speed_test_cancel(_token: String) -> Result<bool, AppError> {
    let g = SPEED_TEST_STATE.lock().unwrap();
    // Only flip the flag if there's a recent run (under 5 min old) so
    // a stale cancel from a long-dead tab doesn't break a fresh run
    // (the flag is single-slot — see comments at the top of the file).
    if let Some(at) = g.started_at {
        if at.elapsed() < Duration::from_secs(5 * 60) {
            g.cancel.store(true, Ordering::Relaxed);
            return Ok(true);
        }
    }
    Ok(false)
}

/// Serial `started → result* → done` for a single user-provided URL.
async fn run_manual(app: &AppHandle, url: &str, m_id: String) {
    let queue = vec![to_queue_entry(url, "manual", "download")];
    emit(
        app,
        SpeedTestEvent {
            kind: "started".into(),
            model_id: m_id.clone(),
            index: 0,
            total: 1,
            result: None,
            error: None,
            queue: Some(queue),
        },
    );
    let r = fallback::speed_test_url(url, "manual", "download").await;
    if cancelled() {
        emit_cancelled(app, &m_id, 1);
        return;
    }
    emit(
        app,
        SpeedTestEvent {
            kind: "result".into(),
            model_id: m_id.clone(),
            index: 0,
            total: 1,
            result: Some(r),
            error: None,
            queue: None,
        },
    );
    emit(
        app,
        SpeedTestEvent {
            kind: "done".into(),
            model_id: m_id,
            index: 1,
            total: 1,
            result: None,
            error: None,
            queue: None,
        },
    );
}

/// Serial `started → result* → done` for a precomputed URL queue.
async fn run_queue(
    app: &AppHandle,
    m_id: String,
    urls: Vec<(String, String, &'static str)>,
    queue: Vec<SpeedTestQueueEntry>,
) {
    let total = urls.len();
    emit(
        app,
        SpeedTestEvent {
            kind: "started".into(),
            model_id: m_id.clone(),
            index: 0,
            total,
            result: None,
            error: None,
            queue: Some(queue),
        },
    );
    for (i, (url, label, kind)) in urls.into_iter().enumerate() {
        if cancelled() {
            emit_cancelled(app, &m_id, i);
            return;
        }
        let r = fallback::speed_test_url(&url, &label, kind).await;
        if cancelled() {
            emit_cancelled(app, &m_id, i + 1);
            return;
        }
        emit(
            app,
            SpeedTestEvent {
                kind: "result".into(),
                model_id: m_id.clone(),
                index: i,
                total,
                result: Some(r),
                error: None,
                queue: None,
            },
        );
    }
    emit(
        app,
        SpeedTestEvent {
            kind: "done".into(),
            model_id: m_id,
            index: total,
            total,
            result: None,
            error: None,
            queue: None,
        },
    );
}

/// Build candidate `(url, label)` pairs by guessing a filename and
/// probing each well-known mirror base.
fn probe_known_mirrors_for_spec(
    spec: &fallback::FallbackModelSpec,
) -> Vec<(String, String)> {
    let quant = spec.quantization;
    let guess_filename = format!("{}-{}.gguf", spec.id, quant);
    let bases: &[(&str, &str)] = &[
        ("huggingface", "https://huggingface.co"),
        ("hf-mirror", "https://hf-mirror.com"),
        ("modelscope", "https://www.modelscope.cn"),
    ];
    let mut out = Vec::with_capacity(bases.len());
    for (label, base) in bases {
        let url = format!("{}/{}/resolve/main/{}", base, spec.hf_repo, guess_filename);
        out.push((url, (*label).to_string()));
    }
    out
}

/// Cheap read of the cancel flag — held only while locking the mutex
/// for an instant.
fn cancelled() -> bool {
    SPEED_TEST_STATE.lock().unwrap().cancel.load(Ordering::Relaxed)
}

/// Emit a terminal `cancelled` event so the FE can immediately stop
/// animating and update the panel.
fn emit_cancelled(app: &AppHandle, m_id: &str, index: usize) {
    emit(
        app,
        SpeedTestEvent {
            kind: "cancelled".into(),
            model_id: m_id.to_string(),
            index,
            total: index,
            result: None,
            error: None,
            queue: None,
        },
    );
}
