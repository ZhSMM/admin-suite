-- V7 — App settings, backups, command palette support.
-- All three new features read/write the same key/value table, so we add it once
-- here and let the rest of the schema grow with later migrations.

CREATE TABLE IF NOT EXISTS app_state (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Default settings keys, all kept as TEXT (parsers coerce to int / bool / etc).
-- Centralizing them here avoids silently reading `NULL` from the table on
-- first run, which would force every consumer to handle missing rows.
INSERT OR IGNORE INTO app_state (key, value, updated_at) VALUES
    ('session.timeout_minutes',  '480',  strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    ('auth.password_min_length', '6',    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    ('auth.login_max_failures',  '10',   strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    ('auth.lockout_minutes',     '15',   strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    ('backup.auto_on_start',     'true', strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    ('backup.keep_count',        '10',   strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    ('ui.default_theme',         'light', strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    ('ui.default_locale',        'zh-CN', strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    ('ui.command_palette',       'true', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));

-- New permissions for the three new feature pages.
INSERT OR IGNORE INTO permissions (id, code, name, resource, action) VALUES
    ('p_settings_manage', 'settings:manage', 'Manage global settings', 'settings', 'manage'),
    ('p_backup_manage',   'backup:manage',   'Manage backups',         'backup',   'manage'),
    ('p_backup_restore',  'backup:restore',  'Restore from backup',    'backup',   'restore');

-- Grant every new permission to super-admin (r_super_admin).
INSERT OR IGNORE INTO role_permissions (role_id, permission_id)
    SELECT 'r_super_admin', id FROM permissions WHERE code IN
        ('settings:manage', 'backup:manage', 'backup:restore');

-- Three new admin pages: Settings, Backups, plus a "palette" placeholder we
-- don't render as a route but flag here so it's discoverable in Menus management.
INSERT OR IGNORE INTO menus (id, parent_id, code, title, title_key, path,
                             icon, sort_order, visible, status, menu_type,
                             permission_code)
VALUES
    ('m_settings', 'm_system', 'system.settings', 'Settings', 'menu.settings',
        '/system/settings', 'Setting', 60, 1, 'active', 'menu',
        'settings:manage'),
    ('m_backups',  'm_system', 'system.backups',  'Backups',  'menu.backups',
        '/system/backups',  'Folder',  70, 1, 'active', 'menu',
        'backup:manage');

-- Auto-grant to super-admin so the pages appear immediately.
INSERT OR IGNORE INTO role_menus (role_id, menu_id)
    SELECT 'r_super_admin', id FROM menus WHERE code IN ('system.settings', 'system.backups');