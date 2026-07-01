use crate::auth::rbac::require_permission;
use crate::auth::session::SessionStore;
use crate::db::Db;
use crate::error::AppResult;
use crate::models::Permission;

pub fn list(db: &Db, sessions: &SessionStore, token: &str) -> AppResult<Vec<Permission>> {
    let user = sessions.lookup(token)?;
    require_permission(&user, "permission:read")?;
    db.with_conn(|c| {
        let mut stmt = c.prepare(
            "SELECT id, code, name, resource, action, description, created_at
             FROM permissions ORDER BY resource, action",
        )?;
        let v = stmt
            .query_map([], |r| {
                Ok(Permission {
                    id: r.get(0)?,
                    code: r.get(1)?,
                    name: r.get(2)?,
                    resource: r.get(3)?,
                    action: r.get(4)?,
                    description: r.get(5)?,
                    created_at: r.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(v)
    })
}