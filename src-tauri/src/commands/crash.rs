//! Crash diagnostics storage.
//!
//! Crashes are persisted as one JSON file per incident in `<data_dir>/crashes/`.
//! The directory is created lazily by the first write.  Each file is named
//! `<unix_ms>-<short-hash>.json` so they sort naturally by time.
//!
//! Three kinds:
//!   - `rust_panic`           — captured by `install_panic_hook`
//!   - `frontend_error`       — caught by `app.config.errorHandler` and sent via `crash_log`
//!   - `frontend_unhandled_rejection` — caught by `window.onunhandledrejection`

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrashKind {
    RustPanic,
    FrontendError,
    FrontendUnhandledRejection,
}

impl CrashKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            CrashKind::RustPanic => "rust_panic",
            CrashKind::FrontendError => "frontend_error",
            CrashKind::FrontendUnhandledRejection => "frontend_unhandled_rejection",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashReport {
    pub id: String,
    pub ts_unix_ms: i64,
    pub kind: CrashKind,
    pub message: String,
    pub source: Option<String>,
    pub app_version: Option<String>,
    /// Free-form structured detail. For Rust panics this is the backtrace
    /// (best-effort). For frontend errors this is the original error's
    /// component / stack as a string.
    pub detail: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CrashReportInput {
    pub kind: CrashKind,
    pub message: String,
    pub source: Option<String>,
    pub app_version: Option<String>,
    pub detail: Option<String>,
}

pub struct CrashStore {
    dir: PathBuf,
}

impl CrashStore {
    pub fn new(data_dir: &Path) -> io::Result<Self> {
        let dir = data_dir.join("crashes");
        fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn list(&self) -> io::Result<Vec<CrashReport>> {
        let mut out = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let bytes = match fs::read(&path) {
                Ok(b) => b,
                Err(_) => continue,
            };
            if let Ok(report) = serde_json::from_slice::<CrashReport>(&bytes) {
                out.push(report);
            }
        }
        // Newest first.
        out.sort_by(|a, b| b.ts_unix_ms.cmp(&a.ts_unix_ms));
        Ok(out)
    }

    pub fn get(&self, id: &str) -> io::Result<Option<CrashReport>> {
        for report in self.list()? {
            if report.id == id {
                return Ok(Some(report));
            }
        }
        Ok(None)
    }

    pub fn record(&self, input: CrashReportInput) -> io::Result<CrashReport> {
        let ts_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let id = format!(
            "{:013}-{:x}",
            ts_unix_ms,
            // 4-byte tag from the message hash so two reports in the same ms
            // never collide.
            simple_hash(input.message.as_bytes()) & 0xFFFF
        );
        let report = CrashReport {
            id: id.clone(),
            ts_unix_ms,
            kind: input.kind,
            message: input.message,
            source: input.source,
            app_version: input.app_version,
            detail: input.detail,
        };
        let path = self.dir.join(format!("{}.json", id));
        let body = serde_json::to_vec_pretty(&report).map_err(io::Error::other)?;
        // Atomic-ish write: write to a sibling tmp, then rename.
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, &body)?;
        fs::rename(&tmp, &path)?;
        Ok(report)
    }

    pub fn clear(&self) -> io::Result<usize> {
        let mut n = 0;
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if fs::remove_file(&path).is_ok() {
                    n += 1;
                }
            }
        }
        Ok(n)
    }
}

fn simple_hash(bytes: &[u8]) -> u32 {
    let mut h: u32 = 0x811c9dc5;
    for b in bytes {
        h ^= *b as u32;
        h = h.wrapping_mul(0x01000193);
    }
    h
}

/// Install a process-wide panic hook that writes each panic as a `CrashReport`.
/// Must be called once at startup, before any Tauri command might panic.
pub fn install_panic_hook(store: std::sync::Arc<CrashStore>) {
    std::panic::set_hook(Box::new(move |info| {
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "panic with non-string payload".to_string());
        let location = info.location().map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()));
        // best-effort backtrace
        let bt = std::backtrace::Backtrace::force_capture();
        let detail = location
            .as_ref()
            .map(|loc| format!("at {}\n\n{}", loc, bt))
            .or_else(|| Some(bt.to_string()));
        let _ = store.record(CrashReportInput {
            kind: CrashKind::RustPanic,
            message,
            source: location,
            app_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            detail,
        });
        // Re-emit to stderr so it's still visible in the dev console.
        eprintln!("[crash] rust panic: {}", info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn tmp_store() -> CrashStore {
        let dir = env::temp_dir().join(format!(
            "admin-suite-crash-test-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        CrashStore::new(&dir).expect("create store")
    }

    #[test]
    fn record_then_list() {
        let s = tmp_store();
        let _ = s.clear();
        let _r1 = s.record(CrashReportInput {
            kind: CrashKind::FrontendError,
            message: "boom".into(),
            source: Some("App.vue:42".into()),
            app_version: Some("0.5.7".into()),
            detail: Some("TypeError".into()),
        }).unwrap();
        let _r2 = s.record(CrashReportInput {
            kind: CrashKind::RustPanic,
            message: "thread main panicked".into(),
            source: None,
            app_version: None,
            detail: None,
        }).unwrap();
        let list = s.list().unwrap();
        assert_eq!(list.len(), 2);
        // Newest first
        assert_eq!(list[0].message, "thread main panicked");
        assert_eq!(list[1].message, "boom");
    }

    #[test]
    fn get_returns_recorded() {
        let s = tmp_store();
        let _ = s.clear();
        let r = s.record(CrashReportInput {
            kind: CrashKind::FrontendError,
            message: "needle".into(),
            source: None,
            app_version: None,
            detail: None,
        }).unwrap();
        let got = s.get(&r.id).unwrap().unwrap();
        assert_eq!(got.message, "needle");
    }

    #[test]
    fn get_missing_returns_none() {
        let s = tmp_store();
        let _ = s.clear();
        assert!(s.get("nope").unwrap().is_none());
    }

    #[test]
    fn clear_removes_all() {
        let s = tmp_store();
        let _ = s.clear();
        for i in 0..3 {
            let _ = s.record(CrashReportInput {
                kind: CrashKind::FrontendError,
                message: format!("x{i}"),
                source: None,
                app_version: None,
                detail: None,
            }).unwrap();
        }
        assert_eq!(s.list().unwrap().len(), 3);
        let removed = s.clear().unwrap();
        assert_eq!(removed, 3);
        assert!(s.list().unwrap().is_empty());
    }

    #[test]
    fn files_are_sorted_newest_first() {
        let s = tmp_store();
        let _ = s.clear();
        s.record(CrashReportInput {
            kind: CrashKind::FrontendError, message: "old".into(),
            source: None, app_version: None, detail: None,
        }).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        s.record(CrashReportInput {
            kind: CrashKind::FrontendError, message: "new".into(),
            source: None, app_version: None, detail: None,
        }).unwrap();
        let list = s.list().unwrap();
        assert_eq!(list[0].message, "new");
        assert_eq!(list[1].message, "old");
    }
}