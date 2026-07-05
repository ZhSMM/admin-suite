-- V9 — IPC performance monitoring + crash diagnostics + auto-update.
--
-- Three new admin pages under the system group:
--   - /system/monitoring   (IPC perf metrics; audit:read analogue)
--   - /system/diagnostics  (Crash logs)
--   - /system/updater      (Auto-update control panel; updater:check)
--
-- All three are read-mostly, so each gets its own permission code rather than
-- reusing `system:read` — finer-grained access lets ops restrict the crash log
-- (which may contain sensitive payload snippets) without locking out the
-- performance metrics view.

INSERT OR IGNORE INTO permissions (id, code, name, resource, action) VALUES
    ('p_monitoring_read',  'monitoring:read',  'View IPC performance metrics',  'monitoring', 'read'),
    ('p_diagnostics_read', 'diagnostics:read', 'View crash diagnostics',        'diagnostics', 'read'),
    ('p_diagnostics_clear','diagnostics:clear','Clear crash logs',              'diagnostics', 'clear'),
    ('p_updater_check',    'updater:check',    'Check for app updates',         'updater', 'check'),
    ('p_updater_apply',    'updater:apply',    'Install app updates',           'updater', 'apply');

-- Super-admin gets everything.
INSERT OR IGNORE INTO role_permissions (role_id, permission_id)
    SELECT 'r_super_admin', id FROM permissions WHERE code IN
        ('monitoring:read', 'diagnostics:read', 'diagnostics:clear',
         'updater:check', 'updater:apply');

-- New menu entries parented under the system group (m_system).  Sort order is
-- interleaved with existing system pages so they don't all clump at the end.
INSERT OR IGNORE INTO menus (id, parent_id, code, title, title_key, path,
                             icon, sort_order, visible, status, menu_type,
                             permission_code)
VALUES
    ('m_monitoring',  'm_system', 'system.monitoring',  'Monitoring',  'menu.monitoring',
        '/system/monitoring',  'DataLine', 65, 1, 'active', 'menu', 'monitoring:read'),
    ('m_diagnostics', 'm_system', 'system.diagnostics', 'Diagnostics', 'menu.diagnostics',
        '/system/diagnostics', 'WarningFilled', 75, 1, 'active', 'menu', 'diagnostics:read'),
    ('m_updater',     'm_system', 'system.updater',     'Updater',     'menu.updater',
        '/system/updater',     'Download', 80, 1, 'active', 'menu', 'updater:check');

-- Auto-grant to super-admin so the pages appear immediately.
INSERT OR IGNORE INTO role_menus (role_id, menu_id)
    SELECT 'r_super_admin', id FROM menus WHERE code IN
        ('system.monitoring', 'system.diagnostics', 'system.updater');