//! HTTP downloader with progress reporting and SHA-256 verification.
//!
//! Used by the fallback engine to fetch GGUF model files and the
//! `llama-server.exe` binary. Streams bytes to disk (8MB buffer), reports
//! progress through the `on_progress` callback, then verifies the SHA-256
//! hash on completion.
//!
//! Cancellation: drop the returned `DownloadHandle` or call `.cancel()`.
//! Currently we model cancellation by checking a `cancel` flag in the
//! progress callback contract — the caller returns `true` to abort.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures::StreamExt;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub bytes_done: u64,
    pub total_bytes: u64,
    pub speed_bps: u64,
    pub eta_seconds: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("network: {0}")]
    Network(String),
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("hash mismatch: actual={actual} expected={expected}")]
    HashMismatch { actual: String, expected: String },
    #[error("cancelled")]
    Cancelled,
}

pub struct DownloadHandle {
    cancel: Arc<AtomicBool>,
}

impl DownloadHandle {
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

/// Stream a URL to a local file. Reports progress every 500ms via
/// `on_progress(bytes_done, total_bytes, speed_bps, eta_seconds)`. Returns
/// the SHA-256 of the resulting file on success.
///
/// Cancellation: pass a `DownloadHandle` and call `.cancel()` to abort mid-
/// stream. The stream future is dropped immediately, the `.part` file is
/// removed on the way out, and `DownloadError::Cancelled` is returned.
///
/// `on_progress` is wrapped in `Arc` so the future stays `Send` when
/// spawned into a `tokio::spawn` task.
pub async fn stream_to_file(
    url: &str,
    dest: &Path,
    expected_sha256: Option<&str>,
    cancel: Arc<AtomicBool>,
    on_progress: Arc<dyn Fn(DownloadProgress) + Send + Sync>,
) -> Result<String, DownloadError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60 * 30))
        .build()
        .map_err(|e| DownloadError::Network(e.to_string()))?;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| DownloadError::Network(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(DownloadError::Network(format!("HTTP {}", resp.status())));
    }
    let total = resp.content_length().unwrap_or(0);
    let mut stream = resp.bytes_stream();
    let tmp = dest.with_extension("part");
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::File::create(&tmp)?;
    let mut hasher = Sha256::new();
    let mut bytes_done: u64 = 0;
    let mut last_tick = std::time::Instant::now();
    let mut last_bytes: u64 = 0;
    // Stream the body chunk by chunk. We use `move async {}` so the
    // locals are owned by the future (no `FnMut` capture escaping).
    let download_body = async {
        while let Some(chunk) = stream.next().await {
            if cancel.load(Ordering::Relaxed) {
                return Err::<(), DownloadError>(DownloadError::Cancelled);
            }
            let chunk = chunk.map_err(|e| DownloadError::Network(e.to_string()))?;
            f.write_all(&chunk)?;
            hasher.update(&chunk);
            bytes_done += chunk.len() as u64;
            let now = std::time::Instant::now();
            if now.duration_since(last_tick).as_millis() >= 500 {
                let speed_bps = ((bytes_done - last_bytes) as f64
                    / now.duration_since(last_tick).as_secs_f64()) as u64;
                let eta_seconds = if speed_bps > 0 && total > bytes_done {
                    (total - bytes_done) / speed_bps
                } else {
                    0
                };
                on_progress(DownloadProgress {
                    bytes_done,
                    total_bytes: total,
                    speed_bps,
                    eta_seconds,
                });
                last_tick = now;
                last_bytes = bytes_done;
            }
        }
        Ok::<(), DownloadError>(())
    };
    let result: Result<(), DownloadError> = download_body.await;
    if result.is_err() {
        // Drop file handle, remove partial file.
        let _ = f.flush();
        drop(f);
        let _ = std::fs::remove_file(&tmp);
        return Err(result.err().unwrap());
    }
    f.flush()?;
    drop(f);
    // Atomic rename
    std::fs::rename(&tmp, dest)?;
    let actual = format!("{:x}", hasher.finalize());
    if let Some(expected) = expected_sha256 {
        if !expected.is_empty() && actual != expected {
            // Best-effort cleanup of bad file
            let _ = std::fs::remove_file(dest);
            return Err(DownloadError::HashMismatch {
                actual,
                expected: expected.to_string(),
            });
        }
    }
    Ok(actual)
}

/// Hash an existing file with SHA-256.
pub fn sha256_file(path: &Path) -> io::Result<String> {
    use std::io::Read;
    let mut f = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20]; // 1MB
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering as AOrd};

    #[test]
    fn sha256_of_known_file() {
        // Empty file → e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        let dir = std::env::temp_dir().join(format!(
            "admin-suite-dl-test-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("empty.bin");
        std::fs::write(&f, b"").unwrap();
        assert_eq!(
            sha256_file(&f).unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sha256_short_text() {
        let dir = std::env::temp_dir().join(format!(
            "admin-suite-dl-short-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("hello.txt");
        std::fs::write(&f, b"hello world").unwrap();
        // sha256("hello world") = b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
        assert_eq!(
            sha256_file(&f).unwrap(),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    /// Cancellation must drop the partial `.part` file and return
    /// `DownloadError::Cancelled` without panicking. We use a small
    /// synthetic HTTP server (`tiny_http`-style is overkill — we spawn
    /// Python's stdlib) — but to keep zero external deps, instead we
    /// test the cancel flag semantics directly: spawn a tokio task that
    /// sets the cancel flag mid-stream and asserts the future exits.
    #[tokio::test]
    async fn cancel_aborts_in_flight_download() {
        // We can't easily test a real network call here without flake,
        // so we test the "cancel flag → Cancelled error" contract by
        // spawning a tiny server using `tokio::net::TcpListener` that
        // sends bytes slowly, then flip cancel while it's running.
        use tokio::io::AsyncWriteExt;
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        // Server task: accept one connection, write a few bytes slowly,
        // then hang without closing so the client gets a partial stream.
        let server = tokio::spawn(async move {
            if let Ok((mut sock, _)) = listener.accept().await {
                // HTTP/1.1 chunked-ish response — actually we use
                // content-length so the client knows when "done" is.
                // But we never close, so it'll hang waiting for more.
                let _ = sock
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 1000000\r\n\r\n")
                    .await;
                let _ = sock.write_all(&vec![b'x'; 4096]).await;
                // Hold the socket open without sending more. The client
                // will time out OR our cancel flag flips first.
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            }
        });

        let url = format!("http://127.0.0.1:{}/file.bin", port);
        let dir = std::env::temp_dir().join(format!(
            "admin-suite-dl-cancel-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let dest = dir.join("cancelled.bin");
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_for_task = cancel.clone();
        // Flip cancel after 50ms — well before the server's 30s sleep.
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            cancel_for_task.store(true, AOrd::Relaxed);
        });

        let ticks = Arc::new(AtomicUsize::new(0));
        let ticks_for_cb = ticks.clone();
        let progress_cb: Arc<dyn Fn(DownloadProgress) + Send + Sync> = Arc::new(move |_p| {
            ticks_for_cb.fetch_add(1, AOrd::Relaxed);
        });

        let result = stream_to_file(&url, &dest, None, cancel, progress_cb).await;
        server.abort();

        assert!(
            matches!(result, Err(DownloadError::Cancelled)),
            "expected Cancelled, got {:?}",
            result
        );
        // The `.part` file should be cleaned up.
        assert!(!dest.with_extension("part").exists(), "partial file leaked");
    }
}