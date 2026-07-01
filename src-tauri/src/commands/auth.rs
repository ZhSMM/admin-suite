use crate::auth::password::verify_password;
use crate::auth::rbac::{load_permission_codes, load_visible_menu_codes};
use crate::auth::session::{AuthenticatedUser, SessionStore};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::models::UserSafe;
use chrono::Utc;
use rusqlite::params;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct LoginResult {
    pub token: String,
    pub user: UserSafe,
    pub permissions: Vec<String>,
    pub menus: Vec<crate::models::Menu>,
    pub expires_at: String,
}

pub fn login(
    db: &Db,
    sessions: &SessionStore,
    username: &str,
    password: &str,
) -> AppResult<LoginResult> {
    let user = db.with_conn(|c| {
        let mut stmt = c.prepare(
            "SELECT id, username, display_name, password_hash, email, phone, avatar,
                    status, is_super_admin, created_at, updated_at, last_login_at
             FROM users WHERE username = ?",
        )?;
        let row = stmt
            .query_row([username], |r| {
                Ok(crate::models::User {
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
            })
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => AppError::Unauthorized,
                other => AppError::Db(other),
            })?;
        Ok(row)
    })?;

    if user.status != "active" {
        return Err(AppError::Forbidden("account disabled".into()));
    }
    if !verify_password(password, &user.password_hash) {
        return Err(AppError::Unauthorized);
    }

    let role_ids: Vec<String> = db.with_conn(|c| {
        let mut stmt = c.prepare("SELECT role_id FROM user_roles WHERE user_id = ?")?;
        let ids = stmt
            .query_map([user.id.as_str()], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ids)
    })?;

    let role_codes: Vec<String> = db.with_conn(|c| {
        let mut stmt = c.prepare(
            "SELECT r.code FROM roles r INNER JOIN user_roles ur ON ur.role_id = r.id
             WHERE ur.user_id = ?",
        )?;
        let codes = stmt
            .query_map([user.id.as_str()], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(codes)
    })?;

    let permission_codes = load_permission_codes(db, &user.id)?;

    // Update last_login_at.
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%fZ").to_string();
    db.with_conn(|c| {
        c.execute(
            "UPDATE users SET last_login_at = ?, updated_at = ? WHERE id = ?",
            params![now, now, user.id],
        )?;
        Ok(())
    })?;

    let auth_user = AuthenticatedUser {
        user_id: user.id.clone(),
        username: user.username.clone(),
        is_super_admin: user.is_super_admin,
        permission_codes: permission_codes.clone(),
        role_ids: role_ids.clone(),
        expires_at: chrono::Utc::now(), // overwritten by store.issue
    };
    let token = sessions.issue(auth_user.clone())?;

    // Build session-bound visible menus for the frontend layout.
    let visible_codes = load_visible_menu_codes(db, &auth_user)?;
    let visible_set: std::collections::HashSet<&String> = visible_codes.iter().collect();
    let all_menus: Vec<crate::models::Menu> = db.with_conn(|c| {
        let mut stmt = c.prepare(
            "SELECT id, parent_id, code, title, path, icon, component, sort_order,
                    visible, status, menu_type, permission_code, created_at, updated_at
             FROM menus WHERE status='active' ORDER BY sort_order, code",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(crate::models::Menu {
                    id: r.get(0)?,
                    parent_id: r.get(1)?,
                    code: r.get(2)?,
                    title: r.get(3)?,
                    path: r.get(4)?,
                    icon: r.get(5)?,
                    component: r.get(6)?,
                    sort_order: r.get(7)?,
                    visible: r.get::<_, i64>(8)? != 0,
                    status: r.get(9)?,
                    menu_type: r.get(10)?,
                    permission_code: r.get(11)?,
                    created_at: r.get(12)?,
                    updated_at: r.get(13)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    })?;
    let menus: Vec<crate::models::Menu> = all_menus
        .into_iter()
        .filter(|m| {
            // Always include group nodes so the tree structure renders, but only if any child is visible.
            if m.menu_type == "group" {
                return true;
            }
            visible_set.contains(&m.code) || m.parent_id.is_none()
        })
        .collect();

    let mut safe: UserSafe = user.into();
    safe.role_codes = role_codes;

    Ok(LoginResult {
        token,
        user: safe,
        permissions: permission_codes,
        menus,
        expires_at: auth_user.expires_at.to_rfc3339(),
    })
}

pub fn logout(sessions: &SessionStore, token: &str) {
    sessions.revoke(token);
}

pub fn current_user(db: &Db, sessions: &SessionStore, token: &str) -> AppResult<UserSafe> {
    let auth = sessions.lookup(token)?;
    let user = db.with_conn(|c| {
        let mut stmt = c.prepare(
            "SELECT id, username, display_name, password_hash, email, phone, avatar,
                    status, is_super_admin, created_at, updated_at, last_login_at
             FROM users WHERE id = ?",
        )?;
        stmt.query_row([auth.user_id.as_str()], |r| {
            Ok(crate::models::User {
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
        })
        .map_err(AppError::Db)
    })?;
    let role_ids = crate::commands::users::load_role_ids(db, &auth.user_id)?;
    let role_codes = crate::commands::users::load_role_codes(db, &auth.user_id)?;
    let mut safe: UserSafe = user.into();
    safe.role_ids = role_ids;
    safe.role_codes = role_codes;
    Ok(safe)
}