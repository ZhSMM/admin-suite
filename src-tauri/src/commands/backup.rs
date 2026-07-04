//! Database backup / restore — SQLite uses `VACUUM INTO` so we get a consistent
//! snapshot without pausing the app, then move the file to `<data_dir>/backups/`.
//!
//! Restore writes a flag file (`<data_dir>/.restore_pending`) and asks the
//! frontend to tell the user to relaunch.  On next boot, `apply_pending_restore`
//! in `lib.rs` swaps the current DB for the selected backup and removes the flag.

use crate::auth::rbac::require_permission;
use crate::auth::session::SessionStore;
use crate::commands::settings;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BackupInfo {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub created_at: String,
}

pub fn backups_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("backups")
}

fn restore_flag_path(data_dir: &Path) -> PathBuf {
    data_dir.join(".restore_pending")
}

fn ensure_backups_dir(data_dir: &Path) -> AppResult<PathBuf> {
    let dir = backups_dir(data_dir);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn read_backup_created_at(path: &Path) -> String {
    // Best-effort: pull created_at from the metadata table the migrator writes.
    // If that fails (file from before V1, or corrupted), fall back to mtime.
    let conn = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    );
    if let Ok(conn) = conn {
        let res: Result<String, _> = conn.query_row(
            "SELECT MAX(installed_on) FROM flyway_schema_history",
            [],
            |r| r.get(0),
        );
        if let Ok(s) = res {
            return s;
        }
    }
    // Fallback to file mtime.
    let mtime = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| {
            chrono::DateTime::<chrono::Utc>::from_timestamp(d.as_secs() as i64, 0)
                .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
                .unwrap_or_default()
        });
    mtime.unwrap_or_else(|| "unknown".into())
}

pub fn list_backups(data_dir: &Path) -> AppResult<Vec<BackupInfo>> {
    let dir = backups_dir(data_dir);
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut out: Vec<BackupInfo> = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("sqlite") {
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        let meta = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        out.push(BackupInfo {
            name,
            path: path.to_string_lossy().to_string(),
            size_bytes: meta.len(),
            created_at: read_backup_created_at(&path),
        });
    }
    // Newest first.
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(out)
}

/// Take a consistent backup of the live DB into `<data_dir>/backups/`.
/// `VACUUM INTO` is the recommended pattern because it serialises writes and
/// produces a self-contained snapshot without needing to pause the app.
pub fn create_backup(db: &Db, data_dir: &Path) -> AppResult<BackupInfo> {
    let dir = ensure_backups_dir(data_dir)?;
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let name = format!("admin-suite-{}.sqlite", stamp);
    let dest = dir.join(&name);
    // VACUUM INTO requires a path *not* attached to the running DB.
    let dest_str = dest.to_string_lossy().to_string();
    db.with_conn(|c| {
        c.execute_batch(&format!("VACUUM INTO '{}'", dest_str.replace('\'', "''")))?;
        Ok(())
    })?;
    let meta = std::fs::metadata(&dest)?;
    Ok(BackupInfo {
        name,
        path: dest.to_string_lossy().to_string(),
        size_bytes: meta.len(),
        created_at: chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string(),
    })
}

/// Schedule a restore: write the chosen backup path to a flag file and ask
/// the user to restart.  The actual swap happens in `apply_pending_restore`.
pub fn request_restore(name: &str, data_dir: &Path) -> AppResult<()> {
    let backup_path = backups_dir(data_dir).join(name);
    if !backup_path.exists() {
        return Err(AppError::NotFound(format!("backup {} not found", name)));
    }
    // Refuse any path outside the backups dir.
    let canon_backup = std::fs::canonicalize(&backup_path).map_err(|_| {
        AppError::Validation(format!("invalid backup file: {}", name))
    })?;
    let canon_dir = std::fs::canonicalize(backups_dir(data_dir))
        .unwrap_or_else(|_| backups_dir(data_dir));
    if !canon_backup.starts_with(&canon_dir) {
        return Err(AppError::Validation("invalid backup path".into()));
    }
    std::fs::write(restore_flag_path(data_dir), &canon_backup.to_string_lossy().to_string())?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct RestoreRequest {
    pub backup_name: String,
    pub backup_path: String,
    pub restart_required: bool,
}

pub fn restore(
    db: &Db,
    sessions: &SessionStore,
    token: &str,
    name: &str,
) -> AppResult<RestoreRequest> {
    let user = sessions.lookup(token)?;
    require_permission(&user, "backup:restore")?;
    let data_dir = match db.path().parent() {
        Some(p) => p.to_path_buf(),
        None => return Err(AppError::Validation("db path has no parent".into())),
    };
    request_restore(name, &data_dir)?;
    Ok(RestoreRequest {
        backup_name: name.to_string(),
        backup_path: backups_dir(&data_dir)
            .join(name)
            .to_string_lossy()
            .to_string(),
        restart_required: true,
    })
}

pub fn list(
    db: &Db,
    sessions: &SessionStore,
    token: &str,
) -> AppResult<Vec<BackupInfo>> {
    let user = sessions.lookup(token)?;
    require_permission(&user, "backup:manage")?;
    let data_dir = match db.path().parent() {
        Some(p) => p.to_path_buf(),
        None => return Err(AppError::Validation("db path has no parent".into())),
    };
    list_backups(&data_dir)
}

pub fn create(
    db: &Db,
    sessions: &SessionStore,
    token: &str,
) -> AppResult<BackupInfo> {
    let user = sessions.lookup(token)?;
    require_permission(&user, "backup:manage")?;
    let data_dir = match db.path().parent() {
        Some(p) => p.to_path_buf(),
        None => return Err(AppError::Validation("db path has no parent".into())),
    };
    let info = create_backup(db, &data_dir)?;
    // Trim old backups to keep_count.
    let keep: u32 = settings::get_or(db, "backup.keep_count", "10")?
        .parse()
        .unwrap_or(10);
    let entries = list_backups(&data_dir)?;
    if (entries.len() as u32) > keep {
        for old in entries.iter().skip(keep as usize) {
            let _ = std::fs::remove_file(&old.path);
        }
    }
    Ok(info)
}

pub fn delete(
    db: &Db,
    sessions: &SessionStore,
    token: &str,
    name: &str,
) -> AppResult<()> {
    let user = sessions.lookup(token)?;
    require_permission(&user, "backup:manage")?;
    let data_dir = match db.path().parent() {
        Some(p) => p.to_path_buf(),
        None => return Err(AppError::Validation("db path has no parent".into())),
    };
    let target = backups_dir(&data_dir).join(name);
    let canon = std::fs::canonicalize(&target).unwrap_or(target.clone());
    let canon_dir = std::fs::canonicalize(backups_dir(&data_dir))
        .unwrap_or_else(|_| backups_dir(&data_dir));
    if !canon.starts_with(&canon_dir) {
        return Err(AppError::Validation("invalid backup path".into()));
    }
    if !canon.exists() {
        return Err(AppError::NotFound(format!("backup {} not found", name)));
    }
    std::fs::remove_file(&canon)?;
    Ok(())
}

// =============================================================
// Auto-backup on start (driven by `backup.auto_on_start` setting)
// =============================================================

pub fn maybe_auto_backup(db: &Db, data_dir: &Path) -> AppResult<Option<BackupInfo>> {
    let flag = settings::get_or(db, "backup.auto_on_start", "true")?;
    if flag != "true" {
        return Ok(None);
    }
    let info = create_backup(db, data_dir)?;
    // Trim old.
    let keep: u32 = settings::get_or(db, "backup.keep_count", "10")?
        .parse()
        .unwrap_or(10);
    let entries = list_backups(data_dir)?;
    if (entries.len() as u32) > keep {
        for old in entries.iter().skip(keep as usize) {
            let _ = std::fs::remove_file(&old.path);
        }
    }
    Ok(Some(info))
}

// =============================================================
// Apply pending restore (called from bootstrap before opening the DB)
// =============================================================

pub fn apply_pending_restore(db_path: &Path) -> AppResult<Option<String>> {
    let data_dir = match db_path.parent() {
        Some(p) => p,
        None => return Ok(None),
    };
    let flag = restore_flag_path(data_dir);
    if !flag.exists() {
        return Ok(None);
    }
    let backup_str = std::fs::read_to_string(&flag)?;
    let backup_path = PathBuf::from(backup_str.trim());
    if !backup_path.exists() {
        // Stale flag — clear and move on.
        let _ = std::fs::remove_file(&flag);
        return Err(AppError::Validation(format!(
            "restore pending flag points at non-existent file {:?}",
            backup_path
        )));
    }
    // Refuse path traversal.
    let canon_backup = std::fs::canonicalize(&backup_path)?;
    let canon_dir = std::fs::canonicalize(backups_dir(data_dir))?;
    if !canon_backup.starts_with(&canon_dir) {
        let _ = std::fs::remove_file(&flag);
        return Err(AppError::Validation("invalid restore source".into()));
    }
    // Replace db_path with backup_path.  Keep a "pre-restore" copy as a safety
    // net so the user can recover if the new backup is also broken.
    let pre_restore = data_dir.join(format!(
        "pre-restore-{}.sqlite",
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    ));
    if db_path.exists() {
        std::fs::copy(db_path, &pre_restore)?;
    }
    std::fs::copy(&backup_path, db_path)?;
    let _ = std::fs::remove_file(&flag);
    Ok(Some(pre_restore.to_string_lossy().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_backups_dir_creates() {
        let mut p = std::env::temp_dir();
        p.push(format!("admin-suite-backup-test-{}", uuid::Uuid::new_v4()));
        let dir = ensure_backups_dir(&p).unwrap();
        assert!(dir.exists());
        assert!(dir.join("foo.sqlite").parent().unwrap().exists());
        let _ = std::fs::remove_dir_all(&p);
    }

    #[test]
    fn empty_list_when_no_dir() {
        let mut p = std::env::temp_dir();
        p.push(format!("admin-suite-backup-test-{}", uuid::Uuid::new_v4()));
        let v = list_backups(&p).unwrap();
        assert!(v.is_empty());
    }
}