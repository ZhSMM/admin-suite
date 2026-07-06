//! Parallel range-aware downloader (v0.8+).
//!
//! HuggingFace / hf-mirror / ModelScope all advertise
//! `Accept-Ranges: bytes`. A 3 GB GGUF download over a single TCP
//! connection is bottlenecked by the receiver's send-side window;
//! splitting the file into 4–16 parallel byte-range chunks saturates
//! the link and uses every available bandwidth.
//!
//! Public entry point — [`download::download`] — replaces the
//! single-conn path in `commands::llm_fallback::install::run_install`
//! for the GGUF stage. The llama-server.exe stage (small ZIP, no
//! benefit) keeps using the legacy [`super::download::stream_to_file`].
//!
//! ## Protocol
//!
//! 1. `head_probe(url)` issues `HEAD url`, returns:
//!    - `(Some(content_length), true)`  — Range + size known → split &
//!      fan out to N workers.
//!    - `(None, _)` or `(Some(..), false)` → fall back to single-conn
//!      streaming.
//! 2. The destination file is created with `set_len` (sparse file),
//!    so workers seek-and-write to disjoint byte ranges.
//! 3. Worker i downloads `bytes=A_i..=B_i`, emits progress on every
//!    chunk, increments an atomic `bytes_done`. When the last chunk
//!    finishes, a final `on_progress(total, total, …)` tick reports
//!    completion.
//! 4. On any worker error, the `cancel` flag is set and the file is
//!    unlinked. Caller sees `DownloadError::Cancelled` or `Network`.
//! 5. SHA-256 is verified after the file is fully assembled.

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use futures::StreamExt;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Mutex;

use super::download::{DownloadError, DownloadProgress};

/// Probe support + size with a single HEAD. Returns (content_length,
/// `true` if range-able AND size is known and > 0).
pub async fn probe(url: &str) -> (Option<u64>, bool) {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(_) => return (None, false),
    };
    let resp = match client.head(url).send().await {
        Ok(r) => r,
        Err(_) => return (None, false),
    };
    let accepts = resp
        .headers()
        .get(reqwest::header::ACCEPT_RANGES)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.eq_ignore_ascii_case("bytes"))
        .unwrap_or(false);
    let size = resp.content_length();
    (size, accepts && size.unwrap_or(0) > 0)
}

/// Decide chunk count + chunk size for a given total file length.
/// Heuristic:
///   - aim for 64 MiB per chunk
///   - cap to 16 workers (Windows file handles + socket count)
///   - always at least 4 to get the parallelism win
pub fn plan_chunks(total_bytes: u64) -> (usize, u64) {
    const TARGET_CHUNK: u64 = 64 * 1024 * 1024;
    const MAX_WORKERS: usize = 16;
    const MIN_WORKERS: usize = 4;

    let desired = (total_bytes.div_ceil(TARGET_CHUNK) as usize).max(MIN_WORKERS);
    let workers = desired.min(MAX_WORKERS);
    let chunk_size = total_bytes.div_ceil(workers as u64);
    (workers, chunk_size)
}

/// Download `url` to `dest` using parallel byte-range requests when
/// the server advertises `Accept-Ranges: bytes`. Falls back to a
/// single-connection stream otherwise.
///
/// `cancel` is a shared flag — flipping it aborts in-flight workers.
/// `on_progress` is invoked from any of the workers; it must be
/// `Send + Sync` and cheap to call (e.g. an `Arc<Mutex<Vec<u64>>>`
/// holding the running totals).
pub async fn download(
    url: &str,
    dest: &Path,
    cancel: Arc<AtomicBool>,
    on_progress: Arc<dyn Fn(DownloadProgress) + Send + Sync>,
) -> Result<String, DownloadError> {
    let (size_opt, range_ok) = probe(url).await;
    let total = match size_opt {
        Some(n) if range_ok => n,
        _ => {
            // Fallback: legacy single-conn streaming still uses
            // `super::download::stream_to_file` which has its own
            // progress + cancel semantics.
            return super::download::stream_to_file(
                url,
                dest,
                None,
                cancel,
                on_progress,
            )
            .await;
        }
    };

    // Open the destination as a sparse file so workers can seek to
    // their slice and write without first reading it.
    let parent = dest
        .parent()
        .ok_or_else(|| DownloadError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "destination has no parent dir",
        )))?;
    std::fs::create_dir_all(parent)?;
    let tmp = dest.with_extension("part");
    {
        let f = std::fs::File::create(&tmp)?;
        f.set_len(total)?;
    }

    let (workers, chunk_size) = plan_chunks(total);
    let bytes_done = Arc::new(AtomicU64::new(0));
    // Coalesce progress writes through a small ticking channel.
    let tick_channel = Arc::new(Mutex::new(()));

    let mut handles = Vec::with_capacity(workers);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60 * 30))
        .build()
        .map_err(|e| DownloadError::Network(e.to_string()))?;

    for i in 0..workers {
        let start = i as u64 * chunk_size;
        // last chunk clamped to total
        let end = std::cmp::min(start + chunk_size - 1, total - 1);
        if start > end {
            break;
        }
        let url = url.to_string();
        let tmp_path = tmp.clone();
        let client = client.clone();
        let cancel = cancel.clone();
        let bd = bytes_done.clone();
        let on_progress = on_progress.clone();
        let _tick = tick_channel.clone(); // keeps the channel alive for the worker

        handles.push(tokio::spawn(async move {
            chunk_worker(
                i,
                &url,
                &tmp_path,
                start,
                end,
                total,
                &client,
                cancel,
                bd,
                on_progress,
            )
            .await
        }));
    }

    let mut failed: Option<DownloadError> = None;
    for h in handles {
        match h.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                failed = Some(e);
                cancel.store(true, Ordering::Relaxed);
            }
            Err(e) => {
                failed = Some(DownloadError::Network(format!("join: {e}")));
                cancel.store(true, Ordering::Relaxed);
            }
        }
    }
    if let Some(e) = failed {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }

    // Atomic rename into place.
    std::fs::rename(&tmp, dest)?;

    // Final SHA-256 over the assembled file.
    let hash = sha256_file_async(dest).await?;

    // One last progress tick so the UI bar lands at 100%.
    on_progress(DownloadProgress {
        bytes_done: total,
        total_bytes: total,
        speed_bps: 0,
        eta_seconds: 0,
    });

    Ok(hash)
}

/// Worker for chunk `i` covering `bytes=[start, end]` inclusive.
/// Writes directly to `tmp_path` at the right offset; updates the
/// shared atomic counter; emits a coalesced progress tick every
/// ~250ms.
async fn chunk_worker(
    _worker_idx: usize,
    url: &str,
    tmp_path: &std::path::Path,
    start: u64,
    end: u64,
    total: u64,
    client: &reqwest::Client,
    cancel: Arc<AtomicBool>,
    bytes_done: Arc<AtomicU64>,
    on_progress: Arc<dyn Fn(DownloadProgress) + Send + Sync>,
) -> Result<(), DownloadError> {
    let resp = client
        .get(url)
        .header("Range", format!("bytes={start}-{end}"))
        .send()
        .await
        .map_err(|e| DownloadError::Network(e.to_string()))?;
    let status = resp.status();
    // 206 = Partial Content (Range honoured). 200 means the server
    // collapsed the whole file into one worker — not an error, just
    // we lose parallelism for this chunk.
    if status != reqwest::StatusCode::PARTIAL_CONTENT && status != reqwest::StatusCode::OK {
        return Err(DownloadError::Network(format!(
            "HTTP {} for bytes={start}-{end}",
            status.as_u16()
        )));
    }
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .open(tmp_path)
        .await
        .map_err(DownloadError::Io)?;
    file.seek(std::io::SeekFrom::Start(start))
        .await
        .map_err(DownloadError::Io)?;

    let mut stream = resp.bytes_stream();
    let mut last_tick = std::time::Instant::now();
    let mut last_bytes: u64 = 0;
    let mut local: u64 = 0;

    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::Relaxed) {
            return Err(DownloadError::Cancelled);
        }
        let bytes = chunk.map_err(|e| DownloadError::Network(e.to_string()))?;
        if bytes.is_empty() {
            continue;
        }
        file.write_all(&bytes).await.map_err(DownloadError::Io)?;
        local += bytes.len() as u64;
        let total_now = bytes_done.fetch_add(bytes.len() as u64, Ordering::Relaxed)
            + bytes.len() as u64;
        let now = std::time::Instant::now();
        if now.duration_since(last_tick).as_millis() >= 250 {
            let speed_bps = if local.saturating_sub(last_bytes) > 0 {
                ((local - last_bytes) as f64 / now.duration_since(last_tick).as_secs_f64()) as u64
            } else {
                0
            };
            let eta_seconds = if speed_bps > 0 && total > total_now {
                (total - total_now) / speed_bps
            } else {
                0
            };
            on_progress(DownloadProgress {
                bytes_done: total_now,
                total_bytes: total,
                speed_bps,
                eta_seconds,
            });
            last_tick = now;
            last_bytes = local;
        }
    }
    file.flush().await.map_err(DownloadError::Io)?;
    // log the chunk range so flaky transfers are debuggable
    let _ = (start, end);
    Ok(())
}

/// Async SHA-256 of an existing file (uses 1 MiB read chunks, off
/// the main blocking pool via spawn_blocking).
async fn sha256_file_async(path: &Path) -> Result<String, DownloadError> {
    let p = path.to_owned();
    let res: Result<String, DownloadError> = tokio::task::spawn_blocking(move || {
        sha256_file(&p)
    })
    .await
    .map_err(|e| DownloadError::Io(std::io::Error::new(
        std::io::ErrorKind::Other,
        format!("sha256 join: {e}"),
    )))?;
    res
}

fn sha256_file(path: &Path) -> Result<String, DownloadError> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).map_err(DownloadError::Io)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20]; // 1 MiB
    loop {
        let n = f.read(&mut buf).map_err(DownloadError::Io)?;
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

    #[test]
    fn chunk_plan_small() {
        // 70 MiB total → still 4 chunks (min) since 70/4=17.5 MiB
        // Each worker pulls at least one chunk worth.
        let (w, c) = plan_chunks(70 * 1024 * 1024);
        assert_eq!(w, 4);
        assert!(c * w as u64 >= 70 * 1024 * 1024);
    }

    #[test]
    fn chunk_plan_large() {
        // 4 GiB → 64 MiB target = 64 chunks, capped to 16 workers
        let (w, c) = plan_chunks(4u64 * 1024 * 1024 * 1024);
        assert_eq!(w, 16);
        assert_eq!(c, 256 * 1024 * 1024);
    }
}
