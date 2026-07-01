-- ============================================================================
-- V1__init_schema.sql
-- Initial schema for the admin suite. All tables use TEXT primary keys (UUID)
-- so we never have to coordinate AUTOINCREMENT across migrations.
-- ============================================================================

-- ---------- Users ----------
CREATE TABLE users (
    id              TEXT PRIMARY KEY,
    username        TEXT NOT NULL UNIQUE,
    display_name    TEXT NOT NULL,
    password_hash   TEXT NOT NULL,                -- argon2id encoded hash
    email           TEXT,
    phone           TEXT,
    avatar          TEXT,
    status          TEXT NOT NULL DEFAULT 'active', -- active | disabled
    is_super_admin  INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    last_login_at   TEXT
);

CREATE INDEX idx_users_status ON users(status);

-- ---------- Roles ----------
CREATE TABLE roles (
    id          TEXT PRIMARY KEY,
    code        TEXT NOT NULL UNIQUE,             -- e.g. 'super_admin', 'manager'
    name        TEXT NOT NULL,                    -- display name, i18n key optional
    description TEXT,
    status      TEXT NOT NULL DEFAULT 'active',   -- active | disabled
    built_in    INTEGER NOT NULL DEFAULT 0,       -- 1 = cannot be deleted
    sort_order  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- ---------- Permissions (resource + action) ----------
CREATE TABLE permissions (
    id          TEXT PRIMARY KEY,
    code        TEXT NOT NULL UNIQUE,             -- 'resource:action', e.g. 'user:read'
    name        TEXT NOT NULL,                    -- human label
    resource    TEXT NOT NULL,                    -- 'user' | 'role' | 'menu' | ...
    action      TEXT NOT NULL,                    -- 'read' | 'write' | 'manage' | custom
    description TEXT,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_permissions_resource ON permissions(resource);

-- ---------- User <-> Role ----------
CREATE TABLE user_roles (
    user_id TEXT NOT NULL,
    role_id TEXT NOT NULL,
    PRIMARY KEY (user_id, role_id),
    FOREIGN KEY (user_id) REFERENCES users(id)  ON DELETE CASCADE,
    FOREIGN KEY (role_id) REFERENCES roles(id)  ON DELETE CASCADE
);

-- ---------- Role <-> Permission ----------
CREATE TABLE role_permissions (
    role_id       TEXT NOT NULL,
    permission_id TEXT NOT NULL,
    PRIMARY KEY (role_id, permission_id),
    FOREIGN KEY (role_id)       REFERENCES roles(id)       ON DELETE CASCADE,
    FOREIGN KEY (permission_id) REFERENCES permissions(id) ON DELETE CASCADE
);

-- ---------- Menus ----------
CREATE TABLE menus (
    id              TEXT PRIMARY KEY,
    parent_id       TEXT,
    code            TEXT NOT NULL UNIQUE,         -- 'system.user'
    title           TEXT NOT NULL,                -- default label (i18n key optional)
    path            TEXT,                          -- frontend route
    icon            TEXT,                          -- element-plus icon name
    component       TEXT,                          -- frontend view path
    sort_order      INTEGER NOT NULL DEFAULT 0,
    visible         INTEGER NOT NULL DEFAULT 1,
    status          TEXT NOT NULL DEFAULT 'active',
    menu_type       TEXT NOT NULL DEFAULT 'menu', -- menu | group | button
    permission_code TEXT,                          -- permission required to see this menu
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY (parent_id) REFERENCES menus(id) ON DELETE CASCADE
);

CREATE INDEX idx_menus_parent ON menus(parent_id);
CREATE INDEX idx_menus_status ON menus(status);

-- ---------- Role <-> Menu ----------
CREATE TABLE role_menus (
    role_id TEXT NOT NULL,
    menu_id TEXT NOT NULL,
    PRIMARY KEY (role_id, menu_id),
    FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE,
    FOREIGN KEY (menu_id) REFERENCES menus(id) ON DELETE CASCADE
);

-- ---------- Resource registry (themes + locales, user-importable) ----------
CREATE TABLE resources (
    id             TEXT PRIMARY KEY,
    resource_type  TEXT NOT NULL,                 -- 'theme' | 'locale'
    code           TEXT NOT NULL,                 -- 'dark' | 'zh-CN' | 'theme-ocean'
    name           TEXT NOT NULL,                 -- display name
    content        TEXT NOT NULL,                 -- JSON payload
    source         TEXT NOT NULL DEFAULT 'builtin', -- 'builtin' | 'imported' | 'user'
    built_in       INTEGER NOT NULL DEFAULT 0,
    active         INTEGER NOT NULL DEFAULT 0,    -- 1 = currently selected for this type
    created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (resource_type, code)
);

CREATE INDEX idx_resources_type ON resources(resource_type);

-- ---------- Audit log ----------
CREATE TABLE audit_log (
    id          TEXT PRIMARY KEY,
    actor_id    TEXT,                             -- user id (null for system)
    actor_name  TEXT,
    action      TEXT NOT NULL,                    -- 'user.create' / 'role.update' / ...
    resource    TEXT,                             -- affected resource type
    target_id   TEXT,                             -- affected resource id
    payload     TEXT,                             -- JSON snapshot (small)
    ip          TEXT,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_audit_created ON audit_log(created_at);
CREATE INDEX idx_audit_actor ON audit_log(actor_id);