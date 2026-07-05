-- V10 — LLM integration (v0.6.0)
--
-- Five new tables + permissions + menus.
-- API keys are stored as AES-GCM ciphertext in `llm_providers.api_key_enc`
-- (see commands::crypto for the master-key derivation); the column is
-- always NULL on read so the key never leaves the Rust process in plaintext.

CREATE TABLE IF NOT EXISTS llm_providers (
    id              TEXT PRIMARY KEY,
    code            TEXT UNIQUE NOT NULL,
    name            TEXT NOT NULL,
    kind            TEXT NOT NULL,                -- 'openai_compat' | 'anthropic' | 'google' | 'custom'
    base_url        TEXT NOT NULL,
    auth_type       TEXT NOT NULL,                -- 'bearer' | 'header' | 'none'
    auth_header     TEXT,
    api_key_enc     BLOB,
    settings_json   TEXT NOT NULL DEFAULT '{}',
    default_model_id TEXT,
    enabled         INTEGER NOT NULL DEFAULT 1,
    sort_order      INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS llm_models (
    id              TEXT PRIMARY KEY,
    provider_id     TEXT NOT NULL REFERENCES llm_providers(id) ON DELETE CASCADE,
    code            TEXT NOT NULL,
    display_name    TEXT NOT NULL,
    capabilities    TEXT NOT NULL DEFAULT '[]',  -- JSON array
    context_window  INTEGER NOT NULL DEFAULT 4096,
    max_output      INTEGER NOT NULL DEFAULT 2048,
    pricing_json    TEXT NOT NULL DEFAULT '{}',
    enabled         INTEGER NOT NULL DEFAULT 1,
    sort_order      INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    UNIQUE(provider_id, code)
);
CREATE INDEX IF NOT EXISTS idx_llm_models_provider ON llm_models(provider_id);

CREATE TABLE IF NOT EXISTS llm_usage (
    id                  TEXT PRIMARY KEY,
    ts_unix_ms          INTEGER NOT NULL,
    user_id             TEXT NOT NULL,
    provider_id         TEXT NOT NULL,
    model_id            TEXT NOT NULL,
    capability          TEXT NOT NULL,
    prompt_tokens       INTEGER NOT NULL DEFAULT 0,
    completion_tokens   INTEGER NOT NULL DEFAULT 0,
    total_tokens        INTEGER NOT NULL DEFAULT 0,
    cost_usd            REAL NOT NULL DEFAULT 0,
    latency_ms          INTEGER NOT NULL,
    success             INTEGER NOT NULL,
    error               TEXT,
    request_id          TEXT NOT NULL,
    UNIQUE(request_id)
);
CREATE INDEX IF NOT EXISTS idx_llm_usage_user_ts ON llm_usage(user_id, ts_unix_ms);
CREATE INDEX IF NOT EXISTS idx_llm_usage_provider_ts ON llm_usage(provider_id, ts_unix_ms);

CREATE TABLE IF NOT EXISTS llm_budgets (
    id              TEXT PRIMARY KEY,
    role_id         TEXT NOT NULL,
    period          TEXT NOT NULL,                -- 'monthly' / 'weekly'
    max_cost_usd    REAL NOT NULL DEFAULT 0,
    max_tokens      INTEGER NOT NULL DEFAULT 0,
    enabled         INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    UNIQUE(role_id, period)
);

CREATE TABLE IF NOT EXISTS llm_user_defaults (
    user_id         TEXT NOT NULL,
    capability      TEXT NOT NULL,
    model_id        TEXT NOT NULL,
    PRIMARY KEY (user_id, capability)
);

-- Fallback engine config (single-row table; CHECK id=1 enforces it).
-- Actual download progress / server health live in
-- <data_dir>/llm/fallback_state.json so we don't write the DB on every
-- progress tick.
CREATE TABLE IF NOT EXISTS llm_fallback_config (
    id                  INTEGER PRIMARY KEY DEFAULT 1,
    enabled             INTEGER NOT NULL DEFAULT 0,
    model_id            TEXT,
    notify_on_start     INTEGER NOT NULL DEFAULT 1,
    updated_at          TEXT NOT NULL,
    CHECK (id = 1)
);

-- ---------------------------------------------------------------------------
-- Permissions
-- ---------------------------------------------------------------------------
INSERT OR IGNORE INTO permissions (id, code, name, resource, action) VALUES
    ('p_llm_use',           'llm:use',           'Invoke LLM tools',          'llm', 'use'),
    ('p_llm_manage',        'llm:manage',        'Manage providers & models', 'llm', 'manage'),
    ('p_llm_usage_read',    'llm:usage:read',    'View LLM usage stats',      'llm', 'usage:read'),
    ('p_llm_budget_manage', 'llm:budget:manage', 'Set LLM cost ceilings',     'llm', 'budget:manage');

INSERT OR IGNORE INTO role_permissions (role_id, permission_id)
    SELECT 'r_super_admin', id FROM permissions
    WHERE code IN ('llm:use', 'llm:manage', 'llm:usage:read', 'llm:budget:manage');

-- ---------------------------------------------------------------------------
-- Menus
-- ---------------------------------------------------------------------------
-- AI tools group (top-level menu)
INSERT OR IGNORE INTO menus (id, parent_id, code, title, title_key, path,
                             icon, sort_order, visible, status, menu_type,
                             permission_code) VALUES
    ('m_ai', NULL, 'ai', 'AI Tools', 'menu.ai', '/ai/chat', 'MagicStick', 30,
        1, 'active', 'group', NULL);

INSERT OR IGNORE INTO menus (id, parent_id, code, title, title_key, path,
                             icon, sort_order, visible, status, menu_type,
                             permission_code) VALUES
    ('m_ai_chat',       'm_ai', 'ai.chat',       'AI Chat',       'menu.ai.chat',       '/ai/chat',       'ChatLineRound',  10, 1, 'active', 'menu', 'llm:use'),
    ('m_ai_translate',  'm_ai', 'ai.translate',  'AI Translate',  'menu.ai.translate',  '/ai/translate',  'Translate',      20, 1, 'active', 'menu', 'llm:use'),
    ('m_ai_explain',    'm_ai', 'ai.explain',    'AI Explain',    'menu.ai.explain',    '/ai/explain',    'QuestionFilled', 30, 1, 'active', 'menu', 'llm:use'),
    ('m_ai_summarize',  'm_ai', 'ai.summarize',  'AI Summarize',  'menu.ai.summarize',  '/ai/summarize',  'Document',       40, 1, 'active', 'menu', 'llm:use');

-- LLM management group under system
INSERT OR IGNORE INTO menus (id, parent_id, code, title, title_key, path,
                             icon, sort_order, visible, status, menu_type,
                             permission_code) VALUES
    ('m_llm',           'm_system', 'system.llm',           'LLM',           'menu.llm',           '/system/llm/providers', 'Connection',    85, 1, 'active', 'group', 'llm:manage'),
    ('m_llm_providers', 'm_llm',    'system.llm.providers', 'Providers',     'menu.llm.providers', '/system/llm/providers', 'Cloud',         10, 1, 'active', 'menu',  'llm:manage'),
    ('m_llm_models',    'm_llm',    'system.llm.models',    'Models',        'menu.llm.models',    '/system/llm/models',    'Box',           20, 1, 'active', 'menu',  'llm:manage'),
    ('m_llm_usage',     'm_llm',    'system.llm.usage',     'Usage',         'menu.llm.usage',     '/system/llm/usage',     'DataAnalysis',  30, 1, 'active', 'menu',  'llm:usage:read');

INSERT OR IGNORE INTO role_menus (role_id, menu_id)
    SELECT 'r_super_admin', id FROM menus WHERE code IN
        ('ai', 'ai.chat', 'ai.translate', 'ai.explain', 'ai.summarize',
         'system.llm', 'system.llm.providers', 'system.llm.models', 'system.llm.usage');

INSERT OR IGNORE INTO llm_fallback_config (id, enabled, model_id, notify_on_start, updated_at)
    VALUES (1, 0, NULL, 1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));