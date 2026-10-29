-- Tokenmonk database schema, version 1 (SQLite).
-- Source of truth for migration 1; the db module applies it and records the
-- version (PRAGMA user_version). Later versions are new migrations, never
-- edits of this file. Timestamps are Unix epoch milliseconds (UTC).
-- Booleans are INTEGER 0/1. Settings and app state values are JSON text.

-- Read progress per log file, for incremental reads.
CREATE TABLE source_files (
    path         TEXT    PRIMARY KEY,          -- absolute path of the .jsonl file
    size_bytes   INTEGER NOT NULL,             -- file size at the last read
    offset_bytes INTEGER NOT NULL,             -- byte after the last complete line read
    mtime_ms     INTEGER NOT NULL,             -- modification time at the last read
    updated_at   INTEGER NOT NULL
);

-- One row per API response (assistant message), deduplicated by message id
-- across all files. Upsert is read-order independent: ts = MIN(old, new),
-- every token column = MAX(old, new); the other columns are identical on
-- every copy of a message and keep their stored value.
CREATE TABLE messages (
    message_id            TEXT    PRIMARY KEY, -- message.id
    request_id            TEXT,                -- requestId, if present
    session_id            TEXT    NOT NULL,    -- sessionId
    agent_id              TEXT,                -- agentId for subagent sidechains, else NULL
    is_sidechain          INTEGER NOT NULL DEFAULT 0,
    ts                    INTEGER NOT NULL,    -- earliest timestamp of the message's lines
    model                 TEXT    NOT NULL,    -- message.model
    project_path          TEXT    NOT NULL,    -- cwd
    input_tokens          INTEGER NOT NULL DEFAULT 0,
    output_tokens         INTEGER NOT NULL DEFAULT 0,
    cache_creation_tokens INTEGER NOT NULL DEFAULT 0,
    cache_read_tokens     INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX messages_ts ON messages (ts);
CREATE INDEX messages_session_ts ON messages (session_id, ts);

-- Every good limit reading, for staleness, burn projection and the daily
-- budget baseline. Percentages are 0..100 (may exceed 100).
CREATE TABLE limit_snapshots (
    id                  INTEGER PRIMARY KEY,
    fetched_at          INTEGER NOT NULL,
    source              TEXT    NOT NULL,      -- LimitSource::id()
    five_hour_pct       REAL,
    five_hour_resets_at INTEGER,
    weekly_pct          REAL,
    weekly_resets_at    INTEGER
);

CREATE INDEX limit_snapshots_fetched_at ON limit_snapshots (fetched_at);

-- User settings as key/value; keys: mode, sound, autostart.
CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL                        -- JSON, e.g. "\"diy\"" or "true"
);

-- XP ledger. Level and progress are derived from SUM(amount).
-- (kind, ref) makes awards idempotent, e.g. ('day', '2026-10-28').
CREATE TABLE xp_events (
    id          INTEGER PRIMARY KEY,
    earned_at   INTEGER NOT NULL,
    mode        TEXT    NOT NULL,              -- 'max' | 'diy'
    kind        TEXT    NOT NULL,              -- defined by the game module
    ref         TEXT    NOT NULL,
    amount      INTEGER NOT NULL,
    retroactive INTEGER NOT NULL DEFAULT 0,
    UNIQUE (kind, ref)
);

CREATE INDEX xp_events_earned_at ON xp_events (earned_at);

-- Achievement state; id is an AchievementId string ('firstBlood', ...).
CREATE TABLE achievements (
    id          TEXT    PRIMARY KEY,
    unlocked_at INTEGER,                       -- NULL while locked
    retroactive INTEGER NOT NULL DEFAULT 0,
    current     INTEGER,                       -- counted progress, NULL for one-shots
    target      INTEGER,
    updated_at  INTEGER NOT NULL
);

-- Internal flags as key/value, e.g. retro_done, first_start_at.
CREATE TABLE app_state (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL                        -- JSON
);
