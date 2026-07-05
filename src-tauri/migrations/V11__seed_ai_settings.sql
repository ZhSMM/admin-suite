-- V11 — v0.6.1 AI defaults (settings)
--
-- Adds global defaults for the Chat / Translate tools.  Per-user defaults
-- (saved by the frontend in localStorage) still take precedence — these
-- only act as the fallback when the user has not yet picked anything.
--
-- Keys (all strings, empty allowed):
--   ai.default_chat_provider      — provider id (or empty)
--   ai.default_chat_model         — model id    (or empty)
--   ai.default_translate_provider — provider id (or empty)
--   ai.default_translate_model    — model id    (or empty)
--   ai.local_first                — 'true' | 'false' (prefer offline fallback)

INSERT OR IGNORE INTO app_state (key, value, updated_at) VALUES
    ('ai.default_chat_provider',      '', strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    ('ai.default_chat_model',         '', strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    ('ai.default_translate_provider', '', strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    ('ai.default_translate_model',    '', strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    ('ai.local_first',                'false', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));