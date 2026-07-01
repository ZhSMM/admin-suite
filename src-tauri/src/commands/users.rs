use crate::auth::password::hash_password;
use crate::auth::rbac::require_permission;
use crate::auth::session::SessionStore;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::models::{User, UserCreate, UserListQuery, UserListResult, UserSafe, UserUpdate};
use rusqlite::params;
use uuid::Uuid;

pub fn load_role_ids(db: &Db, user_id: &str) -> AppResult<Vec<String>> {
    db.with_conn(|c| {
        let mut stmt = c.prepare("SELECT role_id FROM user_roles WHERE user_id = ?")?;
        let ids = stmt
            .query_map([user_id], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ids)
    })
}

pub fn load_role_codes(db: &Db, user_id: &str) -> AppResult<Vec<String>> {
    db.with_conn(|c| {
        let mut stmt = c.prepare(
            "SELECT r.code FROM roles r INNER JOIN user_roles ur ON ur.role_id = r.id
             WHERE ur.user_id = ?",
        )?;
        let codes = stmt
            .query_map([user_id], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(codes)
    })
}

fn row_to_user(r: &rusqlite::Row) -> rusqlite::Result<User> {
    Ok(User {
        id: r.get(0)?,
        username: r.get(1)?,
        display_name: r.get(2)?,
        password_hash: r.get(3)?,
        email: r.get(4)?,
        phone: r.get(5)?,
        avatar: r.get(6)?,
        status: r.get(7)?,
        is_super_admin: r.get::<_, i64>(8)? != 0,
        created_at: r.get(9)?,
        updated_at: r.get(10)?,
        last_login_at: r.get(11)?,
        role_ids: vec![],
    })
}

fn select_user(db: &Db, id: &str) -> AppResult<User> {
    db.with_conn(|c| {
        let mut stmt = c.prepare(
            "SELECT id, username, display_name, password_hash, email, phone, avatar,
                    status, is_super_admin, created_at, updated_at, last_login_at
             FROM users WHERE id = ?",
        )?;
        stmt.query_row([id], row_to_user).map_err(AppError::Db)
    })
}

pub fn list(
    db: &Db,
    sessions: &SessionStore,
    token: &str,
    query: UserListQuery,
) -> AppResult<UserListResult> {
    let user = sessions.lookup(token)?;
    require_permission(&user, "user:read")?;

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 200);
    let offset = (page - 1) * page_size;
    let keyword = query.keyword.unwrap_or_default();
    let status = query.status.unwrap_or_default();
    let role_id = query.role_id.unwrap_or_default();

    db.with_conn(|c| {
        let mut where_clauses: Vec<String> = Vec::new();
        let mut binds: Vec<String> = Vec::new();
        if !keyword.is_empty() {
            where_clauses.push("(username LIKE ?1 OR display_name LIKE ?1)".into());
            binds.push(format!("%{}%", keyword));
        }
        if !status.is_empty() {
            where_clauses.push(format!("status = ?{}", binds.len() + 1));
            binds.push(status);
        }
        if !role_id.is_empty() {
            where_clauses.push(format!(
                "id IN (SELECT user_id FROM user_roles WHERE role_id = ?{})",
                binds.len() + 1
            ));
            binds.push(role_id);
        }
        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };

        let count_sql = format!("SELECT COUNT(*) FROM users {}", where_sql);
        let total: i64 = {
            let mut stmt = c.prepare(&count_sql)?;
            let mut rows = stmt.query(rusqlite::params_from_iter(binds.iter()))?;
            rows.next()?.unwrap().get(0)?
        };

        let list_sql = format!(
            "SELECT id, username, display_name, password_hash, email, phone, avatar,
                    status, is_super_admin, created_at, updated_at, last_login_at
             FROM users {} ORDER BY created_at DESC LIMIT ? OFFSET ?",
            where_sql
        );
        let mut stmt = c.prepare(&list_sql)?;
        let mut all_binds = binds.clone();
        all_binds.push(page_size.to_string());
        all_binds.push(offset.to_string());

        let users: Vec<User> = stmt
            .query_map(rusqlite::params_from_iter(all_binds.iter()), row_to_user)?
            .collect::<Result<Vec<_>, _>>()?;

        let mut items: Vec<UserSafe> = Vec::with_capacity(users.len());
        for u in users {
            let role_ids = load_role_ids(db, &u.id)?;
            let role_codes = load_role_codes(db, &u.id)?;
            let mut safe: UserSafe = u.into();
            safe.role_ids = role_ids;
            safe.role_codes = role_codes;
            items.push(safe);
        }

        Ok(UserListResult {
            items,
            total,
            page,
            page_size,
        })
    })
}

pub fn get(db: &Db, sessions: &SessionStore, token: &str, id: &str) -> AppResult<UserSafe> {
    let user = sessions.lookup(token)?;
    require_permission(&user, "user:read")?;
    let u = select_user(db, id)?;
    let mut safe: UserSafe = u.into();
    safe.role_ids = load_role_ids(db, id)?;
    safe.role_codes = load_role_codes(db, id)?;
    Ok(safe)
}

pub fn create(
    db: &Db,
    sessions: &SessionStore,
    token: &str,
    payload: UserCreate,
) -> AppResult<UserSafe> {
    let user = sessions.lookup(token)?;
    require_permission(&user, "user:write")?;

    if payload.username.trim().is_empty() {
        return Err(AppError::Validation("username is required".into()));
    }
    if payload.password.len() < 6 {
        return Err(AppError::Validation("password must be at least 6 chars".into()));
    }

    let id = Uuid::new_v4().to_string();
    let hash = hash_password(&payload.password)?;
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%fZ").to_string();
    let status = payload.status.unwrap_or_else(|| "active".into());

    db.with_tx(|tx| {
        tx.execute(
            "INSERT INTO users (id, username, display_name, password_hash, email, phone,
                                avatar, status, is_super_admin, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?)",
            params![
                id,
                payload.username.trim(),
                payload.display_name,
                hash,
                payload.email,
                payload.phone,
                payload.avatar,
                status,
                now,
                now,
            ],
        )?;
        for role_id in &payload.role_ids {
            tx.execute(
                "INSERT INTO user_roles (user_id, role_id) VALUES (?, ?)",
                params![id, role_id],
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
    payload: UserUpdate,
) -> AppResult<UserSafe> {
    let user = sessions.lookup(token)?;
    require_permission(&user, "user:write")?;

    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%fZ").to_string();

    db.with_tx(|tx| {
        let mut sets: Vec<String> = Vec::new();
        let mut binds: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(v) = &payload.display_name {
            sets.push("display_name = ?".into());
            binds.push(Box::new(v.clone()));
        }
        if let Some(v) = &payload.email {
            sets.push("email = ?".into());
            binds.push(Box::new(v.clone()));
        }
        if let Some(v) = &payload.phone {
            sets.push("phone = ?".into());
            binds.push(Box::new(v.clone()));
        }
        if let Some(v) = &payload.avatar {
            sets.push("avatar = ?".into());
            binds.push(Box::new(v.clone()));
        }
        if let Some(v) = &payload.status {
            sets.push("status = ?".into());
            binds.push(Box::new(v.clone()));
        }
        if let Some(p) = &payload.password {
            if p.len() < 6 {
                return Err(AppError::Validation("password too short".into()));
            }
            let h = hash_password(p)?;
            sets.push("password_hash = ?".into());
            binds.push(Box::new(h));
        }
        if !sets.is_empty() {
            sets.push("updated_at = ?".into());
            binds.push(Box::new(now.clone()));
            let sql = format!("UPDATE users SET {} WHERE id = ?", sets.join(", "));
            binds.push(Box::new(payload.id.clone()));
            let bind_refs: Vec<&dyn rusqlite::ToSql> = binds.iter().map(|b| b.as_ref()).collect();
            tx.execute(&sql, rusqlite::params_from_iter(bind_refs))?;
        }

        if let Some(role_ids) = &payload.role_ids {
            tx.execute("DELETE FROM user_roles WHERE user_id = ?", params![payload.id])?;
            for role_id in role_ids {
                tx.execute(
                    "INSERT INTO user_roles (user_id, role_id) VALUES (?, ?)",
                    params![payload.id, role_id],
                )?;
            }
        }
        Ok(())
    })?;

    get(db, sessions, token, &payload.id)
}

pub fn delete(db: &Db, sessions: &SessionStore, token: &str, id: &str) -> AppResult<()> {
    let user = sessions.lookup(token)?;
    require_permission(&user, "user:delete")?;

    // Refuse to delete the only remaining super-admin.
    let target = select_user(db, id)?;
    if target.is_super_admin {
        let remaining: i64 = db.with_conn(|c| {
            let n: i64 = c.query_row(
                "SELECT COUNT(*) FROM users WHERE is_super_admin = 1 AND status='active' AND id != ?",
                [id],
                |r| r.get(0),
            )?;
            Ok(n)
        })?;
        if remaining == 0 {
            return Err(AppError::Conflict(
                "cannot delete the last active super-admin".into(),
            ));
        }
    }

    db.with_conn(|c| {
        c.execute("DELETE FROM users WHERE id = ?", params![id])?;
        Ok(())
    })
}