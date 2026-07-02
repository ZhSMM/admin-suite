-- ============================================================================
-- V5__add_menu_title_key.sql
-- Adds a `title_key` column to `menus` so the sidebar can render titles
-- through vue-i18n instead of the raw DB string. Existing rows are mapped
-- to keys that already exist in the bundled en-US / zh-CN locales; new menus
-- can set their own keys from the menu management page.
-- ============================================================================

ALTER TABLE menus ADD COLUMN title_key TEXT;

-- Map every existing menu to an i18n key. Keys are case-sensitive and
-- must match what the bundled locales expose.
UPDATE menus SET title_key = 'menu.dashboard'  WHERE code = 'system';
UPDATE menus SET title_key = 'menu.users'      WHERE code = 'system.user';
UPDATE menus SET title_key = 'menu.roles'      WHERE code = 'system.role';
UPDATE menus SET title_key = 'menu.menus'      WHERE code = 'system.menu';
UPDATE menus SET title_key = 'menu.permissions' WHERE code = 'system.perm';
UPDATE menus SET title_key = 'menu.themes'     WHERE code = 'system.theme';
UPDATE menus SET title_key = 'menu.locales'    WHERE code = 'system.lang';
UPDATE menus SET title_key = 'menu.audit'      WHERE code = 'system.audit';

UPDATE menus SET title_key = 'menu.tools' WHERE code = 'tools';
UPDATE menus SET title_key = 'menu.tools.base'     WHERE code = 'tools.base';
UPDATE menus SET title_key = 'menu.tools.json'     WHERE code = 'tools.json';
UPDATE menus SET title_key = 'menu.tools.datetime' WHERE code = 'tools.datetime';
UPDATE menus SET title_key = 'menu.tools.sql'      WHERE code = 'tools.sql';
UPDATE menus SET title_key = 'menu.tools.encode'   WHERE code = 'tools.encode';
UPDATE menus SET title_key = 'menu.tools.hash'     WHERE code = 'tools.hash';
UPDATE menus SET title_key = 'menu.tools.generate' WHERE code = 'tools.generate';
UPDATE menus SET title_key = 'menu.tools.regex'    WHERE code = 'tools.regex';
UPDATE menus SET title_key = 'menu.tools.diff'     WHERE code = 'tools.diff';