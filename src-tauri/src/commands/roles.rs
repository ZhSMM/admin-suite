use crate::auth::rbac::require_permission;
use crate::auth::session::SessionStore;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::models::{Role, RoleCreate, RoleUpdate};
use rusqlite::params;
use uuid::Uuid;

fn row_to_role(r: &rusqlite::Row) -> rusqlite::Result<Role> {
    Ok(Role {
        id: r.get(0)?,
        code: r.get(1)?,
        name: r.get(2)?,
        description: r.get(3)?,
        status: r.get(4)?,
        built_in: r.get::<_, i64>(5)? != 0,
        sort_order: r.get(6)?,
        created_at: r.get(7)?,
        updated_at: r.get(8)?,
        permission_codes: vec![],
    })
}

fn load_permission_codes_for_role(db: &Db, role_id: &str) -> AppResult<Vec<String>> {
    db.with_conn(|c| {
        let mut stmt = c.prepare(
            "SELECT p.code FROM permissions p
             INNER JOIN role_permissions rp ON rp.permission_id = p.id
             WHERE rp.role_id = ?",
        )?;
        let codes = stmt
            .query_map([role_id], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(codes)
    })
}

fn load_menu_ids_for_role(db: &Db, role_id: &str) -> AppResult<Vec<String>> {
    db.with_conn(|c| {
        let mut stmt = c.prepare("SELECT menu_id FROM role_menus WHERE role_id = ?")?;
        let ids = stmt
            .query_map([role_id], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ids)
    })
}

pub fn list(db: &Db, sessions: &SessionStore, token: &str) -> AppResult<Vec<Role>> {
    let user = sessions.lookup(token)?;
    require_permission(&user, "role:read")?;

    let roles = db.with_conn(|c| {
        let mut stmt = c.prepare(
            "SELECT id, code, name, description, status, built_in, sort_order, created_at, updated_at
             FROM roles ORDER BY sort_order, code",
        )?;
        let v = stmt
            .query_map([], row_to_role)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(v)
    })?;

    let mut out = Vec::with_capacity(roles.len());
    for mut r in roles {
        r.permission_codes = load_permission_codes_for_role(db, &r.id)?;
        out.push(r);
    }
    Ok(out)
}

pub fn get(db: &Db, sessions: &SessionStore, token: &str, id: &str) -> AppResult<Role> {
    let user = sessions.lookup(token)?;
    require_permission(&user, "role:read")?;
    let mut role = db.with_conn(|c| {
        let mut stmt = c.prepare(
            "SELECT id, code, name, description, status, built_in, sort_order, created_at, updated_at
             FROM roles WHERE id = ?",
        )?;
        stmt.query_row([id], row_to_role).map_err(AppError::Db)
    })?;
    role.permission_codes = load_permission_codes_for_role(db, id)?;
    Ok(role)
}

pub fn create(
    db: &Db,
    sessions: &SessionStore,
    token: &str,
    payload: RoleCreate,
) -> AppResult<Role> {
    let user = sessions.lookup(token)?;
    require_permission(&user, "role:write")?;
    if payload.code.trim().is_empty() {
        return Err(AppError::Validation("role code is required".into()));
    }
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%fZ").to_string();
    let status = payload.status.unwrap_or_else(|| "active".into());
    let sort_order = payload.sort_order.unwrap_or(100);

    db.with_tx(|tx| {
        tx.execute(
            "INSERT INTO roles (id, code, name, description, status, built_in, sort_order,
                                created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, 0, ?, ?, ?)",
            params![
                id,
                payload.code.trim(),
                payload.name,
                payload.description,
                status,
                sort_order,
                now,
                now
            ],
        )?;
        for pid in &payload.permission_ids {
            tx.execute(
                "INSERT OR IGNORE INTO role_permissions (role_id, permission_id) VALUES (?, ?)",
                params![id, pid],
            )?;
        }
        Ok(())
    })?;
    get(db, sessions, token, &id)
}

pub fn update(
    db: &Db,
    sessions: &SessionStore,
    token: &str,
    payload: RoleUpdate,
) -> AppResult<Role> {
    let user = sessions.lookup(token)?;
    require_permission(&user, "role:write")?;
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%fZ").to_string();

    db.with_tx(|tx| {
        let mut sets: Vec<String> = Vec::new();
        let mut binds: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(v) = &payload.name {
            sets.push("name = ?".into());
            binds.push(Box::new(v.clone()));
        }
        if let Some(v) = &payload.description {
            sets.push("description = ?".into());
            binds.push(Box::new(v.clone()));
        }
        if let Some(v) = &payload.status {
            sets.push("status = ?".into());
            binds.push(Box::new(v.clone()));
        }
        if let Some(v) = payload.sort_order {
            sets.push("sort_order = ?".into());
            binds.push(Box::new(v));
        }

        if !sets.is_empty() {
            sets.push("updated_at = ?".into());
            binds.push(Box::new(now.clone()));
            let sql = format!("UPDATE roles SET {} WHERE id = ?", sets.join(", "));
            binds.push(Box::new(payload.id.clone()));
            let bind_refs: Vec<&dyn rusqlite::ToSql> = binds.iter().map(|b| b.as_ref()).collect();
            tx.execute(&sql, rusqlite::params_from_iter(bind_refs))?;
        }

        if let Some(perm_ids) = &payload.permission_ids {
            tx.execute(
                "DELETE FROM role_permissions WHERE role_id = ?",
                params![payload.id],
            )?;
            for pid in perm_ids {
                tx.execute(
                    "INSERT OR IGNORE INTO role_permissions (role_id, permission_id) VALUES (?, ?)",
                    params![payload.id, pid],
                )?;
            }
        }
        Ok(())
    })?;
    get(db, sessions, token, &payload.id)
}

pub fn delete(db: &Db, sessions: &SessionStore, token: &str, id: &str) -> AppResult<()> {
    let user = sessions.lookup(token)?;
    require_permission(&user, "role:delete")?;
    let role = get(db, sessions, token, id)?;
    if role.built_in {
        return Err(AppError::Conflict("built-in roles cannot be deleted".into()));
    }
    db.with_conn(|c| {
        c.execute("DELETE FROM roles WHERE id = ?", params![id])?;
        Ok(())
    })
}

pub fn assign_menus(
    db: &Db,
    sessions: &SessionStore,
    token: &str,
    role_id: &str,
    menu_ids: Vec<String>,
) -> AppResult<()> {
    let user = sessions.lookup(token)?;
    require_permission(&user, "role:write")?;
    db.with_tx(|tx| {
        tx.execute("DELETE FROM role_menus WHERE role_id = ?", params![role_id])?;
        for menu_id in &menu_ids {
            tx.execute(
                "INSERT OR IGNORE INTO role_menus (role_id, menu_id) VALUES (?, ?)",
                params![role_id, menu_id],
            )?;
        }
        Ok(())
    })
}

pub fn get_role_menus(db: &Db, sessions: &SessionStore, token: &str, role_id: &str) -> AppResult<Vec<String>> {
    let user = sessions.lookup(token)?;
    require_permission(&user, "role:read")?;
    load_menu_ids_for_role(db, role_id)
}