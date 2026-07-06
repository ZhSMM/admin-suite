-- v0.7.0 — Persistent multi-level chat history.
--
-- A "session" is a tree root. Each message node has an optional
-- `parent_id`; siblings under the same parent = a branch.
-- The "active path" is the user-picked root→leaf chain that the LLM
-- actually sees when sending. We reconstruct the path client-side
-- from the persisted tree, so we don't need a separate branch table.

CREATE TABLE IF NOT EXISTS chat_sessions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id         INTEGER NOT NULL,
    title           TEXT    NOT NULL DEFAULT '',
    provider_id     TEXT    NOT NULL DEFAULT '',
    model_id        TEXT    NOT NULL DEFAULT '',
    created_at      TEXT    NOT NULL,
    updated_at      TEXT    NOT NULL,
    archived        INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_chat_sessions_user_updated
    ON chat_sessions(user_id, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_chat_sessions_archived
    ON chat_sessions(user_id, archived);

CREATE TABLE IF NOT EXISTS chat_messages (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id      INTEGER NOT NULL,
    parent_id       INTEGER,
    role            TEXT    NOT NULL,
    content         TEXT    NOT NULL,
    provider_id     TEXT    NOT NULL DEFAULT '',
    model_id        TEXT    NOT NULL DEFAULT '',
    status          TEXT    NOT NULL DEFAULT 'done',
    error           TEXT,
    created_at      TEXT    NOT NULL,
    FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_id)  REFERENCES chat_messages(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_chat_messages_session
    ON chat_messages(session_id, id);

CREATE INDEX IF NOT EXISTS idx_chat_messages_parent
    ON chat_messages(parent_id);
