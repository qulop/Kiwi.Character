-- Kiwi.Character SQLite schema (version 4).
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

-- ---------- groups ----------
-- A group room (multiple characters + the user). Group chat itself isn't
-- implemented yet — for now a group is created and shown in the sidebar list.
CREATE TABLE IF NOT EXISTS groups (
    id              TEXT    PRIMARY KEY,
    name            TEXT    NOT NULL,
    topic           TEXT    NOT NULL DEFAULT '',
    background      TEXT    NOT NULL DEFAULT '',  -- "Background & Relationships"
    avatar_path     TEXT,                         -- NULL = use a member-avatar collage
    created_at      INTEGER NOT NULL,             -- ms epoch
    updated_at      INTEGER NOT NULL              -- ms epoch
);

CREATE INDEX IF NOT EXISTS idx_groups_created_at
    ON groups (created_at);

-- ---------- group_members ----------
CREATE TABLE IF NOT EXISTS group_members (
    group_id        TEXT    NOT NULL
                        REFERENCES groups(id) ON DELETE CASCADE,
    character_id    TEXT    NOT NULL
                        REFERENCES characters(id) ON DELETE CASCADE,
    position        INTEGER NOT NULL,             -- add order; drives "first 4" for the collage
    PRIMARY KEY (group_id, character_id)
);

CREATE INDEX IF NOT EXISTS idx_group_members_group
    ON group_members (group_id, position);

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

-- ---------- memory_settings (single row, id is always 1) ----------
CREATE TABLE IF NOT EXISTS memory_settings (
    id                        INTEGER PRIMARY KEY CHECK (id = 1),
    enabled                   INTEGER NOT NULL DEFAULT 1,
    embedding_endpoint        TEXT    NOT NULL DEFAULT 'http://localhost:1234/v1',
    embedding_model           TEXT    NOT NULL DEFAULT 'Qwen/Qwen3-Embedding-0.6B',
    embedding_dimensions      INTEGER NOT NULL DEFAULT 1024 CHECK (embedding_dimensions > 0),
    recent_message_limit      INTEGER NOT NULL DEFAULT 20 CHECK (recent_message_limit > 0),
    recall_depth              INTEGER NOT NULL DEFAULT 6 CHECK (recall_depth > 0),
    ranking_mode              TEXT    NOT NULL DEFAULT 'embedding'
                              CHECK (ranking_mode IN ('embedding', 'reranker')),
    reranker_model            TEXT,
    reranker_candidate_limit  INTEGER NOT NULL DEFAULT 24 CHECK (reranker_candidate_limit > 0),
    updated_at                INTEGER NOT NULL
);

-- ---------- memories ----------
CREATE TABLE IF NOT EXISTS memories (
    id                   TEXT    PRIMARY KEY,
    conversation_id      TEXT    NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    character_id         TEXT    NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    persona_id           TEXT    REFERENCES personas(id) ON DELETE CASCADE,
    kind                 TEXT    NOT NULL CHECK (kind IN ('fact','preference','event','relationship','summary','manual')),
    content              TEXT    NOT NULL,
    embedding            BLOB    NOT NULL,
    embedding_dimensions INTEGER NOT NULL CHECK (embedding_dimensions > 0),
    embedding_model      TEXT    NOT NULL,
    importance           INTEGER NOT NULL DEFAULT 3 CHECK (importance BETWEEN 1 AND 5),
    pinned               INTEGER NOT NULL DEFAULT 0,
    status               TEXT    NOT NULL DEFAULT 'active'
                         CHECK (status IN ('active','stale','invalid')),
    created_at           INTEGER NOT NULL,
    updated_at           INTEGER NOT NULL,
    last_recalled_at     INTEGER
);

CREATE INDEX IF NOT EXISTS idx_memories_active_scope
    ON memories(conversation_id, persona_id, status);

-- ---------- memory_sources ----------
CREATE TABLE IF NOT EXISTS memory_sources (
    memory_id  TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    PRIMARY KEY (memory_id, message_id)
);

CREATE INDEX IF NOT EXISTS idx_memory_sources_message ON memory_sources (message_id);
