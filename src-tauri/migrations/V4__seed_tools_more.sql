-- ============================================================================
-- V4__seed_tools_more.sql
-- Adds 6 more client-side developer tools to the Tools menu:
--   - SQL Formatter       (pretty-print / minify SQL)
--   - URL / HTML Encode   (URL & HTML entity encode/decode)
--   - Hash Calculator     (MD5 / SHA-1 / SHA-256 / SHA-384 / SHA-512)
--   - UUID & Password     (UUIDv4 / v7, random passwords)
--   - Regex Tester        (live regex match with capture groups)
--   - Diff                (text diff: side-by-side or unified)
-- ============================================================================

INSERT INTO menus (id, parent_id, code, title, path, icon, component, sort_order, menu_type, permission_code) VALUES
    ('m_tool_sql',      'm_tools', 'tools.sql',      'SQL Formatter',    '/tools/sql',      'Files',      'tools/SqlFormatter',    240, 'menu', 'tool:use'),
    ('m_tool_encode',   'm_tools', 'tools.encode',   'URL / HTML',       '/tools/encode',   'Connection', 'tools/Encode',          250, 'menu', 'tool:use'),
    ('m_tool_hash',     'm_tools', 'tools.hash',     'Hash Calculator',  '/tools/hash',     'Histogram',  'tools/Hash',            260, 'menu', 'tool:use'),
    ('m_tool_generate', 'm_tools', 'tools.generate', 'UUID & Password',  '/tools/generate', 'Key',        'tools/Generate',        270, 'menu', 'tool:use'),
    ('m_tool_regex',    'm_tools', 'tools.regex',    'Regex Tester',     '/tools/regex',    'Search',     'tools/Regex',           280, 'menu', 'tool:use'),
    ('m_tool_diff',     'm_tools', 'tools.diff',     'Diff',             '/tools/diff',     'View',       'tools/Diff',            290, 'menu', 'tool:use');

-- Mirror the new entries onto admin + viewer roles.
INSERT OR IGNORE INTO role_menus (role_id, menu_id) VALUES
    ('r_admin',  'm_tool_sql'),
    ('r_admin',  'm_tool_encode'),
    ('r_admin',  'm_tool_hash'),
    ('r_admin',  'm_tool_generate'),
    ('r_admin',  'm_tool_regex'),
    ('r_admin',  'm_tool_diff'),
    ('r_viewer', 'm_tool_sql'),
    ('r_viewer', 'm_tool_encode'),
    ('r_viewer', 'm_tool_hash'),
    ('r_viewer', 'm_tool_generate'),
    ('r_viewer', 'm_tool_regex'),
    ('r_viewer', 'm_tool_diff');