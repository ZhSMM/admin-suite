-- ============================================================================
-- V2__seed_acl.sql
-- Default permissions, the built-in super_admin role, and the management
-- menus that ship with the app. The first-run admin user is created in
-- Rust (after migrations) so the password hash can be Argon2id-salted.
-- ============================================================================

-- ---------- Permissions ----------
-- Convention: <resource>:<action>.  Wildcard '*:*' means "everything".
INSERT INTO permissions (id, code, name, resource, action, description) VALUES
    ('p_all',          '*:*',           'All permissions', '*', '*', 'Wildcard — bypasses all checks'),

    -- user management
    ('p_user_read',    'user:read',     'View users',    'user', 'read',   'List & inspect users'),
    ('p_user_write',   'user:write',    'Create/Edit user','user', 'write', 'Create or update users'),
    ('p_user_delete',  'user:delete',   'Delete user',   'user', 'delete', 'Remove users'),
    ('p_user_manage',  'user:manage',   'Manage users',  'user', 'manage', 'Full user administration'),

    -- role management
    ('p_role_read',    'role:read',     'View roles',    'role', 'read',   'List & inspect roles'),
    ('p_role_write',   'role:write',    'Create/Edit role','role', 'write', 'Create or update roles'),
    ('p_role_delete',  'role:delete',   'Delete role',   'role', 'delete', 'Remove roles'),
    ('p_role_manage',  'role:manage',   'Manage roles',  'role', 'manage', 'Full role administration'),

    -- menu management
    ('p_menu_read',    'menu:read',     'View menus',    'menu', 'read',   'List & inspect menus'),
    ('p_menu_write',   'menu:write',    'Create/Edit menu','menu', 'write', 'Create or update menus'),
    ('p_menu_delete',  'menu:delete',   'Delete menu',   'menu', 'delete', 'Remove menus'),
    ('p_menu_manage',  'menu:manage',   'Manage menus',  'menu', 'manage', 'Full menu administration'),

    -- permission management
    ('p_perm_read',    'permission:read',   'View permissions', 'permission', 'read',   'List permissions'),
    ('p_perm_write',   'permission:write',  'Edit permissions','permission', 'write',  'Edit permission grants'),
    ('p_perm_manage',  'permission:manage', 'Manage permissions','permission', 'manage', 'Full permission admin'),

    -- resource (theme / i18n) management
    ('p_theme_manage', 'theme:manage',  'Manage themes',  'theme', 'manage', 'Import / edit / activate themes'),
    ('p_locale_manage','locale:manage', 'Manage locales', 'locale','manage', 'Import / edit / activate locales'),

    -- audit
    ('p_audit_read',   'audit:read',    'View audit log', 'audit', 'read',   'Read audit trail'),
    ('p_audit_export', 'audit:export',  'Export audit log','audit','export', 'Export audit trail');

-- ---------- Roles ----------
-- 'super_admin' is built-in and has the wildcard.
INSERT INTO roles (id, code, name, description, built_in, sort_order) VALUES
    ('r_super_admin', 'super_admin', 'Super Administrator', 'Has every permission. Cannot be deleted.', 1, 0),
    ('r_admin',       'admin',       'Administrator',       'Day-to-day administrator. No wildcard.',       1, 10),
    ('r_viewer',      'viewer',      'Read-only Viewer',    'Can read all admin data but cannot modify.',    1, 90);

-- ---------- Role <-> Permission grants ----------
INSERT INTO role_permissions (role_id, permission_id) VALUES
    -- super_admin gets the wildcard; everything else is implicit.
    ('r_super_admin', 'p_all'),

    -- admin gets everything except user/role deletion and audit export
    ('r_admin', 'p_user_read'),
    ('r_admin', 'p_user_write'),
    ('r_admin', 'p_role_read'),
    ('r_admin', 'p_role_write'),
    ('r_admin', 'p_menu_read'),
    ('r_admin', 'p_menu_write'),
    ('r_admin', 'p_perm_read'),
    ('r_admin', 'p_theme_manage'),
    ('r_admin', 'p_locale_manage'),
    ('r_admin', 'p_audit_read'),

    -- viewer is read-only
    ('r_viewer', 'p_user_read'),
    ('r_viewer', 'p_role_read'),
    ('r_viewer', 'p_menu_read'),
    ('r_viewer', 'p_perm_read'),
    ('r_viewer', 'p_audit_read');

-- ---------- Menus ----------
-- Top-level: System (under which all management menus live).
--   System
--     ├─ Users          (requires user:read)
--     ├─ Roles          (requires role:read)
--     ├─ Menus          (requires menu:read)
--     ├─ Permissions    (requires permission:read)
--     ├─ Themes         (requires theme:manage)
--     ├─ Languages      (requires locale:manage)
--     └─ Audit Log      (requires audit:read)
INSERT INTO menus (id, parent_id, code, title, path, icon, component, sort_order, menu_type, permission_code) VALUES
    ('m_system',       NULL,         'system',       'System',         '/system',         'Setting',  'Layout',       100, 'group', NULL),

    ('m_user',         'm_system',   'system.user',  'Users',          '/system/users',   'User',     'admin/Users',  110, 'menu',   'user:read'),
    ('m_role',         'm_system',   'system.role',  'Roles',          '/system/roles',   'UserFilled','admin/Roles', 120, 'menu',   'role:read'),
    ('m_menu',         'm_system',   'system.menu',  'Menus',          '/system/menus',   'Menu',     'admin/Menus',  130, 'menu',   'menu:read'),
    ('m_permission',   'm_system',   'system.perm',  'Permissions',    '/system/permissions','Lock', 'admin/Permissions', 140, 'menu', 'permission:read'),
    ('m_theme',        'm_system',   'system.theme', 'Themes',         '/system/themes',  'Brush',    'admin/Themes', 150, 'menu',   'theme:manage'),
    ('m_locale',       'm_system',   'system.lang',  'Languages',      '/system/locales', 'ChatDotRound','admin/Locales', 160, 'menu', 'locale:manage'),
    ('m_audit',        'm_system',   'system.audit', 'Audit Log',      '/system/audit',   'Document', 'admin/Audit',  170, 'menu',   'audit:read');

-- ---------- Default themes (registered as resources so they can be selected at runtime) ----------
INSERT INTO resources (id, resource_type, code, name, content, source, built_in, active) VALUES
    ('r_theme_light',
     'theme',
     'light',
     'Light',
     '{
        "id": "light",
        "label": "Light",
        "isDark": false,
        "tokens": {
          "--bg-primary": "#ffffff",
          "--bg-secondary": "#f5f7fa",
          "--bg-sidebar": "#001529",
          "--bg-sidebar-text": "#ffffff",
          "--text-primary": "#1f2329",
          "--text-secondary": "#646a73",
          "--text-inverse": "#ffffff",
          "--border-color": "#e5e6eb",
          "--primary-color": "#409eff",
          "--success-color": "#67c23a",
          "--warning-color": "#e6a23c",
          "--danger-color": "#f56c6c",
          "--info-color": "#909399",
          "--shadow-sm": "0 1px 2px rgba(0,0,0,0.06)",
          "--shadow-md": "0 2px 8px rgba(0,0,0,0.10)"
        }
     }',
     'builtin', 1, 1),

    ('r_theme_dark',
     'theme',
     'dark',
     'Dark',
     '{
        "id": "dark",
        "label": "Dark",
        "isDark": true,
        "tokens": {
          "--bg-primary": "#1f1f1f",
          "--bg-secondary": "#141414",
          "--bg-sidebar": "#000000",
          "--bg-sidebar-text": "#e5e6eb",
          "--text-primary": "#e5e6eb",
          "--text-secondary": "#a3a6ad",
          "--text-inverse": "#1f2329",
          "--border-color": "#3a3a3a",
          "--primary-color": "#409eff",
          "--success-color": "#67c23a",
          "--warning-color": "#e6a23c",
          "--danger-color": "#f56c6c",
          "--info-color": "#909399",
          "--shadow-sm": "0 1px 2px rgba(0,0,0,0.50)",
          "--shadow-md": "0 2px 8px rgba(0,0,0,0.65)"
        }
     }',
     'builtin', 1, 0),

    ('r_theme_ocean',
     'theme',
     'ocean',
     'Ocean',
     '{
        "id": "ocean",
        "label": "Ocean",
        "isDark": false,
        "tokens": {
          "--bg-primary": "#ffffff",
          "--bg-secondary": "#eef6fb",
          "--bg-sidebar": "#0c4a6e",
          "--bg-sidebar-text": "#e0f2fe",
          "--text-primary": "#0c4a6e",
          "--text-secondary": "#475569",
          "--text-inverse": "#ffffff",
          "--border-color": "#cbd5e1",
          "--primary-color": "#0284c7",
          "--success-color": "#10b981",
          "--warning-color": "#f59e0b",
          "--danger-color": "#ef4444",
          "--info-color": "#64748b",
          "--shadow-sm": "0 1px 2px rgba(8,47,73,0.08)",
          "--shadow-md": "0 4px 12px rgba(8,47,73,0.15)"
        }
     }',
     'builtin', 1, 0);

-- ---------- Default locales ----------
INSERT INTO resources (id, resource_type, code, name, content, source, built_in, active) VALUES
    ('r_locale_en',
     'locale',
     'en-US',
     'English',
     '{
        "id": "en-US",
        "label": "English",
        "messages": {
          "common.ok": "OK",
          "common.cancel": "Cancel",
          "common.save": "Save",
          "common.delete": "Delete",
          "common.edit": "Edit",
          "common.create": "Create",
          "common.search": "Search",
          "common.refresh": "Refresh",
          "common.confirmDelete": "Are you sure you want to delete this item?",
          "common.yes": "Yes",
          "common.no": "No",
          "common.loading": "Loading...",
          "common.success": "Success",
          "common.failed": "Failed",
          "auth.login": "Login",
          "auth.logout": "Logout",
          "auth.username": "Username",
          "auth.password": "Password",
          "auth.loginSuccess": "Login successful",
          "auth.invalidCredentials": "Invalid username or password",
          "menu.dashboard": "Dashboard",
          "menu.system": "System",
          "menu.users": "Users",
          "menu.roles": "Roles",
          "menu.menus": "Menus",
          "menu.permissions": "Permissions",
          "menu.themes": "Themes",
          "menu.locales": "Languages",
          "menu.audit": "Audit Log"
        }
     }',
     'builtin', 1, 1),

    ('r_locale_zh',
     'locale',
     'zh-CN',
     '简体中文',
     '{
        "id": "zh-CN",
        "label": "简体中文",
        "messages": {
          "common.ok": "确定",
          "common.cancel": "取消",
          "common.save": "保存",
          "common.delete": "删除",
          "common.edit": "编辑",
          "common.create": "新建",
          "common.search": "搜索",
          "common.refresh": "刷新",
          "common.confirmDelete": "确定要删除这条记录吗?",
          "common.yes": "是",
          "common.no": "否",
          "common.loading": "加载中...",
          "common.success": "操作成功",
          "common.failed": "操作失败",
          "auth.login": "登录",
          "auth.logout": "退出登录",
          "auth.username": "用户名",
          "auth.password": "密码",
          "auth.loginSuccess": "登录成功",
          "auth.invalidCredentials": "用户名或密码错误",
          "menu.dashboard": "仪表盘",
          "menu.system": "系统管理",
          "menu.users": "用户管理",
          "menu.roles": "角色管理",
          "menu.menus": "菜单管理",
          "menu.permissions": "权限管理",
          "menu.themes": "主题管理",
          "menu.locales": "语言管理",
          "menu.audit": "审计日志"
        }
     }',
     'builtin', 1, 0);