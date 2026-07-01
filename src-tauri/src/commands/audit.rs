use crate::auth::rbac::require_permission;
use crate::auth::session::SessionStore;
use crate::db::Db;
use crate::error::AppResult;
use crate::models::{AuditLogEntry, AuditQuery};
use rusqlite::params;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct AuditListResult {
    pub items: Vec<AuditLogEntry>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

pub fn list(
    db: &Db,
    sessions: &SessionStore,
    token: &str,
    query: AuditQuery,
) -> AppResult<AuditListResult> {
    let user = sessions.lookup(token)?;
    require_permission(&user, "audit:read")?;
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(50).clamp(1, 500);
    let offset = (page - 1) * page_size;

    let actor_filter = query.actor_id.unwrap_or_default();
    let action_filter = query.action.unwrap_or_default();

    db.with_conn(|c| {
        let mut where_clauses: Vec<String> = Vec::new();
        let mut binds: Vec<String> = Vec::new();
        if !actor_filter.is_empty() {
            where_clauses.push(format!("actor_id = ?{}", binds.len() + 1));
            binds.push(actor_filter);
        }
        if !action_filter.is_empty() {
            where_clauses.push(format!("action LIKE ?{}", binds.len() + 1));
            binds.push(format!("%{}%", action_filter));
        }
        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };

        let total: i64 = {
            let sql = format!("SELECT COUNT(*) FROM audit_log {}", where_sql);
            let mut stmt = c.prepare(&sql)?;
            let mut rows = stmt.query(rusqlite::params_from_iter(binds.iter()))?;
            rows.next()?.unwrap().get(0)?
        };

        let list_sql = format!(
            "SELECT id, actor_id, actor_name, action, resource, target_id, payload, ip, created_at
             FROM audit_log {} ORDER BY created_at DESC LIMIT ? OFFSET ?",
            where_sql
        );
        let mut stmt = c.prepare(&list_sql)?;
        let mut all_binds = binds.clone();
        all_binds.push(page_size.to_string());
        all_binds.push(offset.to_string());
        let items = stmt
            .query_map(rusqlite::params_from_iter(all_binds.iter()), |r| {
                Ok(AuditLogEntry {
                    id: r.get(0)?,
                    actor_id: r.get(1)?,
                    actor_name: r.get(2)?,
                    action: r.get(3)?,
                    resource: r.get(4)?,
                    target_id: r.get(5)?,
                    payload: r.get(6)?,
                    ip: r.get(7)?,
                    created_at: r.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(AuditListResult {
            items,
            total,
            page,
            page_size,
        })
    })
}

#[allow(dead_code)]
pub fn write(
    db: &Db,
    actor_id: Option<&str>,
    actor_name: Option<&str>,
    action: &str,
    resource: Option<&str>,
    target_id: Option<&str>,
    payload: Option<&str>,
) -> AppResult<()> {
    let id = Uuid::new_v4().to_string();
    db.with_conn(|c| {
        c.execute(
            "INSERT INTO audit_log (id, actor_id, actor_name, action, resource, target_id, payload)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![id, actor_id, actor_name, action, resource, target_id, payload],
        )?;
        Ok(())
    })
}