-- bendy-message D1 初始化迁移

CREATE TABLE IF NOT EXISTS bmsg_messages (
    id TEXT PRIMARY KEY,
    platform TEXT NOT NULL,
    app_package TEXT NOT NULL,
    user_id TEXT NOT NULL,
    msg_type TEXT NOT NULL,
    content TEXT NOT NULL,
    persist INTEGER NOT NULL DEFAULT 1,
    ttl INTEGER,
    created_at INTEGER NOT NULL,
    expires_at INTEGER
);

CREATE INDEX IF NOT EXISTS idx_bmsg_messages_platform ON bmsg_messages(platform);
CREATE INDEX IF NOT EXISTS idx_bmsg_messages_app_package ON bmsg_messages(app_package);
CREATE INDEX IF NOT EXISTS idx_bmsg_messages_user_id ON bmsg_messages(user_id);
CREATE INDEX IF NOT EXISTS idx_bmsg_messages_created_at ON bmsg_messages(created_at);
CREATE INDEX IF NOT EXISTS idx_bmsg_messages_expires_at ON bmsg_messages(expires_at);

CREATE TABLE IF NOT EXISTS bmsg_services (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    app_package TEXT NOT NULL,
    platforms TEXT NOT NULL,
    secret_hash TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'offline',
    last_heartbeat INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_bmsg_services_app_package ON bmsg_services(app_package);
CREATE INDEX IF NOT EXISTS idx_bmsg_services_status ON bmsg_services(status);
