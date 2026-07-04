//! Flyway-like migration runner for SQLite.
//!
//! ## Conventions
//! - `V{version}__{description}.sql` — versioned, applied once, in version order.
//! - `R__{description}.sql` — repeatable, re-applied whenever checksum changes (views, functions...).
//! - One transaction per migration; if it fails the whole DB is left untouched for that step.
//! - `flyway_schema_history` (the standard Flyway table) records applied migrations, with
//!   a SHA-256 checksum to detect file tampering.
//!
//! ## Storage
//! - Migration files are read from the directory passed at construction. The Tauri bundle
//!   embeds them as resources (`migrations/**`) and we resolve the real path at runtime.

use crate::db::Db;
use crate::error::{AppError, AppResult};
use rusqlite::params;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize)]
pub struct MigrationRecord {
    pub installed_rank: i64,
    pub version: Option<String>,
    pub description: String,
    pub script: String,
    pub checksum: String,
    pub installed_by: String,
    pub installed_on: String,
    pub execution_time_ms: i64,
    pub success: bool,
    pub kind: String, // "VERSIONED" | "REPEATABLE"
}

#[derive(Debug, Clone)]
struct MigrationFile {
    /// "VERSIONED" | "REPEATABLE"
    kind: &'static str,
    /// For VERSIONED: dotted version (e.g. "1", "1.1", "2.3.4").
    /// For REPEATABLE: None.
    version: Option<String>,
    description: String,
    script_name: String,
    sql: String,
    checksum: String,
}

/// Resolve a usable migrations directory at runtime. Falls back to:
/// 1. Path passed in
/// 2. `<exe>/migrations`
/// 3. `<exe>/../../../migrations` (typical `tauri dev` layout: target/debug/admin-suite  ->  repo/migrations)
/// 4. `resources/migrations` next to the exe (production bundle)
pub fn resolve_migrations_dir(hint: Option<PathBuf>) -> AppResult<PathBuf> {
    let exe_dir = std::env::current_exe()
        .map(|p| p.parent().map(|p| p.to_path_buf()))
        .ok()
        .flatten();

    let candidates: Vec<PathBuf> = match hint {
        Some(p) => vec![p],
        None => {
            let mut v = Vec::new();
            if let Some(d) = &exe_dir {
                v.push(d.join("migrations"));
                v.push(d.join("resources").join("migrations"));
                // `cargo run` layout: src-tauri/target/debug/admin-suite -> src-tauri/migrations
                v.push(d.join("../../../migrations"));
                v.push(d.join("../../migrations"));
            }
            // Finally: CWD relative (dev).
            v.push(PathBuf::from("migrations"));
            v.push(PathBuf::from("src-tauri/migrations"));
            v
        }
    };

    for c in &candidates {
        if c.exists() && c.is_dir() {
            return Ok(c.canonicalize().unwrap_or(c.clone()));
        }
    }
    Err(AppError::Migration(format!(
        "migrations directory not found. Tried: {:?}",
        candidates
    )))
}

/// Run all pending migrations. Safe to call on every startup.
pub fn run_migrations(db: &Db, dir: &Path) -> AppResult<Vec<MigrationRecord>> {
    ensure_history_table(db)?;
    let applied = load_applied(db)?;
    let files = discover_migrations(dir)?;

    // ---- Integrity check on already-applied migrations (Flyway `validate`) ----
    for rec in &applied {
        if !rec.success {
            return Err(AppError::Migration(format!(
                "previous migration {} failed; manual repair required (mark as fixed or clean DB)",
                rec.script
            )));
        }
        if let Some(script) = files.iter().find(|m| m.script_name == rec.script) {
            if script.checksum != rec.checksum {
                return Err(AppError::Migration(format!(
                    "checksum mismatch for {}: expected {}, file is {} — file changed after apply",
                    rec.script, rec.checksum, script.checksum
                )));
            }
        } else {
            // File removed on disk but DB says applied: that's a problem too.
            return Err(AppError::Migration(format!(
                "applied migration {} is missing on disk",
                rec.script
            )));
        }
    }

    // ---- Resolve versions present on disk ----
    let mut applied_versions: Vec<String> = applied
        .iter()
        .filter_map(|r| r.version.clone())
        .collect();
    applied_versions.sort();

    let mut new_records: Vec<MigrationRecord> = Vec::new();

    // Apply versioned migrations in version order.
    let mut versioned: Vec<&MigrationFile> = files
        .iter()
        .filter(|m| m.kind == "VERSIONED")
        .collect();
    versioned.sort_by(|a, b| {
        cmp_versions(a.version.as_deref().unwrap(), b.version.as_deref().unwrap())
    });

    for mig in versioned {
        let v = mig.version.as_deref().unwrap();
        if applied_versions.iter().any(|av| av == v) {
            continue;
        }
        let rec = apply_versioned(db, mig)?;
        new_records.push(rec);
        applied_versions.push(v.to_string());
        applied_versions.sort();
    }

    // Apply repeatable migrations whose checksum has changed since last apply.
    let mut repeatable: Vec<&MigrationFile> = files
        .iter()
        .filter(|m| m.kind == "REPEATABLE")
        .collect();
    repeatable.sort_by(|a, b| a.script_name.cmp(&b.script_name));
    for mig in repeatable {
        let prior = applied.iter().find(|r| r.script == mig.script_name);
        if let Some(prior) = prior {
            if prior.checksum == mig.checksum {
                continue;
            }
        }
        let rec = apply_repeatable(db, mig, prior.map(|r| r.installed_rank))?;
        new_records.push(rec);
    }

    Ok(new_records)
}

fn ensure_history_table(db: &Db) -> AppResult<()> {
    db.with_conn(|c| {
        c.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS flyway_schema_history (
                installed_rank    INTEGER PRIMARY KEY AUTOINCREMENT,
                version           VARCHAR(50),
                description       VARCHAR(200) NOT NULL,
                type              VARCHAR(20)  NOT NULL,
                script            VARCHAR(1000) NOT NULL,
                checksum          VARCHAR(64)  NOT NULL,
                installed_by      VARCHAR(100) NOT NULL,
                installed_on      DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
                execution_time    INTEGER      NOT NULL,
                success           INTEGER      NOT NULL DEFAULT 1
            );
            "#,
        )?;
        Ok(())
    })
}

fn load_applied(db: &Db) -> AppResult<Vec<MigrationRecord>> {
    db.with_conn(|c| {
        let mut stmt = c.prepare(
            "SELECT installed_rank, version, description, type, script, checksum,
                    installed_by, installed_on, execution_time, success
             FROM flyway_schema_history ORDER BY installed_rank",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(MigrationRecord {
                    installed_rank: r.get(0)?,
                    version: r.get(1)?,
                    description: r.get(2)?,
                    kind: r.get(3)?,
                    script: r.get(4)?,
                    checksum: r.get(5)?,
                    installed_by: r.get(6)?,
                    installed_on: r.get(7)?,
                    execution_time_ms: r.get(8)?,
                    success: r.get::<_, i64>(9)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    })
}

fn apply_versioned(db: &Db, mig: &MigrationFile) -> AppResult<MigrationRecord> {
    let started = std::time::Instant::now();
    let installed_by = current_user();

    let result = db.with_tx(|tx| {
        tx.execute_batch(&mig.sql)?;
        // Record inserted within the same transaction so we never end up half-applied.
        tx.execute(
            "INSERT INTO flyway_schema_history
             (version, description, type, script, checksum, installed_by, execution_time, success)
             VALUES (?, ?, 'SQL', ?, ?, ?, ?, 1)",
            params![
                mig.version,
                mig.description,
                mig.script_name,
                mig.checksum,
                installed_by,
                started.elapsed().as_millis() as i64,
            ],
        )?;
        Ok(())
    });

    let elapsed = started.elapsed().as_millis() as i64;

    match result {
        Ok(_) => Ok(MigrationRecord {
            installed_rank: 0, // filled below
            version: mig.version.clone(),
            description: mig.description.clone(),
            script: mig.script_name.clone(),
            checksum: mig.checksum.clone(),
            installed_by,
            installed_on: now_iso(),
            execution_time_ms: elapsed,
            success: true,
            kind: "VERSIONED".into(),
        }),
        Err(e) => Err(AppError::Migration(format!(
            "failed to apply {} ({}): {}",
            mig.script_name, mig.version.as_deref().unwrap_or("?"), e
        ))),
    }
}

fn apply_repeatable(
    db: &Db,
    mig: &MigrationFile,
    prior_rank: Option<i64>,
) -> AppResult<MigrationRecord> {
    let started = std::time::Instant::now();
    let installed_by = current_user();

    let result = db.with_tx(|tx| {
        // For repeatable, drop any prior history row first so we keep a single lineage row.
        if let Some(rank) = prior_rank {
            tx.execute(
                "DELETE FROM flyway_schema_history WHERE installed_rank = ?",
                params![rank],
            )?;
        }
        tx.execute_batch(&mig.sql)?;
        tx.execute(
            "INSERT INTO flyway_schema_history
             (version, description, type, script, checksum, installed_by, execution_time, success)
             VALUES (NULL, ?, 'SQL', ?, ?, ?, ?, 1)",
            params![
                mig.description,
                mig.script_name,
                mig.checksum,
                installed_by,
                started.elapsed().as_millis() as i64,
            ],
        )?;
        Ok(())
    });

    match result {
        Ok(_) => Ok(MigrationRecord {
            installed_rank: 0,
            version: None,
            description: mig.description.clone(),
            script: mig.script_name.clone(),
            checksum: mig.checksum.clone(),
            installed_by,
            installed_on: now_iso(),
            execution_time_ms: started.elapsed().as_millis() as i64,
            success: true,
            kind: "REPEATABLE".into(),
        }),
        Err(e) => Err(AppError::Migration(format!(
            "failed to apply repeatable {}: {}",
            mig.script_name, e
        ))),
    }
}

/// List migrations on disk (and whether they are applied). Useful for a "migrations" admin view.
pub fn status(db: &Db, dir: &Path) -> AppResult<Vec<MigrationStatus>> {
    ensure_history_table(db)?;
    let applied = load_applied(db)?;
    let files = discover_migrations(dir)?;
    let mut out: Vec<MigrationStatus> = Vec::new();

    for f in files {
        let status = applied.iter().find(|r| r.script == f.script_name);
        out.push(MigrationStatus {
            script: f.script_name.clone(),
            kind: f.kind.into(),
            version: f.version.clone(),
            description: f.description.clone(),
            checksum: f.checksum.clone(),
            state: match status {
                Some(s) if s.success => "APPLIED".into(),
                Some(_) => "FAILED".into(),
                None => "PENDING".into(),
            },
            installed_on: status.map(|s| s.installed_on.clone()),
            execution_time_ms: status.map(|s| s.execution_time_ms),
        });
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize)]
pub struct MigrationStatus {
    pub script: String,
    pub kind: String,
    pub version: Option<String>,
    pub description: String,
    pub checksum: String,
    pub state: String, // APPLIED | PENDING | FAILED
    pub installed_on: Option<String>,
    pub execution_time_ms: Option<i64>,
}

fn discover_migrations(dir: &Path) -> AppResult<Vec<MigrationFile>> {
    let mut out = Vec::new();
    if !dir.exists() {
        return Err(AppError::Migration(format!(
            "migrations dir does not exist: {}",
            dir.display()
        )));
    }
    for entry in WalkDir::new(dir).sort_by_file_name() {
        let entry = entry.map_err(|e| AppError::Migration(e.to_string()))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };
        let parsed = parse_migration_name(name);
        let parsed = match parsed {
            Some(p) => p,
            None => continue,
        };
        let sql = std::fs::read_to_string(path).map_err(|e| {
            AppError::Migration(format!("failed to read migration {}: {}", name, e))
        })?;
        let checksum = sha256_hex(&sql);
        out.push(MigrationFile {
            kind: parsed.kind,
            version: parsed.version,
            description: parsed.description,
            script_name: name.to_string(),
            sql,
            checksum,
        });
    }
    Ok(out)
}

struct ParsedName {
    kind: &'static str,
    version: Option<String>,
    description: String,
}

/// `V1__init.sql` or `V1.1__init.sql` or `R__views.sql`.
fn parse_migration_name(name: &str) -> Option<ParsedName> {
    if !name.ends_with(".sql") {
        return None;
    }
    let stem = &name[..name.len() - 4];
    if let Some(rest) = stem.strip_prefix('V') {
        let (ver, desc) = rest.split_once("__")?;
        if ver.is_empty() || !ver.chars().all(|c| c.is_ascii_digit() || c == '.') {
            return None;
        }
        return Some(ParsedName {
            kind: "VERSIONED",
            version: Some(ver.to_string()),
            description: desc.to_string(),
        });
    }
    if let Some(rest) = stem.strip_prefix('R') {
        let desc = rest.strip_prefix("__")?;
        return Some(ParsedName {
            kind: "REPEATABLE",
            version: None,
            description: desc.to_string(),
        });
    }
    None
}

fn cmp_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let av: Vec<u64> = a.split('.').map(|x| x.parse().unwrap_or(0)).collect();
    let bv: Vec<u64> = b.split('.').map(|x| x.parse().unwrap_or(0)).collect();
    av.cmp(&bv)
}

fn sha256_hex(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    hex::encode(h.finalize())
}

fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn current_user() -> String {
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "system".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_v() {
        let p = parse_migration_name("V1__init.sql").unwrap();
        assert_eq!(p.version.as_deref(), Some("1"));
        assert_eq!(p.description, "init");
        assert_eq!(p.kind, "VERSIONED");
    }

    #[test]
    fn parse_r() {
        let p = parse_migration_name("R__views.sql").unwrap();
        assert_eq!(p.version, None);
        assert_eq!(p.description, "views");
        assert_eq!(p.kind, "REPEATABLE");
    }

    #[test]
    fn ignore_other_files() {
        assert!(parse_migration_name("README.md").is_none());
        assert!(parse_migration_name("init.sql").is_none());
    }

    #[test]
    fn cmp_versions_works() {
        assert!(cmp_versions("1", "2").is_lt());
        assert!(cmp_versions("1.10", "1.2").is_gt());
        assert!(cmp_versions("1.2.0", "1.2").is_gt());
    }

    /// End-to-end: boot a fresh DB and run every shipped migration against it.
    /// This is the regression that catches typos in column names (the original
    /// V7 used `parent_code` instead of `parent_id` and only blew up at user
    /// startup, never in CI).
    #[test]
    fn full_migration_suite_applies_clean() {
        use crate::db::Db;
        let mut p = std::env::temp_dir();
        p.push(format!("admin-suite-migrate-test-{}.sqlite", uuid::Uuid::new_v4()));
        let _ = std::fs::remove_file(&p);
        let db = Db::open(&p).expect("open test db");
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
        let applied = run_migrations(&db, &dir).expect("migrations apply");
        // We seeded V1..V8 — assert the count is at least 8 so a typo in V9
        // can't silently pass CI either.
        assert!(
            applied.len() >= 8,
            "expected at least 8 migrations applied, got {}",
            applied.len()
        );
        // Spot-check: app_state and the new menus rows from V7 exist.
        db.with_conn(|c| {
            let has_state: i64 =
                c.query_row("SELECT COUNT(*) FROM app_state", [], |r| r.get(0))?;
            assert!(has_state >= 9, "app_state should have at least 9 rows");
            let m_settings: i64 = c.query_row(
                "SELECT COUNT(*) FROM menus WHERE code = 'system.settings'",
                [],
                |r| r.get(0),
            )?;
            assert_eq!(m_settings, 1, "system.settings menu must be seeded");
            // V8 should refresh built-in locales with the current bundle key count.
            for code in ["en-US", "zh-CN"] {
                let content: String = c.query_row(
                    "SELECT content FROM resources WHERE resource_type = 'locale' AND code = ?",
                    [code],
                    |r| r.get(0),
                )?;
                let parsed: serde_json::Value = serde_json::from_str(&content)
                    .expect("built-in locale content must be valid JSON");
                let n = parsed
                    .get("messages")
                    .and_then(|m| m.as_object())
                    .map(|o| o.len())
                    .unwrap_or(0);
                assert!(
                    n >= 400,
                    "built-in locale {} only has {} keys — V8 may be stale, re-run scripts/gen-v8-locale-refresh.py",
                    code, n
                );
            }
            Ok(())
        })
        .unwrap();
        let _ = std::fs::remove_file(&p);
    }
}