use crate::error::AppResult;
use parking_lot::Mutex;
use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub mod migrate;

/// Process-wide SQLite handle.
///
/// Single-connection is intentional: SQLite serialises writes internally, and Tauri commands
/// execute on a tokio runtime where we'd otherwise pay connection-pool overhead with no gain.
/// Reads are still fast (SQLite WAL) and our command set is admin-scale (low concurrency).
pub struct Db {
    conn: Mutex<Connection>,
    path: PathBuf,
}

impl Db {
    pub fn open<P: AsRef<Path>>(path: P) -> AppResult<Arc<Self>> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )?;
        // Pragmas tuned for an admin desktop app: WAL for concurrent reads,
        // foreign keys ON, moderate cache.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "temp_store", "MEMORY")?;
        conn.pragma_update(None, "cache_size", -64_000)?; // 64 MB
        Ok(Arc::new(Self {
            conn: Mutex::new(conn),
            path,
        }))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Execute a closure with the connection. Caller must NOT hold the lock across
    /// long-running awaits — but our API is sync so that's fine.
    pub fn with_conn<R>(&self, f: impl FnOnce(&mut Connection) -> AppResult<R>) -> AppResult<R> {
        let mut guard = self.conn.lock();
        f(&mut guard)
    }

    /// Convenience: run many statements inside a transaction.
    pub fn with_tx<R>(&self, f: impl FnOnce(&rusqlite::Transaction<'_>) -> AppResult<R>) -> AppResult<R> {
        let mut guard = self.conn.lock();
        let tx = guard.transaction()?;
        let result = f(&tx)?;
        tx.commit()?;
        Ok(result)
    }

    pub fn current_user_version(&self) -> AppResult<i64> {
        let guard = self.conn.lock();
        let v: i64 = guard.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        Ok(v)
    }

    pub fn set_user_version(&self, version: i64) -> AppResult<()> {
        let guard = self.conn.lock();
        // user_version is a single 32-bit integer; SQLite accepts larger integers but truncates.
        guard.pragma_update(None, "user_version", version)?;
        Ok(())
    }

    /// Used by tests / diagnostic commands.
    #[allow(dead_code)]
    pub fn row_count(&self, table: &str) -> AppResult<i64> {
        let guard = self.conn.lock();
        let sql = format!("SELECT COUNT(*) FROM {}", sanitize_ident(table));
        let n: i64 = guard.query_row(&sql, [], |r| r.get(0))?;
        Ok(n)
    }

    /// Quick integrity check, used by the migrator before running new migrations.
    pub fn integrity_check(&self) -> AppResult<bool> {
        let guard = self.conn.lock();
        let s: String = guard.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        Ok(s.trim().eq_ignore_ascii_case("ok"))
    }
}

fn sanitize_ident(name: &str) -> String {
    // Strict allow-list: identifier must start with letter or _,
    // followed by alphanumerics or _. This avoids SQL-injection vectors
    // when we build dynamic SQL from internal table names.
    let mut out = String::with_capacity(name.len());
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => out.push(c),
        _ => return "invalid".into(),
    }
    for c in chars {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_ident_blocks_injection() {
        assert_eq!(sanitize_ident("users; DROP TABLE x"), "usersDROPTABLEx");
        assert_eq!(sanitize_ident("1abc"), "invalid");
        assert_eq!(sanitize_ident("user_roles"), "user_roles");
    }

    #[test]
    fn open_and_basic() {
        let dir = tempdir_in_target();
        let db = Db::open(&dir).unwrap();
        assert!(db.integrity_check().unwrap());
    }

    fn tempdir_in_target() -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("admin-suite-test-{}", uuid::Uuid::new_v4()));
        p.push("test.sqlite");
        p
    }
}