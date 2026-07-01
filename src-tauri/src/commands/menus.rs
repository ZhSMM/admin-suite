use crate::auth::rbac::require_permission;
use crate::auth::session::SessionStore;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::models::{Menu, MenuCreate, MenuNode, MenuUpdate};
use rusqlite::params;
use uuid::Uuid;

fn row_to_menu(r: &rusqlite::Row) -> rusqlite::Result<Menu> {
    Ok(Menu {
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
}

fn select_all(db: &Db) -> AppResult<Vec<Menu>> {
    db.with_conn(|c| {
        let mut stmt = c.prepare(
            "SELECT id, parent_id, code, title, path, icon, component, sort_order,
                    visible, status, menu_type, permission_code, created_at, updated_at
             FROM menus ORDER BY sort_order, code",
        )?;
        let v = stmt
            .query_map([], row_to_menu)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(v)
    })
}

pub fn tree(db: &Db, sessions: &SessionStore, token: &str) -> AppResult<Vec<MenuNode>> {
    let user = sessions.lookup(token)?;
    require_permission(&user, "menu:read")?;
    let menus = select_all(db)?;
    Ok(build_tree(menus, None))
}

fn build_tree(menus: Vec<Menu>, parent: Option<String>) -> Vec<MenuNode> {
    let mut out = Vec::new();
    for m in menus.iter().filter(|m| m.parent_id == parent) {
        let children = build_tree(menus.clone(), Some(m.id.clone()));
        out.push(MenuNode {
            menu: m.clone(),
            children,
        });
    }
    out
}

pub fn get(db: &Db, sessions: &SessionStore, token: &str, id: &str) -> AppResult<Menu> {
    let user = sessions.lookup(token)?;
    require_permission(&user, "menu:read")?;
    db.with_conn(|c| {
        let mut stmt = c.prepare(
            "SELECT id, parent_id, code, title, path, icon, component, sort_order,
                    visible, status, menu_type, permission_code, created_at, updated_at
             FROM menus WHERE id = ?",
        )?;
        stmt.query_row([id], row_to_menu).map_err(AppError::Db)
    })
}

pub fn create(
    db: &Db,
    sessions: &SessionStore,
    token: &str,
    payload: MenuCreate,
) -> AppResult<Menu> {
    let user = sessions.lookup(token)?;
    require_permission(&user, "menu:write")?;
    if payload.code.trim().is_empty() {
        return Err(AppError::Validation("menu code is required".into()));
    }
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%fZ").to_string();
    let visible = payload.visible.unwrap_or(true);
    let status = payload.status.unwrap_or_else(|| "active".into());
    let sort_order = payload.sort_order.unwrap_or(0);
    let menu_type = payload.menu_type.unwrap_or_else(|| "menu".into());

    db.with_conn(|c| {
        c.execute(
            "INSERT INTO menus (id, parent_id, code, title, path, icon, component,
                                sort_order, visible, status, menu_type, permission_code,
                                created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                id,
                payload.parent_id,
                payload.code,
                payload.title,
                payload.path,
                payload.icon,
                payload.component,
                sort_order,
                visible as i64,
                status,
                menu_type,
                payload.permission_code,
                now,
                now
            ],
        )?;
        Ok(())
    })?;
    get(db, sessions, token, &id)
}

pub fn update(
    db: &Db,
    sessions: &SessionStore,
    token: &str,
    payload: MenuUpdate,
) -> AppResult<Menu> {
    let user = sessions.lookup(token)?;
    require_permission(&user, "menu:write")?;
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%fZ").to_string();

    db.with_tx(|tx| {
        let mut sets: Vec<String> = Vec::new();
        let mut binds: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(v) = &payload.title {
            sets.push("title = ?".into());
            binds.push(Box::new(v.clone()));
        }
        if let Some(v) = &payload.path {
            sets.push("path = ?".into());
            binds.push(Box::new(v.clone()));
        }
        if let Some(v) = &payload.icon {
            sets.push("icon = ?".into());
            binds.push(Box::new(v.clone()));
        }
        if let Some(v) = &payload.component {
            sets.push("component = ?".into());
            binds.push(Box::new(v.clone()));
        }
        if let Some(v) = payload.sort_order {
            sets.push("sort_order = ?".into());
            binds.push(Box::new(v));
        }
        if let Some(v) = payload.visible {
            sets.push("visible = ?".into());
            binds.push(Box::new(v as i64));
        }
        if let Some(v) = &payload.status {
            sets.push("status = ?".into());
            binds.push(Box::new(v.clone()));
        }
        if let Some(v) = &payload.menu_type {
            sets.push("menu_type = ?".into());
            binds.push(Box::new(v.clone()));
        }
        if let Some(v) = &payload.permission_code {
            sets.push("permission_code = ?".into());
            binds.push(Box::new(v.clone()));
        }
        if let Some(v) = &payload.parent_id {
            sets.push("parent_id = ?".into());
            binds.push(Box::new(v.clone()));
        }

        if !sets.is_empty() {
            sets.push("updated_at = ?".into());
            binds.push(Box::new(now));
            let sql = format!("UPDATE menus SET {} WHERE id = ?", sets.join(", "));
            binds.push(Box::new(payload.id.clone()));
            let bind_refs: Vec<&dyn rusqlite::ToSql> = binds.iter().map(|b| b.as_ref()).collect();
            tx.execute(&sql, rusqlite::params_from_iter(bind_refs))?;
        }
        Ok(())
    })?;
    get(db, sessions, token, &payload.id)
}

pub fn delete(db: &Db, sessions: &SessionStore, token: &str, id: &str) -> AppResult<()> {
    let user = sessions.lookup(token)?;
    require_permission(&user, "menu:delete")?;
    db.with_conn(|c| {
        c.execute("DELETE FROM menus WHERE id = ?", params![id])?;
        Ok(())
    })
}