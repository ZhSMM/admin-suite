//! Event payloads + `emit_*` helpers shared by install/server streams.
//!
//! Kept tiny on purpose: every other module in `commands/llm_fallback`
//! imports `emit_progress` / `emit_done` from here rather than
//! duplicating the `app.emit_all("llm:fallback:...")` calls.

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::llm::fallback::DownloadProgress;

/// Event payload emitted every 250ms during install download progress.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FallbackProgressEvent {
    /// "model" | "server"
    pub stage: String,
    pub bytes_done: u64,
    pub total_bytes: u64,
    pub speed_bps: u64,
    pub eta_seconds: u64,
    /// Convenience mirror of `stage` for clients that flatten payloads.
    pub current_stage: String,
    /// User-facing model id used by the FE to filter events.
    pub model_id: String,
}

/// Emit a progress tick. Tauri's event-name validator rejects `.` and
/// other characters, but the user-picked `model_id` may legitimately
/// contain them (e.g. `qwen2.5-1.5b-instruct-q4km`), so we encode it
/// in the payload instead of in the event name.
pub fn emit_progress(app: &AppHandle, model_id: &str, stage: &str, p: &DownloadProgress) {
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

/// Emit a terminal event (success or failure) for an install run.
pub fn emit_done(app: &AppHandle, model_id: &str, success: bool, error: &str) {
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
