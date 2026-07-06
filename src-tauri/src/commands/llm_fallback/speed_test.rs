//! Streaming mirror speed-test (v0.7.2).
//!
//! Returns immediately; results stream via the
//! `llm:fallback:speed-test` event channel so the UI can paint one
//! row per mirror as soon as its probe finishes.
//!
//! Event payload (camelCase on the wire):
//! ```text
//! { kind: "started" | "result" | "done",
//!   modelId, index, total,
//!   result?: SpeedTestResult,
//!   error?: string }
//! ```

use tauri::{AppHandle, Manager};

use crate::commands::llm as llm_cmd;
use crate::commands::metrics as m;
use crate::error::{AppError, AppResult};
use crate::llm::fallback;
use crate::AppState;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SpeedTestEvent {
    kind: String,
    model_id: String,
    index: usize,
    total: usize,
    result: Option<fallback::SpeedTestResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn emit(app: &AppHandle, payload: SpeedTestEvent) {
    let _ = app.emit_all("llm:fallback:speed-test", payload);
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

    // User-provided URL — single probe. Still streams via the event
    // channel so the UI has one code path.
    if let Some(u) = manual_url.as_deref() {
        if !u.is_empty() {
            let m_id = model_id.clone().unwrap_or_else(|| "manual".into());
            let u_owned = u.to_string();
            let app_c = app.clone();
            tokio::spawn(async move { run_manual(&app_c, &u_owned, m_id).await });
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

    let total = urls.len();
    let app_clone = app.clone();
    let mid = model_id.clone();
    tokio::spawn(async move { run_queue(&app_clone, mid, urls).await });
    Ok(())
}

/// Serial `started → result* → done` for a single user-provided URL.
async fn run_manual(app: &AppHandle, url: &str, m_id: String) {
    emit(
        app,
        SpeedTestEvent {
            kind: "started".into(),
            model_id: m_id.clone(),
            index: 0,
            total: 1,
            result: None,
            error: None,
        },
    );
    let r = fallback::speed_test_url(url, "manual", "download").await;
    emit(
        app,
        SpeedTestEvent {
            kind: "result".into(),
            model_id: m_id.clone(),
            index: 0,
            total: 1,
            result: Some(r),
            error: None,
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
        },
    );
}

/// Serial `started → result* → done` for a precomputed URL queue.
async fn run_queue(app: &AppHandle, m_id: String, urls: Vec<(String, String, &'static str)>) {
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
        },
    );
    for (i, (url, label, kind)) in urls.into_iter().enumerate() {
        let r = fallback::speed_test_url(&url, &label, kind).await;
        emit(
            app,
            SpeedTestEvent {
                kind: "result".into(),
                model_id: m_id.clone(),
                index: i,
                total,
                result: Some(r),
                error: None,
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
        },
    );
}

/// Build candidate `(url, label)` pairs by guessing a filename and
/// probing each well-known mirror base. Returned as plain data so the
/// caller can spawn a background task to probe them.
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
