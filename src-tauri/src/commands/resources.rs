use crate::auth::rbac::require_permission;
use crate::auth::session::SessionStore;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::models::{Locale, Resource, ResourceImport, ResourceUpdate, Theme};
use rusqlite::params;
use serde_json::Value;
use uuid::Uuid;

fn row_to_resource(r: &rusqlite::Row) -> rusqlite::Result<Resource> {
    Ok(Resource {
        id: r.get(0)?,
        resource_type: r.get(1)?,
        code: r.get(2)?,
        name: r.get(3)?,
        content: r.get(4)?,
        source: r.get(5)?,
        built_in: r.get::<_, i64>(6)? != 0,
        active: r.get::<_, i64>(7)? != 0,
        created_at: r.get(8)?,
        updated_at: r.get(9)?,
    })
}

pub fn list(
    db: &Db,
    sessions: &SessionStore,
    token: &str,
    resource_type: &str,
) -> AppResult<Vec<Resource>> {
    let user = sessions.lookup(token)?;
    // Any authenticated user can read themes/locales — they are needed to render the UI.
    let _ = user;
    db.with_conn(|c| {
        let mut stmt = c.prepare(
            "SELECT id, resource_type, code, name, content, source, built_in, active,
                    created_at, updated_at
             FROM resources WHERE resource_type = ? ORDER BY built_in DESC, code",
        )?;
        let v = stmt
            .query_map([resource_type], row_to_resource)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(v)
    })
}

pub fn get_active(db: &Db, resource_type: &str) -> AppResult<Option<Resource>> {
    db.with_conn(|c| {
        let mut stmt = c.prepare(
            "SELECT id, resource_type, code, name, content, source, built_in, active,
                    created_at, updated_at
             FROM resources WHERE resource_type = ? AND active = 1 LIMIT 1",
        )?;
        let mut rows = stmt.query([resource_type])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row_to_resource(row)?))
        } else {
            Ok(None)
        }
    })
}

/// Validate and import a theme.
pub fn import_theme(
    db: &Db,
    sessions: &SessionStore,
    token: &str,
    payload: ResourceImport,
) -> AppResult<Resource> {
    let user = sessions.lookup(token)?;
    require_permission(&user, "theme:manage")?;
    if payload.resource_type != "theme" {
        return Err(AppError::Validation(
            "resource_type must be 'theme'".into(),
        ));
    }
    let theme: Theme = serde_json::from_value(payload.content.clone())
        .map_err(|e| AppError::Validation(format!("invalid theme payload: {}", e)))?;
    if theme.id.trim().is_empty() {
        return Err(AppError::Validation("theme.id is required".into()));
    }
    if theme.tokens.is_empty() {
        return Err(AppError::Validation("theme.tokens cannot be empty".into()));
    }
    upsert(db, payload, "imported")
}

pub fn import_locale(
    db: &Db,
    sessions: &SessionStore,
    token: &str,
    payload: ResourceImport,
) -> AppResult<Resource> {
    let user = sessions.lookup(token)?;
    require_permission(&user, "locale:manage")?;
    if payload.resource_type != "locale" {
        return Err(AppError::Validation(
            "resource_type must be 'locale'".into(),
        ));
    }
    let loc: Locale = serde_json::from_value(payload.content.clone())
        .map_err(|e| AppError::Validation(format!("invalid locale payload: {}", e)))?;
    if loc.id.trim().is_empty() {
        return Err(AppError::Validation("locale.id is required".into()));
    }
    if loc.messages.is_empty() {
        return Err(AppError::Validation("locale.messages cannot be empty".into()));
    }
    upsert(db, payload, "imported")
}

fn upsert(db: &Db, payload: ResourceImport, source: &str) -> AppResult<Resource> {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%fZ").to_string();
    let content_str = serde_json::to_string(&payload.content)?;

    db.with_tx(|tx| {
        // Find existing row for (resource_type, code).
        let existing: Option<String> = tx
            .query_row(
                "SELECT id FROM resources WHERE resource_type = ? AND code = ?",
                params![payload.resource_type, payload.code],
                |r| r.get(0),
            )
            .ok();
        if let Some(existing_id) = existing {
            tx.execute(
                "UPDATE resources SET name = ?, content = ?, source = ?, updated_at = ?
                 WHERE id = ?",
                params![payload.name, content_str, source, now, existing_id],
            )?;
        } else {
            let id = Uuid::new_v4().to_string();
            tx.execute(
                "INSERT INTO resources (id, resource_type, code, name, content, source,
                                         built_in, active, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, 0, 0, ?, ?)",
                params![
                    id,
                    payload.resource_type,
                    payload.code,
                    payload.name,
                    content_str,
                    source,
                    now,
                    now
                ],
            )?;
        }
        Ok(())
    })?;

    fetch_one(db, &payload.resource_type, &payload.code)
}

fn fetch_one(db: &Db, resource_type: &str, code: &str) -> AppResult<Resource> {
    db.with_conn(|c| {
        let mut stmt = c.prepare(
            "SELECT id, resource_type, code, name, content, source, built_in, active,
                    created_at, updated_at
             FROM resources WHERE resource_type = ? AND code = ?",
        )?;
        stmt.query_row(params![resource_type, code], row_to_resource)
            .map_err(AppError::Db)
    })
}

pub fn activate(
    db: &Db,
    sessions: &SessionStore,
    token: &str,
    resource_type: &str,
    code: &str,
) -> AppResult<()> {
    let user = sessions.lookup(token)?;
    let required = match resource_type {
        "theme" => "theme:manage",
        "locale" => "locale:manage",
        _ => return Err(AppError::Validation("unknown resource_type".into())),
    };
    require_permission(&user, required)?;

    db.with_tx(|tx| {
        tx.execute(
            "UPDATE resources SET active = 0 WHERE resource_type = ?",
            params![resource_type],
        )?;
        let n = tx.execute(
            "UPDATE resources SET active = 1 WHERE resource_type = ? AND code = ?",
            params![resource_type, code],
        )?;
        if n == 0 {
            return Err(AppError::NotFound(format!(
                "{}/{} not found",
                resource_type, code
            )));
        }
        Ok(())
    })
}

pub fn delete(db: &Db, sessions: &SessionStore, token: &str, id: &str) -> AppResult<()> {
    let user = sessions.lookup(token)?;
    let target = db.with_conn(|c| {
        let mut stmt = c.prepare(
            "SELECT resource_type, built_in, active FROM resources WHERE id = ?",
        )?;
        stmt.query_row([id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)? != 0,
                r.get::<_, i64>(2)? != 0,
            ))
        })
        .map_err(AppError::Db)
    })?;
    let (resource_type, built_in, active) = target;
    if built_in {
        return Err(AppError::Conflict("built-in resources cannot be deleted".into()));
    }
    if active {
        return Err(AppError::Conflict("cannot delete the active resource".into()));
    }
    let required = match resource_type.as_str() {
        "theme" => "theme:manage",
        "locale" => "locale:manage",
        _ => "permission:manage",
    };
    require_permission(&user, required)?;
    db.with_conn(|c| {
        c.execute("DELETE FROM resources WHERE id = ?", params![id])?;
        Ok(())
    })
}

pub fn update(
    db: &Db,
    sessions: &SessionStore,
    token: &str,
    payload: ResourceUpdate,
) -> AppResult<Resource> {
    let user = sessions.lookup(token)?;
    let current = db.with_conn(|c| {
        let mut stmt = c.prepare(
            "SELECT resource_type, built_in, code FROM resources WHERE id = ?",
        )?;
        stmt.query_row([payload.id.as_str()], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)? != 0,
                r.get::<_, String>(2)?,
            ))
        })
        .map_err(AppError::Db)
    })?;
    let (resource_type, built_in, code) = current;

    if built_in {
        return Err(AppError::Conflict(
            "built-in resources cannot be edited in place — use import to add a custom copy".into(),
        ));
    }
    let required = match resource_type.as_str() {
        "theme" => "theme:manage",
        "locale" => "locale:manage",
        _ => "permission:manage",
    };
    require_permission(&user, required)?;

    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%fZ").to_string();
    if let Some(content) = &payload.content {
        match resource_type.as_str() {
            "theme" => {
                let _: Theme = serde_json::from_value(content.clone()).map_err(|e| {
                    AppError::Validation(format!("invalid theme payload: {}", e))
                })?;
            }
            "locale" => {
                let _: Locale = serde_json::from_value(content.clone()).map_err(|e| {
                    AppError::Validation(format!("invalid locale payload: {}", e))
                })?;
            }
            _ => {
                return Err(AppError::Validation("unknown resource_type".into()));
            }
        }
        let s = serde_json::to_string(content)?;
        db.with_conn(|c| {
            c.execute(
                "UPDATE resources SET content = ?, updated_at = ? WHERE id = ?",
                params![s, now, payload.id],
            )?;
            Ok(())
        })?;
    }
    if let Some(name) = &payload.name {
        db.with_conn(|c| {
            c.execute(
                "UPDATE resources SET name = ?, updated_at = ? WHERE id = ?",
                params![name, now, payload.id],
            )?;
            Ok(())
        })?;
    }
    fetch_one(db, &resource_type, &code)
}

/// Parse and validate raw JSON text from an imported file.
pub fn parse_import(resource_type: &str, raw: &str) -> AppResult<(String, String, Value)> {
    let json: Value = serde_json::from_str(raw)
        .map_err(|e| AppError::Validation(format!("invalid JSON: {}", e)))?;
    let (id, name) = match resource_type {
        "theme" => {
            let t: Theme = serde_json::from_value(json.clone())
                .map_err(|e| AppError::Validation(format!("invalid theme: {}", e)))?;
            (t.id, t.label)
        }
        "locale" => {
            let l: Locale = serde_json::from_value(json.clone())
                .map_err(|e| AppError::Validation(format!("invalid locale: {}", e)))?;
            (l.id, l.label)
        }
        _ => return Err(AppError::Validation("unknown resource_type".into())),
    };
    Ok((id, name, json))
}