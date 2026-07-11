-- Kiwi.Character SQLite schema (version 1).
-- Authoritative reference: agent-docs/database-scheme.md
-- All CREATE ... IF NOT EXISTS so this doubles as the migration for v1.

-- ---------- characters ----------
CREATE TABLE IF NOT EXISTS characters (
    id              TEXT    PRIMARY KEY,
    name            TEXT    NOT NULL,
    info            TEXT    NOT NULL DEFAULT '',
    avatar_path     TEXT,                       -- NULL when no avatar; else relative path under avatars/
    appearance      TEXT    NOT NULL DEFAULT '',
    description     TEXT    NOT NULL DEFAULT '',
    initial_message TEXT    NOT NULL DEFAULT '',
    is_favorite     INTEGER NOT NULL DEFAULT 0, -- 0/1
    created_at      INTEGER NOT NULL,           -- ms epoch
    updated_at      INTEGER NOT NULL            -- ms epoch
);

-- Case-insensitive uniqueness for the name-collision rule (Step 6).
CREATE UNIQUE INDEX IF NOT EXISTS idx_characters_name_nocase
    ON characters (name COLLATE NOCASE);

CREATE INDEX IF NOT EXISTS idx_characters_is_favorite
    ON characters (is_favorite);

CREATE INDEX IF NOT EXISTS idx_characters_created_at
    ON characters (created_at);

-- ---------- conversations ----------
CREATE TABLE IF NOT EXISTS conversations (
    id              TEXT    PRIMARY KEY,        -- frontend uses "conv-<characterId>"
    character_id    TEXT    NOT NULL
                        REFERENCES characters(id) ON DELETE CASCADE,
    created_at      INTEGER NOT NULL,           -- ms epoch
    last_message_at INTEGER,                    -- ms epoch of newest message; NULL until first message
    active_persona_id TEXT                      -- NULL when no persona is selected for this chat
);

CREATE INDEX IF NOT EXISTS idx_conversations_character
    ON conversations (character_id);

CREATE INDEX IF NOT EXISTS idx_conversations_last_message
    ON conversations (last_message_at);

-- ---------- messages ----------
CREATE TABLE IF NOT EXISTS messages (
    id              TEXT    PRIMARY KEY,
    conversation_id TEXT    NOT NULL
                        REFERENCES conversations(id) ON DELETE CASCADE,
    role            TEXT    NOT NULL CHECK (role IN ('user','assistant','system')),
    content         TEXT    NOT NULL,
    created_at      INTEGER NOT NULL,           -- ms epoch
    hidden          INTEGER NOT NULL DEFAULT 0  -- 1 = technical message, not shown in the UI
);

CREATE INDEX IF NOT EXISTS idx_messages_conversation
    ON messages (conversation_id, created_at);

-- ---------- personas ----------
CREATE TABLE IF NOT EXISTS personas (
    id              TEXT    PRIMARY KEY,
    name            TEXT    NOT NULL,
    description     TEXT    NOT NULL DEFAULT '',
    avatar_path     TEXT,                       -- NULL when no avatar; else relative path under avatars/
    is_default      INTEGER NOT NULL DEFAULT 0, -- 0/1 — reserved for a future "default persona" feature
    created_at      INTEGER NOT NULL,           -- ms epoch
    updated_at      INTEGER NOT NULL            -- ms epoch
);

CREATE INDEX IF NOT EXISTS idx_personas_created_at
    ON personas (created_at);

-- ---------- settings (single row, id is always 1) ----------
CREATE TABLE IF NOT EXISTS settings (
    id              INTEGER PRIMARY KEY CHECK (id = 1),
    endpoint        TEXT    NOT NULL,
    model           TEXT    NOT NULL,
    context_length  INTEGER NOT NULL,
    gpu_offload     INTEGER NOT NULL,
    temperature     REAL    NOT NULL,
    max_tokens      INTEGER NOT NULL,
    system_prompt   TEXT    NOT NULL DEFAULT ''
);
