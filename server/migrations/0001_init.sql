-- Phase 1 schema. One Dissedoff deployment == one community ("server_info" is a
-- singleton row); multi-guild-per-deployment is out of scope for v1 (the stated
-- requirement is federation *across* deployments, not multi-tenancy within one).

CREATE TABLE server_info (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    server_id TEXT NOT NULL,
    public_server_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    visibility TEXT NOT NULL DEFAULT 'private',
    federation_mode TEXT NOT NULL DEFAULT 'disabled',
    created_at INTEGER NOT NULL
);

CREATE TABLE users (
    internal_id TEXT PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE sessions (
    token_hash TEXT PRIMARY KEY,
    user_internal_id TEXT NOT NULL REFERENCES users(internal_id),
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL
);

CREATE TABLE roles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    position INTEGER NOT NULL,
    permissions INTEGER NOT NULL,
    is_default INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE member_roles (
    user_internal_id TEXT NOT NULL REFERENCES users(internal_id),
    role_id TEXT NOT NULL REFERENCES roles(id),
    PRIMARY KEY (user_internal_id, role_id)
);

CREATE TABLE channels (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    parent_id TEXT REFERENCES channels(id),
    position INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
);

CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    channel_id TEXT NOT NULL REFERENCES channels(id),
    author_internal_id TEXT NOT NULL REFERENCES users(internal_id),
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    edited_at INTEGER
);

CREATE INDEX idx_messages_channel_created ON messages(channel_id, created_at);
