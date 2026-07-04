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
    let resource_filter = query.resource.unwrap_or_default();
    let payload_filter = query.payload_search.unwrap_or_default();
    let from_filter = query.from.unwrap_or_default();
    let to_filter = query.to.unwrap_or_default();

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
        if !resource_filter.is_empty() {
            where_clauses.push(format!("resource = ?{}", binds.len() + 1));
            binds.push(resource_filter);
        }
        if !payload_filter.is_empty() {
            where_clauses.push(format!("payload LIKE ?{}", binds.len() + 1));
            binds.push(format!("%{}%", payload_filter));
        }
        if !from_filter.is_empty() {
            where_clauses.push(format!("created_at >= ?{}", binds.len() + 1));
            binds.push(from_filter);
        }
        if !to_filter.is_empty() {
            where_clauses.push(format!("created_at <= ?{}", binds.len() + 1));
            binds.push(to_filter);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::session::{AuthenticatedUser, SessionStore};
    use crate::db::migrate;
    use std::path::PathBuf;

    fn temp_db() -> std::sync::Arc<Db> {
        let mut p = std::env::temp_dir();
        p.push(format!("admin-suite-audit-test-{}.sqlite", uuid::Uuid::new_v4()));
        let _ = std::fs::remove_file(&p);
        Db::open(&p).expect("open test db")
    }

    fn session_with(db: &Db) -> (SessionStore, String) {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations");
        let _ = migrate::run_migrations(db, &dir);
        let s = SessionStore::new(60);
        let token = s
            .issue(AuthenticatedUser {
                user_id: "u_test".into(),
                username: "tester".into(),
                is_super_admin: true,
                permission_codes: vec!["*:*".into()],
                role_ids: vec![],
                expires_at: chrono::Utc::now() + chrono::Duration::minutes(5),
            })
            .unwrap();
        (s, token)
    }

    fn seed(db: &Db) {
        // Three audit rows with predictable fields.
        write(db, Some("u_alice"), Some("alice"), "user.create", Some("user"), Some("u1"),
              Some(r#"{"username":"alice"}"#)).unwrap();
        write(db, Some("u_bob"), Some("bob"), "user.delete", Some("user"), Some("u2"),
              Some(r#"{"username":"bob"}"#)).unwrap();
        write(db, Some("u_alice"), Some("alice"), "role.update", Some("role"), Some("r1"),
              Some(r#"{"code":"viewer"}"#)).unwrap();
    }

    #[test]
    fn filter_by_actor() {
        let db = temp_db();
        let (s, tok) = session_with(&db);
        seed(&db);
        let r = list(
            &db, &s, &tok,
            AuditQuery {
                actor_id: Some("u_alice".into()),
                action: None,
                resource: None,
                payload_search: None,
                from: None,
                to: None,
                page: Some(1),
                page_size: Some(50),
            },
        )
        .unwrap();
        assert_eq!(r.total, 2);
        assert!(r.items.iter().all(|i| i.actor_id.as_deref() == Some("u_alice")));
    }

    #[test]
    fn filter_by_action_like() {
        let db = temp_db();
        let (s, tok) = session_with(&db);
        seed(&db);
        let r = list(
            &db, &s, &tok,
            AuditQuery {
                actor_id: None,
                action: Some("user".into()),
                resource: None,
                payload_search: None,
                from: None,
                to: None,
                page: Some(1),
                page_size: Some(50),
            },
        )
        .unwrap();
        assert_eq!(r.total, 2, "expected 2 user.* entries");
    }

    #[test]
    fn filter_by_resource_and_payload() {
        let db = temp_db();
        let (s, tok) = session_with(&db);
        seed(&db);
        let r = list(
            &db, &s, &tok,
            AuditQuery {
                actor_id: None,
                action: None,
                resource: Some("role".into()),
                payload_search: Some("viewer".into()),
                from: None,
                to: None,
                page: Some(1),
                page_size: Some(50),
            },
        )
        .unwrap();
        assert_eq!(r.total, 1);
        assert_eq!(r.items[0].action, "role.update");
    }

    #[test]
    fn filter_by_time_window() {
        let db = temp_db();
        let (s, tok) = session_with(&db);
        seed(&db);
        // Far-future window — no rows.
        let r = list(
            &db, &s, &tok,
            AuditQuery {
                actor_id: None,
                action: None,
                resource: None,
                payload_search: None,
                from: Some("9999-01-01T00:00:00Z".into()),
                to: Some("9999-12-31T23:59:59Z".into()),
                page: Some(1),
                page_size: Some(50),
            },
        )
        .unwrap();
        assert_eq!(r.total, 0);
        // Far-past window — all 3 rows.
        let r = list(
            &db, &s, &tok,
            AuditQuery {
                actor_id: None,
                action: None,
                resource: None,
                payload_search: None,
                from: Some("1000-01-01T00:00:00Z".into()),
                to: None,
                page: Some(1),
                page_size: Some(50),
            },
        )
        .unwrap();
        assert_eq!(r.total, 3);
    }

    #[test]
    fn filters_combine_with_and() {
        let db = temp_db();
        let (s, tok) = session_with(&db);
        seed(&db);
        let r = list(
            &db, &s, &tok,
            AuditQuery {
                actor_id: Some("u_alice".into()),
                action: Some("user.create".into()),
                resource: None,
                payload_search: None,
                from: None,
                to: None,
                page: Some(1),
                page_size: Some(50),
            },
        )
        .unwrap();
        assert_eq!(r.total, 1);
        assert_eq!(r.items[0].action, "user.create");
    }
}