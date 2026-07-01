//! Role-Based Access Control. The session store already loads `permission_codes`
//! at login, so most checks are O(N) over a small list. Critical commands
//! (role/permission/audit) re-check from the DB to avoid privilege escalation
//! through stale sessions.

use crate::auth::session::AuthenticatedUser;
use crate::db::Db;
use crate::error::{AppError, AppResult};

pub fn require_permission(user: &AuthenticatedUser, code: &str) -> AppResult<()> {
    if user.is_super_admin {
        return Ok(());
    }
    if has_permission(user, code) {
        Ok(())
    } else {
        Err(AppError::Forbidden(code.to_string()))
    }
}

pub fn has_permission(user: &AuthenticatedUser, code: &str) -> bool {
    if user.is_super_admin {
        return true;
    }
    // Direct match.
    if user.permission_codes.iter().any(|c| c == code) {
        return true;
    }
    // Wildcard resource ('*:*' on the user).
    if user.permission_codes.iter().any(|c| c == "*:*") {
        return true;
    }
    // Resource-wildcard ('user:*').
    let (res, _) = match code.split_once(':') {
        Some(t) => t,
        None => return false,
    };
    let wild = format!("{}:*", res);
    user.permission_codes.iter().any(|c| c == &wild)
}

/// Fetch the full set of permission codes for a user from the DB.
pub fn load_permission_codes(db: &Db, user_id: &str) -> AppResult<Vec<String>> {
    db.with_conn(|c| {
        let mut stmt = c.prepare(
            "SELECT DISTINCT p.code FROM permissions p
             INNER JOIN role_permissions rp ON rp.permission_id = p.id
             INNER JOIN user_roles ur       ON ur.role_id = rp.role_id
             WHERE ur.user_id = ?",
        )?;
        let codes = stmt
            .query_map([user_id], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(codes)
    })
}

/// Fetch menu ids the user is allowed to see. Super admin sees everything.
pub fn load_visible_menu_codes(db: &Db, user: &AuthenticatedUser) -> AppResult<Vec<String>> {
    if user.is_super_admin {
        return db.with_conn(|c| {
            let mut stmt =
                c.prepare("SELECT code FROM menus WHERE status='active' AND visible=1")?;
            let codes = stmt
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(codes)
        });
    }
    db.with_conn(|c| {
        let mut stmt = c.prepare(
            "SELECT DISTINCT m.code FROM menus m
             INNER JOIN role_menus rm ON rm.menu_id = m.id
             INNER JOIN user_roles ur ON ur.role_id = rm.role_id
             WHERE ur.user_id = ? AND m.status='active' AND m.visible=1",
        )?;
        let codes = stmt
            .query_map([user.user_id.as_str()], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(codes)
    })
}

#[allow(dead_code)]
pub fn user_is_super_admin(db: &Db, user_id: &str) -> AppResult<bool> {
    db.with_conn(|c| {
        let n: i64 = c.query_row(
            "SELECT is_super_admin FROM users WHERE id = ?",
            [user_id],
            |r| r.get(0),
        )?;
        Ok(n != 0)
    })
}