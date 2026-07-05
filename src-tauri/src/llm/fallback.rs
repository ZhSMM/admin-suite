//! Local fallback engine.
//!
//! When no cloud provider is reachable (or none is configured), we spawn a
//! locally-running inference server (`llama-server` from llama.cpp) and
//! load a small GGUF model. This gives "AI works offline" out of the box.
//!
//! Components:
//!   - `MODELS`       — static registry of fallback candidates (URLs, hashes, sizes)
//!   - `FallbackState` — JSON-backed on-disk state machine (downloaded/verifying/ready/...)
//!   - `ServerManager` — spawn / health-check / idle-kill of `llama-server.exe`
//!
//! The actual download (HTTP + SHA-256) and the server binary download live
//! in `download.rs` (we'd split this file if it grew past a few hundred lines).

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

pub mod download;

/// Phase of the fallback model's lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    #[default]
    NotDownloaded,
    Downloading { bytes_done: u64, total_bytes: u64, speed_bps: u64, eta_seconds: u64 },
    Verifying,
    Ready { path: PathBuf, downloaded_at_unix_ms: i64 },
    Error { message: String },
    HashMismatch { actual: String, expected: String },
}

/// A curated fallback model candidate.
#[derive(Debug, Clone)]
pub struct FallbackModel {
    pub id: &'static str,
    pub display_name: &'static str,
    pub family: &'static str,
    pub parameter_count: &'static str,
    pub quantization: &'static str,
    pub size_bytes: u64,
    /// Empty string means "no checksum recorded yet" — verification is then skipped.
    pub sha256: &'static str,
    pub context_window: u32,
    pub primary_url: &'static str,
    pub mirror_urls: &'static [&'static str],
    pub min_ram_gb: u32,
}

pub static MODELS: &[FallbackModel] = &[
    FallbackModel {
        id: "qwen2.5-1.5b-instruct-q4km",
        display_name: "Qwen2.5 1.5B Instruct (Q4_K_M)",
        family: "qwen2.5",
        parameter_count: "1.5B",
        quantization: "Q4_K_M",
        size_bytes: 1_100_000_000,
        sha256: "",
        context_window: 8192,
        primary_url: "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf",
        mirror_urls: &[
            "https://hf-mirror.com/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf",
        ],
        min_ram_gb: 4,
    },
    FallbackModel {
        id: "llama-3.2-3b-instruct-q4km",
        display_name: "Llama 3.2 3B Instruct (Q4_K_M)",
        family: "llama3.2",
        parameter_count: "3B",
        quantization: "Q4_K_M",
        size_bytes: 2_050_000_000,
        sha256: "",
        context_window: 8192,
        primary_url: "https://huggingface.co/bartowski/Llama-3.2-3B-Instruct-GGUF/resolve/main/Llama-3.2-3B-Instruct-Q4_K_M.gguf",
        mirror_urls: &[
            "https://hf-mirror.com/bartowski/Llama-3.2-3B-Instruct-GGUF/resolve/main/Llama-3.2-3B-Instruct-Q4_K_M.gguf",
        ],
        min_ram_gb: 6,
    },
];

pub fn find_model(id: &str) -> Option<&'static FallbackModel> {
    MODELS.iter().find(|m| m.id == id)
}

pub const LLAMA_SERVER_VERSION: &str = "b3900";
pub const LLAMA_SERVER_URL: &str =
    "https://github.com/ggerganov/llama.cpp/releases/download/b3900/llama-b3900-bin-win-cuda-cu12.4-x64.zip";

/// On-disk state — written to `<data_dir>/llm/fallback_state.json`.
///
/// Concurrency: state file is small (a few hundred bytes); we serialize all
/// reads/writes through the manager's Mutex, no DB involvement. This keeps
/// progress ticks cheap and crash-safe.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FallbackState {
    pub enabled: bool,
    pub selected_model_id: Option<String>,
    pub phase: Phase,
    pub model_path: Option<PathBuf>,
    pub llama_server_path: Option<PathBuf>,
    pub llama_server_port: Option<u16>,
    pub last_error: Option<String>,
    pub last_started_unix_ms: Option<i64>,
}

impl FallbackState {
    pub fn load_or_default(dir: &Path) -> Self {
        let path = dir.join("fallback_state.json");
        match std::fs::read_to_string(&path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, dir: &Path) -> io::Result<()> {
        std::fs::create_dir_all(dir)?;
        let body = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        let tmp = dir.join("fallback_state.json.tmp");
        std::fs::write(&tmp, &body)?;
        std::fs::rename(&tmp, dir.join("fallback_state.json"))?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct FallbackManager {
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    state: FallbackState,
    data_dir: PathBuf,
    server_child: Option<Child>,
    server_started_at: Option<Instant>,
    /// Map of model_id -> last error string for UI.
    error_log: HashMap<String, String>,
}

impl FallbackManager {
    pub fn new(data_dir: PathBuf) -> Self {
        let state = FallbackState::load_or_default(&data_dir.join("llm"));
        Self {
            inner: Arc::new(Mutex::new(Inner {
                state,
                data_dir,
                server_child: None,
                server_started_at: None,
                error_log: HashMap::new(),
            })),
        }
    }

    pub fn data_dir(&self) -> PathBuf {
        self.inner.lock().unwrap().data_dir.clone()
    }

    pub fn state(&self) -> FallbackState {
        self.inner.lock().unwrap().state.clone()
    }

    pub fn llm_dir(&self) -> PathBuf {
        self.inner.lock().unwrap().data_dir.join("llm")
    }

    pub fn model_dir(&self) -> PathBuf {
        self.llm_dir().join("models")
    }

    pub fn bin_dir(&self) -> PathBuf {
        self.llm_dir().join("bin")
    }

    pub fn set_selected_model(&self, id: Option<String>) -> io::Result<()> {
        let mut g = self.inner.lock().unwrap();
        g.state.selected_model_id = id;
        g.state.save(&g.data_dir.join("llm"))
    }

    pub fn set_enabled(&self, enabled: bool) -> io::Result<()> {
        let mut g = self.inner.lock().unwrap();
        g.state.enabled = enabled;
        g.state.save(&g.data_dir.join("llm"))
    }

    pub fn update_phase(&self, phase: Phase) -> io::Result<()> {
        let mut g = self.inner.lock().unwrap();
        g.state.phase = phase;
        g.state.save(&g.data_dir.join("llm"))
    }

    pub fn record_error(&self, msg: String) -> io::Result<()> {
        let mut g = self.inner.lock().unwrap();
        g.state.last_error = Some(msg.clone());
        g.error_log.insert("fallback".into(), msg);
        g.state.save(&g.data_dir.join("llm"))
    }

    pub fn ensure_server(&self, port: u16) -> Result<PathBuf, String> {
        let mut g = self.inner.lock().unwrap();

        // Already running on the right port?
        let already_running = g
            .server_child
            .as_mut()
            .map(|c| matches!(c.try_wait(), Ok(None)))
            .unwrap_or(false);
        let on_right_port = g.state.llama_server_port == Some(port);
        if already_running && on_right_port {
            let path = llama_server_path(&g.data_dir)
                .map_err(|e| format!("llama_server_path: {e}"))?;
            return Ok(path);
        }

        // Try to spawn.
        let exe = llama_server_path(&g.data_dir).map_err(|e| format!("llama_server_path: {e}"))?;
        if !exe.exists() {
            return Err(format!(
                "llama-server.exe not found at {} — download it first (Settings → AI)",
                exe.display()
            ));
        }

        let model = g
            .state
            .selected_model_id
            .as_ref()
            .and_then(|id| find_model(id))
            .ok_or_else(|| "no fallback model selected".to_string())?;
        let model_path = g.data_dir
            .join("llm")
            .join("models")
            .join(format!("{}.gguf", model.id));
        if !model_path.exists() {
            return Err(format!(
                "fallback model file missing: {}",
                model_path.display()
            ));
        }

        let child = Command::new(&exe)
            .arg("--model").arg(&model_path)
            .arg("--port").arg(port.to_string())
            .arg("--host").arg("127.0.0.1")
            .arg("--ctx-size").arg(model.context_window.to_string())
            .arg("-ngl").arg("20")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("spawn llama-server: {e}"))?;

        g.server_child = Some(child);
        g.server_started_at = Some(Instant::now());
        g.state.llama_server_port = Some(port);
        g.state.llama_server_path = Some(exe.clone());
        g.state.last_started_unix_ms = Some(unix_ms());
        let _ = g.state.save(&g.data_dir.join("llm"));
        Ok(exe)
    }

    pub fn kill_server(&self) {
        let mut g = self.inner.lock().unwrap();
        if let Some(mut child) = g.server_child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        g.server_started_at = None;
    }

    pub fn local_base_url(&self) -> Option<String> {
        let g = self.inner.lock().unwrap();
        g.state.llama_server_port.map(|p| format!("http://127.0.0.1:{}/v1", p))
    }

    pub fn idle_seconds(&self) -> Option<u64> {
        self.inner
            .lock()
            .unwrap()
            .server_started_at
            .map(|t| t.elapsed().as_secs())
    }
}

fn llama_server_path(data_dir: &Path) -> io::Result<PathBuf> {
    let path = data_dir.join("llm").join("bin").join("llama-server.exe");
    Ok(path)
}

pub fn unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn tmp_dir() -> PathBuf {
        env::temp_dir().join(format!(
            "admin-suite-fb-test-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ))
    }

    #[test]
    fn state_round_trips() {
        let dir = tmp_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let mut s = FallbackState::default();
        s.enabled = true;
        s.selected_model_id = Some("qwen2.5-1.5b-instruct-q4km".into());
        s.phase = Phase::Ready {
            path: dir.join("models").join("qwen.gguf"),
            downloaded_at_unix_ms: 1234,
        };
        s.save(&dir).unwrap();
        let loaded = FallbackState::load_or_default(&dir);
        assert_eq!(loaded.enabled, s.enabled);
        assert_eq!(loaded.selected_model_id, s.selected_model_id);
        match loaded.phase {
            Phase::Ready { path, downloaded_at_unix_ms } => {
                assert_eq!(path, s.phase.phase_path());
                assert_eq!(downloaded_at_unix_ms, 1234);
            }
            _ => panic!("phase mismatch"),
        }
    }

    #[test]
    fn find_model_by_id() {
        assert!(find_model("qwen2.5-1.5b-instruct-q4km").is_some());
        assert!(find_model("nonexistent").is_none());
    }

    #[test]
    fn ensure_server_returns_error_when_binary_missing() {
        let dir = tmp_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let mgr = FallbackManager::new(dir);
        mgr.set_selected_model(Some("qwen2.5-1.5b-instruct-q4km".into())).unwrap();
        let res = mgr.ensure_server(39100);
        assert!(res.is_err());
    }

    #[test]
    fn kill_server_is_idempotent() {
        let mgr = FallbackManager::new(tmp_dir());
        mgr.kill_server();
        mgr.kill_server();
    }
}

// Helper accessor so we can compare the path inside the Phase enum.
impl Phase {
    pub fn phase_path(&self) -> PathBuf {
        match self {
            Phase::Ready { path, .. } => path.clone(),
            _ => PathBuf::new(),
        }
    }
}