-- ============================================================================
-- V3__seed_tools.sql
-- Adds the "Tools" menu group with three built-in utilities:
--   - Base Converter       (binary / octal / decimal / hex / base64)
--   - JSON Formatter       (pretty-print / minify / validate / tree view)
--   - Date & Time          (unix <-> ISO, timezones, format strings)
--
-- Tools are pure client-side utilities. We gate them behind a new
-- `tool:use` permission so the rest of the RBAC plumbing (route guards,
-- menu visibility) keeps working uniformly.
-- ============================================================================

-- ---------- Permission ----------
INSERT INTO permissions (id, code, name, resource, action, description) VALUES
    ('p_tool_use', 'tool:use', 'Use built-in tools', 'tool', 'use',
     'Access developer utilities (base convert / JSON / datetime)');

-- ---------- Role grants ----------
-- super_admin has *:* so it gets this for free. Grant admin + viewer explicitly
-- because tools are read-only-ish and useful for both roles.
INSERT OR IGNORE INTO role_permissions (role_id, permission_id) VALUES
    ('r_admin',  'p_tool_use'),
    ('r_viewer', 'p_tool_use');

-- ---------- Menus ----------
-- Top-level group placed AFTER 'm_system' (sort_order 100) so it shows up
-- below the management menu cluster.
INSERT INTO menus (id, parent_id, code, title, path, icon, component, sort_order, menu_type, permission_code) VALUES
    ('m_tools',         NULL,        'tools',         'Tools',           '/tools',          'Tools',    'Layout',         200, 'group', NULL),

    ('m_tool_base',     'm_tools',   'tools.base',     'Base Converter',  '/tools/base',     'Refresh',  'tools/BaseConvert',    210, 'menu',   'tool:use'),
    ('m_tool_json',     'm_tools',   'tools.json',     'JSON Formatter',  '/tools/json',     'Document', 'tools/JsonFormatter',  220, 'menu',   'tool:use'),
    ('m_tool_datetime', 'm_tools',   'tools.datetime', 'Date & Time',     '/tools/datetime', 'Clock',    'tools/DateTime',       230, 'menu',   'tool:use');

-- ---------- Role <-> Menu (so the menus are visible) ----------
-- super_admin already gets everything via its wildcard. Mirror the menus onto
-- r_admin and r_viewer explicitly so the tool entries appear in the sidebar.
INSERT OR IGNORE INTO role_menus (role_id, menu_id) VALUES
    ('r_admin',  'm_tools'),
    ('r_admin',  'm_tool_base'),
    ('r_admin',  'm_tool_json'),
    ('r_admin',  'm_tool_datetime'),
    ('r_viewer', 'm_tools'),
    ('r_viewer', 'm_tool_base'),
    ('r_viewer', 'm_tool_json'),
    ('r_viewer', 'm_tool_datetime');