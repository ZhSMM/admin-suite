-- v0.7.1 — Grant r_admin the LLM permissions so the default `admin` user
-- (the seeded `admin / admin123`) can list providers & models, configure
-- them, and chat. Before this migration, r_admin only had domain-read
-- perms; LLM flows required llm:manage which only super_admin carried.
-- That's why a freshly-deployed user could see the AI menu but every
-- dropdown was empty (llm_providers_list → forbidden).
INSERT OR IGNORE INTO role_permissions (role_id, permission_id)
SELECT 'r_admin', p.id
FROM permissions p
WHERE p.code IN ('llm:use', 'llm:manage', 'llm:usage:read', 'llm:budget:manage');
