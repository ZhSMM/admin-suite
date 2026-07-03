-- ============================================================================
-- V6__seed_string_crypto.sql
-- Adds two more client-side developer tools to the Tools menu:
--   - String Converter:  Unicode / HTML / URL / Normalize / ASCII / JS / CSS
--                        escape + decode — a much-expanded version of the
--                        old URL/HTML encoder.
--   - Crypto:           AES-GCM / AES-CBC / RSA-OAEP via WebCrypto, plus
--                        legacy RC4 and classical ciphers (Caesar, Vigenère,
--                        XOR) implemented in pure JS.
-- ============================================================================

INSERT INTO menus (id, parent_id, code, title, title_key, path, icon, component, sort_order, menu_type, permission_code) VALUES
    ('m_tool_string', 'm_tools', 'tools.string',  'String Converter',  'menu.tools.string',  '/tools/string',  'DocumentCopy', 'tools/StringConverter', 295, 'menu', 'tool:use'),
    ('m_tool_crypto', 'm_tools', 'tools.crypto',  'Crypto',            'menu.tools.crypto',  '/tools/crypto',  'Lock',        'tools/Crypto',         300, 'menu', 'tool:use');

-- Mirror the new entries onto admin + viewer roles.
INSERT OR IGNORE INTO role_menus (role_id, menu_id) VALUES
    ('r_admin',  'm_tool_string'),
    ('r_admin',  'm_tool_crypto'),
    ('r_viewer', 'm_tool_string'),
    ('r_viewer', 'm_tool_crypto');