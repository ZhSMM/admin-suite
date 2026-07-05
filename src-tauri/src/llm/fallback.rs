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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

pub mod download;

pub use download::{DownloadError, DownloadHandle, DownloadProgress};

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
        // Try HF mirror first (often faster than HF), then ModelScope (works
        // when both HF and its mirror are blocked, e.g. in mainland China).
        mirror_urls: &[
            "https://hf-mirror.com/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf",
            "https://www.modelscope.cn/models/qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf",
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
            "https://www.modelscope.cn/models/LLM-Research/Llama-3.2-3B-Instruct-GGUF/resolve/main/Llama-3.2-3B-Instruct-Q4_K_M.gguf",
        ],
        min_ram_gb: 6,
    },
];

pub fn find_model(id: &str) -> Option<&'static FallbackModel> {
    MODELS.iter().find(|m| m.id == id)
}

/// llama-server version we pin to. Bump these when upgrading the runtime.
/// Note: the GitHub repo moved from `ggerganov/llama.cpp` to
/// `ggml-org/llama.cpp` around late 2025 — make sure URLs use the new path.
pub const LLAMA_SERVER_VERSION: &str = "b9873";
/// Vulkan build is the best default: ~30 MB, runs on any GPU vendor
/// (NVIDIA / AMD / Intel), and still CPU-fallbacks when no GPU is present.
/// If the user needs NVIDIA CUDA specifically, switch the URL at runtime.
pub const LLAMA_SERVER_URL: &str =
    "https://github.com/ggml-org/llama.cpp/releases/download/b9873/llama-b9873-bin-win-vulkan-x64.zip";
/// Fallback if vulkan build fails: pure CPU build (16 MB, no GPU).
pub const LLAMA_SERVER_URL_CPU: &str =
    "https://github.com/ggml-org/llama.cpp/releases/download/b9873/llama-b9873-bin-win-cpu-x64.zip";

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
    /// Active download cancellation flag (None when nothing is downloading).
    download_cancel: Option<Arc<AtomicBool>>,
    /// Which stage is currently being downloaded ("model" or "server"), for
    /// UI progress display.
    download_stage: Option<String>,
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
                download_cancel: None,
                download_stage: None,
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

    // -----------------------------------------------------------------
    // v0.6.2 — One-click local install
    // -----------------------------------------------------------------

    /// Returns the cancel handle for the in-flight download (if any), and
    /// marks it cancelled. The download future will observe the flag on its
    /// next chunk and exit with `DownloadError::Cancelled`.
    pub fn cancel_download(&self) -> bool {
        let mut g = self.inner.lock().unwrap();
        if let Some(flag) = g.download_cancel.take() {
            flag.store(true, Ordering::Relaxed);
            g.download_stage = None;
            let _ = g.state.save(&g.data_dir.join("llm"));
            true
        } else {
            false
        }
    }

    /// True iff a download is currently in flight.
    pub fn is_downloading(&self) -> bool {
        self.inner.lock().unwrap().download_cancel.is_some()
    }

    /// Which stage is currently being downloaded ("model" | "server" | None).
    pub fn current_download_stage(&self) -> Option<String> {
        self.inner.lock().unwrap().download_stage.clone()
    }

    /// Allocate a fresh cancel handle and install it in the manager. Returns
    /// the handle + a guard RAII type that clears the handle on drop.
    pub fn begin_download(&self, stage: &str) -> DownloadGuard {
        let mut g = self.inner.lock().unwrap();
        // Cancel anything still in flight (last-resort cleanup).
        if let Some(prev) = g.download_cancel.take() {
            prev.store(true, Ordering::Relaxed);
        }
        let flag = Arc::new(AtomicBool::new(false));
        g.download_cancel = Some(flag.clone());
        g.download_stage = Some(stage.to_string());
        let _ = g.state.save(&g.data_dir.join("llm"));
        DownloadGuard {
            manager: self.clone(),
            flag,
        }
    }

    fn clear_download(&self) {
        let mut g = self.inner.lock().unwrap();
        g.download_cancel = None;
        g.download_stage = None;
        let _ = g.state.save(&g.data_dir.join("llm"));
    }

    /// Path where the model GGUF file lives (or will live after download).
    pub fn model_file_path(&self, model_id: &str) -> PathBuf {
        self.model_dir().join(format!("{}.gguf", model_id))
    }

    /// Path where the llama-server binary lives (or will live after download).
    pub fn llama_server_binary_path(&self) -> PathBuf {
        self.bin_dir().join("llama-server.exe")
    }

    /// Path where the llama-server zip is cached during download.
    fn llama_server_zip_path(&self) -> PathBuf {
        self.bin_dir().join("llama-server.zip")
    }

    /// Download (if missing) + extract `llama-server.exe`. Reports progress
    /// via `on_progress`; cancels if the guard's `cancel()` is called.
    ///
    /// `on_progress` is wrapped in an `Arc` so the future is `Send` when
    /// invoked from a `tokio::spawn`'d task.
    pub async fn ensure_llama_server(
        &self,
        guard: &DownloadGuard,
        on_progress: Arc<dyn Fn(DownloadProgress) + Send + Sync>,
    ) -> Result<PathBuf, DownloadError> {
        let dest = self.llama_server_binary_path();
        if dest.exists() {
            // Already extracted — nothing to do.
            return Ok(dest);
        }
        let zip_dest = self.llama_server_zip_path();
        // Try URLs in order: vulkan first (best default), CPU as fallback.
        let urls: [&str; 2] = [LLAMA_SERVER_URL, LLAMA_SERVER_URL_CPU];
        let mut last_err: Option<DownloadError> = None;
        for url in urls.iter() {
            match download::stream_to_file(
                url,
                &zip_dest,
                None, // upstream release zip has no fixed sha256 we ship
                guard.flag.clone(),
                on_progress.clone(),
            )
            .await
            {
                Ok(_) => {
                    last_err = None;
                    break;
                }
                Err(e) => {
                    let _ = std::fs::remove_file(&zip_dest);
                    last_err = Some(e);
                }
            }
        }
        if let Some(e) = last_err {
            return Err(e);
        }
        // Extract the zip.
        let bin_dir = self.bin_dir();
        std::fs::create_dir_all(&bin_dir)?;
        let f = std::fs::File::open(&zip_dest)?;
        let mut archive = zip::ZipArchive::new(f)
            .map_err(|e| DownloadError::Io(io::Error::new(io::ErrorKind::Other, e.to_string())))?;
        // b3900 ships llama-server.exe at the root of the archive.
        let target_name = "llama-server.exe";
        let mut found = false;
        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .map_err(|e| DownloadError::Io(io::Error::new(io::ErrorKind::Other, e.to_string())))?;
            let name = entry.name().to_string();
            let base = name.rsplit('/').next().unwrap_or(&name);
            if base == target_name {
                let mut out = std::fs::File::create(&dest)?;
                std::io::copy(&mut entry, &mut out)?;
                found = true;
                break;
            }
        }
        if !found {
            let _ = std::fs::remove_file(&zip_dest);
            return Err(DownloadError::Io(io::Error::new(
                io::ErrorKind::Other,
                format!("llama-server.exe not found in zip"),
            )));
        }
        let _ = std::fs::remove_file(&zip_dest);
        Ok(dest)
    }

    /// Download the GGUF model file for `model_id`. Reports progress via
    /// `on_progress`; cancels if the guard's `cancel()` is called.
    ///
    /// `on_progress` is wrapped in an `Arc` so the future is `Send` when
    /// invoked from a `tokio::spawn`'d task.
    pub async fn download_model(
        &self,
        model_id: &str,
        guard: &DownloadGuard,
        on_progress: Arc<dyn Fn(DownloadProgress) + Send + Sync>,
    ) -> Result<PathBuf, DownloadError> {
        let model = find_model(model_id)
            .ok_or_else(|| DownloadError::Io(io::Error::new(
                io::ErrorKind::NotFound,
                format!("unknown fallback model: {}", model_id),
            )))?;
        let dest = self.model_file_path(model_id);
        let urls = std::iter::once(model.primary_url).chain(model.mirror_urls.iter().copied());
        let mut last_err: Option<DownloadError> = None;
        for url in urls {
            match download::stream_to_file(
                url,
                &dest,
                if model.sha256.is_empty() { None } else { Some(model.sha256) },
                guard.flag.clone(),
                on_progress.clone(),
            )
            .await
            {
                Ok(_) => {
                    last_err = None;
                    break;
                }
                Err(e) => {
                    let _ = std::fs::remove_file(&dest);
                    last_err = Some(e);
                }
            }
        }
        if let Some(e) = last_err {
            return Err(e);
        }
        Ok(dest)
    }

    /// Delete the downloaded model file (and any cached server zip) and
    /// reset the phase to NotDownloaded.
    pub fn remove_model(&self) -> io::Result<()> {
        let mut g = self.inner.lock().unwrap();
        // Kill the server first so the file isn't in use.
        if let Some(mut child) = g.server_child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        g.server_started_at = None;
        g.state.llama_server_path = None;
        g.state.llama_server_port = None;
        g.state.last_started_unix_ms = None;
        // Remove all model files.
        let dir = g.data_dir.join("llm").join("models");
        if dir.exists() {
            for entry in std::fs::read_dir(&dir)? {
                let entry = entry?;
                if entry.file_type()?.is_file() {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
        g.state.model_path = None;
        g.state.phase = Phase::NotDownloaded;
        g.state.last_error = None;
        g.state.save(&g.data_dir.join("llm"))
    }

    /// Total size of the on-disk model files (sum of GGUF sizes). 0 if none.
    pub fn model_files_size_bytes(&self) -> u64 {
        let dir = self.model_dir();
        if !dir.exists() {
            return 0;
        }
        std::fs::read_dir(&dir)
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .filter_map(|e| e.metadata().ok())
                    .map(|m| m.len())
                    .sum()
            })
            .unwrap_or(0)
    }
}

/// RAII handle for an in-flight download. Dropping without calling
/// `cancel()` is fine — the flag stays false and the download completes.
/// Calling `cancel()` flips the flag; the download future then exits
/// with `DownloadError::Cancelled` on its next chunk.
pub struct DownloadGuard {
    manager: FallbackManager,
    flag: Arc<AtomicBool>,
}

impl DownloadGuard {
    /// Flip the cancel flag. Does NOT remove the guard from the manager —
    /// that happens when the download future exits and calls
    /// `manager.clear_download()`.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::Relaxed);
    }
}

impl Drop for DownloadGuard {
    fn drop(&mut self) {
        // Clear the manager's reference so subsequent `is_downloading()`
        // returns false. The flag itself may still be held by a stranded
        // download future, but it'll be observed and the future will exit.
        self.manager.clear_download();
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

    // --- v0.6.2 additions ---

    #[test]
    fn begin_download_records_stage_and_clears_on_drop() {
        let mgr = FallbackManager::new(tmp_dir());
        assert!(!mgr.is_downloading());
        let guard = mgr.begin_download("model");
        assert!(mgr.is_downloading());
        assert_eq!(mgr.current_download_stage().as_deref(), Some("model"));
        drop(guard);
        assert!(!mgr.is_downloading());
        assert!(mgr.current_download_stage().is_none());
    }

    #[test]
    fn cancel_download_flips_flag_and_clears_state() {
        let mgr = FallbackManager::new(tmp_dir());
        let guard = mgr.begin_download("server");
        assert!(mgr.cancel_download());
        // Guard flag is shared (Arc), so it sees the cancel.
        // After guard drop, manager slot should be clear.
        drop(guard);
        assert!(!mgr.is_downloading());
    }

    #[test]
    fn cancel_download_returns_false_when_idle() {
        let mgr = FallbackManager::new(tmp_dir());
        assert!(!mgr.cancel_download());
    }

    #[test]
    fn begin_download_replaces_prior_guard() {
        let mgr = FallbackManager::new(tmp_dir());
        let g1 = mgr.begin_download("model");
        let g2 = mgr.begin_download("server");
        // g1's flag should be flipped (cancelled).
        // We can observe this via the AtomicBool directly via a small
        // trick: the guard exposes nothing public, but we know cancel
        // semantics from `cancel_download` — easier to assert stage.
        assert_eq!(mgr.current_download_stage().as_deref(), Some("server"));
        drop(g2);
        drop(g1);
        assert!(!mgr.is_downloading());
    }

    #[test]
    fn model_file_path_uses_id() {
        let mgr = FallbackManager::new(tmp_dir());
        let p = mgr.model_file_path("qwen2.5-1.5b-instruct-q4km");
        assert!(p.ends_with("qwen2.5-1.5b-instruct-q4km.gguf"));
    }

    #[test]
    fn remove_model_resets_phase() {
        let dir = tmp_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let mgr = FallbackManager::new(dir.clone());
        // Pretend we already downloaded a model.
        let models = mgr.model_dir();
        std::fs::create_dir_all(&models).unwrap();
        std::fs::write(models.join("qwen2.5-1.5b-instruct-q4km.gguf"), b"fake").unwrap();
        let _ = mgr.update_phase(Phase::Ready {
            path: models.join("qwen2.5-1.5b-instruct-q4km.gguf"),
            downloaded_at_unix_ms: 1,
        });
        assert!(mgr.model_files_size_bytes() > 0);
        mgr.remove_model().unwrap();
        assert_eq!(mgr.model_files_size_bytes(), 0);
        assert!(matches!(mgr.state().phase, Phase::NotDownloaded));
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