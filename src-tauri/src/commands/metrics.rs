//! IPC performance monitoring.
//!
//! A shared `MetricsRegistry` lives in `AppState`. Each command starts with one
//! line: `let _t = metrics::time(&state.metrics, "<command_name>");`. The guard
//! records elapsed time on `Drop`. If the command returns an error, the caller
//! may invoke `_t.fail(&err.to_string())` to bump the error count and pin the
//! message into `last_error`.
//!
//! Snapshots are pulled via `metrics_snapshot()` for the admin UI; they include
//! per-command `count / last_ms / avg_ms / max_ms / error_count / history_ms`
//! (rolling window of the last `HISTORY_CAP` latencies).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde::Serialize;

/// Per-command rolling-window cap. ~100 calls gives the UI enough granularity
/// to spot spikes without blowing memory; commands rarely fire that often in
/// one session, so this is mostly a safety net.
pub const HISTORY_CAP: usize = 100;

#[derive(Debug, Clone, Serialize)]
pub struct IpcMetric {
    pub command: String,
    pub count: u64,
    pub total_ms: u128,
    pub last_ms: u128,
    pub max_ms: u128,
    pub avg_ms: f64,
    pub error_count: u64,
    pub last_error: Option<String>,
    pub last_ts: i64,
    pub history_ms: Vec<u128>,
}

#[derive(Default)]
pub struct MetricsRegistry {
    inner: Mutex<HashMap<String, IpcMetric>>,
}

pub type SharedMetrics = Arc<MetricsRegistry>;

impl MetricsRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn shared() -> SharedMetrics {
        Arc::new(Self::new())
    }

    /// Sorted by total time spent (descending) — the noisest commands float
    /// to the top of the admin UI.
    pub fn snapshot(&self) -> Vec<IpcMetric> {
        let g = self.inner.lock().expect("metrics lock poisoned");
        let mut v: Vec<IpcMetric> = g.values().cloned().collect();
        v.sort_by(|a, b| b.total_ms.cmp(&a.total_ms));
        v
    }

    pub fn clear(&self) {
        self.inner.lock().expect("metrics lock poisoned").clear();
    }

    fn record(&self, name: &str, ms: u128, err: Option<String>, ts: i64) {
        let mut g = self.inner.lock().expect("metrics lock poisoned");
        let entry = g.entry(name.to_string()).or_insert_with(|| IpcMetric {
            command: name.to_string(),
            count: 0,
            total_ms: 0,
            last_ms: 0,
            max_ms: 0,
            avg_ms: 0.0,
            error_count: 0,
            last_error: None,
            last_ts: 0,
            history_ms: Vec::with_capacity(HISTORY_CAP),
        });
        entry.count += 1;
        entry.total_ms += ms;
        entry.last_ms = ms;
        if ms > entry.max_ms {
            entry.max_ms = ms;
        }
        entry.avg_ms = entry.total_ms as f64 / entry.count as f64;
        entry.last_ts = ts;
        if let Some(e) = err {
            entry.error_count += 1;
            entry.last_error = Some(e);
        }
        if entry.history_ms.len() >= HISTORY_CAP {
            entry.history_ms.remove(0);
        }
        entry.history_ms.push(ms);
    }
}

/// RAII timer guard. Records the elapsed time on drop unless `fail()` was
/// called first, which records an error string in addition.
pub struct TimerGuard {
    registry: SharedMetrics,
    name: String,
    start: Instant,
    finished: bool,
}

impl TimerGuard {
    pub fn new(registry: SharedMetrics, name: impl Into<String>) -> Self {
        Self {
            registry,
            name: name.into(),
            start: Instant::now(),
            finished: false,
        }
    }

    #[allow(dead_code)] // consumed by callers via `time()` to mark an error.
    pub fn fail(mut self, err: impl AsRef<str>) {
        self.registry.record(
            &self.name,
            self.start.elapsed().as_millis(),
            Some(err.as_ref().to_string()),
            chrono::Utc::now().timestamp(),
        );
        self.finished = true;
    }
}

impl Drop for TimerGuard {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        self.registry.record(
            &self.name,
            self.start.elapsed().as_millis(),
            None,
            chrono::Utc::now().timestamp(),
        );
    }
}

pub fn time(registry: &SharedMetrics, name: &str) -> TimerGuard {
    TimerGuard::new(registry.clone(), name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn success_is_recorded_on_drop() {
        let r = MetricsRegistry::shared();
        {
            let _g = time(&r, "demo.success");
            thread::sleep(Duration::from_millis(2));
        }
        let snap = r.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].command, "demo.success");
        assert_eq!(snap[0].count, 1);
        assert_eq!(snap[0].error_count, 0);
        assert!(snap[0].last_ms >= 2);
        assert_eq!(snap[0].last_error, None);
        assert_eq!(snap[0].history_ms.len(), 1);
    }

    #[test]
    fn fail_marks_error_and_message() {
        let r = MetricsRegistry::shared();
        time(&r, "demo.fail").fail("boom");
        let snap = r.snapshot();
        assert_eq!(snap[0].error_count, 1);
        assert_eq!(snap[0].last_error.as_deref(), Some("boom"));
    }

    #[test]
    fn clear_resets_state() {
        let r = MetricsRegistry::shared();
        time(&r, "demo.a").fail("e");
        time(&r, "demo.b");
        assert_eq!(r.snapshot().len(), 2);
        r.clear();
        assert!(r.snapshot().is_empty());
    }

    #[test]
    fn history_is_capped_at_window() {
        let r = MetricsRegistry::shared();
        for _ in 0..(HISTORY_CAP + 5) {
            time(&r, "demo.c");
        }
        let snap = r.snapshot();
        assert_eq!(snap[0].count as usize, HISTORY_CAP + 5);
        assert_eq!(snap[0].history_ms.len(), HISTORY_CAP);
    }

    #[test]
    fn max_and_avg_track_running() {
        let r = MetricsRegistry::shared();
        // Sleep a measurable amount so the millisecond clock ticks. We bind
        // the guard to a name and explicit-scope it so it cannot be dropped
        // mid-sleep (Rust's NRVO sometimes elides `_g = expr` when expr has
        // no observable side effects).
        for _ in 0..3 {
            {
                let g = time(&r, "demo.d");
                thread::sleep(Duration::from_millis(20));
                drop(g);
            }
        }
        let snap = r.snapshot();
        assert_eq!(snap[0].count, 3);
        assert!(snap[0].max_ms >= snap[0].avg_ms as u128);
        assert!(snap[0].avg_ms > 0.0, "avg_ms was {}", snap[0].avg_ms);
        assert!(snap[0].max_ms >= 1, "max_ms was {}", snap[0].max_ms);
    }
}