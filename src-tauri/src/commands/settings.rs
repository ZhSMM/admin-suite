//! Global settings — backed by the `app_state` key/value table.
//!
//! Reads go through the public `app_state_get` helper so other commands
//! (session timeout, password policy, etc.) can read settings without
//! holding a session token.  Writes go through `set_many` which is gated
//! by the `settings:manage` permission.

use crate::auth::rbac::require_permission;
use crate::auth::session::SessionStore;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Setting {
    pub key: String,
    pub value: String,
    pub updated_at: String,
}

fn row_to_setting(r: &rusqlite::Row) -> rusqlite::Result<Setting> {
    Ok(Setting {
        key: r.get(0)?,
        value: r.get(1)?,
        updated_at: r.get(2)?,
    })
}

/// Public, top-level read used by other modules (session timeout, etc).
pub fn get(db: &Db, key: &str) -> AppResult<Option<String>> {
    db.with_conn(|c| {
        let mut stmt = c.prepare("SELECT value FROM app_state WHERE key = ?")?;
        let v = stmt.query_row([key], |r| r.get::<_, String>(0)).ok();
        Ok(v)
    })
}

/// Return the value for `key` or `default` if absent — the common shape for
/// settings that have a sensible static fallback (e.g. session timeout).
pub fn get_or(db: &Db, key: &str, default: &str) -> AppResult<String> {
    Ok(get(db, key)?.unwrap_or_else(|| default.to_string()))
}

pub fn list(db: &Db, sessions: &SessionStore, token: &str) -> AppResult<Vec<Setting>> {
    let user = sessions.lookup(token)?;
    require_permission(&user, "settings:manage")?;
    db.with_conn(|c| {
        let mut stmt = c.prepare(
            "SELECT key, value, updated_at FROM app_state ORDER BY key",
        )?;
        let v = stmt
            .query_map([], row_to_setting)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(v)
    })
}

#[derive(Debug, Deserialize)]
pub struct SettingUpdate {
    pub key: String,
    pub value: String,
}

/// Validate one setting before persisting.  Keys are intentionally limited to
/// the ones we seeded in V7 — anything else is rejected so a typo doesn't
/// silently store garbage in the kv table.
fn validate(key: &str, value: &str) -> AppResult<()> {
    match key {
        "session.timeout_minutes" => {
            let n: u32 = value
                .parse()
                .map_err(|_| AppError::Validation("session.timeout_minutes must be an integer".into()))?;
            if !(5..=24 * 60).contains(&n) {
                return Err(AppError::Validation(
                    "session.timeout_minutes must be between 5 and 1440".into(),
                ));
            }
        }
        "auth.password_min_length" => {
            let n: u32 = value.parse().map_err(|_| {
                AppError::Validation("auth.password_min_length must be an integer".into())
            })?;
            if !(4..=128).contains(&n) {
                return Err(AppError::Validation(
                    "auth.password_min_length must be between 4 and 128".into(),
                ));
            }
        }
        "auth.login_max_failures" => {
            let n: u32 = value.parse().map_err(|_| {
                AppError::Validation("auth.login_max_failures must be an integer".into())
            })?;
            if !(1..=1000).contains(&n) {
                return Err(AppError::Validation(
                    "auth.login_max_failures must be between 1 and 1000".into(),
                ));
            }
        }
        "auth.lockout_minutes" => {
            let n: u32 = value.parse().map_err(|_| {
                AppError::Validation("auth.lockout_minutes must be an integer".into())
            })?;
            if !(1..=24 * 60).contains(&n) {
                return Err(AppError::Validation(
                    "auth.lockout_minutes must be between 1 and 1440".into(),
                ));
            }
        }
        "backup.auto_on_start" => {
            if !matches!(value, "true" | "false") {
                return Err(AppError::Validation(
                    "backup.auto_on_start must be 'true' or 'false'".into(),
                ));
            }
        }
        "backup.keep_count" => {
            let n: u32 = value
                .parse()
                .map_err(|_| AppError::Validation("backup.keep_count must be an integer".into()))?;
            if !(1..=1000).contains(&n) {
                return Err(AppError::Validation(
                    "backup.keep_count must be between 1 and 1000".into(),
                ));
            }
        }
        "ui.default_theme" | "ui.default_locale" => {
            if value.is_empty() {
                return Err(AppError::Validation(format!(
                    "{} must not be empty",
                    key
                )));
            }
        }
        "ui.command_palette" => {
            if !matches!(value, "true" | "false") {
                return Err(AppError::Validation(
                    "ui.command_palette must be 'true' or 'false'".into(),
                ));
            }
        }
        _ => {
            return Err(AppError::Validation(format!("unknown setting key: {}", key)));
        }
    }
    Ok(())
}

pub fn set_many(
    db: &Db,
    sessions: &SessionStore,
    token: &str,
    updates: Vec<SettingUpdate>,
) -> AppResult<Vec<Setting>> {
    let user = sessions.lookup(token)?;
    require_permission(&user, "settings:manage")?;
    if updates.is_empty() {
        return list(db, sessions, token);
    }
    // Validate everything up front so we never write half a batch.
    for u in &updates {
        validate(&u.key, &u.value)?;
    }
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%fZ").to_string();
    db.with_tx(|tx| {
        for u in &updates {
            tx.execute(
                "INSERT INTO app_state (key, value, updated_at) VALUES (?, ?, ?)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value,
                                                 updated_at = excluded.updated_at",
                params![u.key, u.value, now],
            )?;
        }
        Ok(())
    })?;
    list(db, sessions, token)
}