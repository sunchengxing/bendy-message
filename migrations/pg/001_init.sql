-- bendy-message PostgreSQL 初始化迁移

CREATE TABLE IF NOT EXISTS bmsg_messages (
    id TEXT PRIMARY KEY,
    platform TEXT NOT NULL,
    app_package TEXT NOT NULL,
    user_id TEXT NOT NULL,
    msg_type TEXT NOT NULL CHECK (msg_type IN ('notification', 'message', 'shell')),
    content JSONB NOT NULL,
    persist BOOLEAN NOT NULL DEFAULT TRUE,
    ttl BIGINT,
    created_at BIGINT NOT NULL,
    expires_at BIGINT
);

CREATE INDEX IF NOT EXISTS idx_bmsg_messages_platform ON bmsg_messages(platform);
CREATE INDEX IF NOT EXISTS idx_bmsg_messages_app_package ON bmsg_messages(app_package);
CREATE INDEX IF NOT EXISTS idx_bmsg_messages_user_id ON bmsg_messages(user_id);
CREATE INDEX IF NOT EXISTS idx_bmsg_messages_created_at ON bmsg_messages(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_bmsg_messages_expires_at ON bmsg_messages(expires_at) WHERE expires_at IS NOT NULL;

CREATE TABLE IF NOT EXISTS bmsg_services (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    app_package TEXT NOT NULL,
    platforms JSONB NOT NULL,
    secret_hash TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'offline' CHECK (status IN ('online', 'offline')),
    last_heartbeat BIGINT NOT NULL,
    created_at BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_bmsg_services_app_package ON bmsg_services(app_package);
CREATE INDEX IF NOT EXISTS idx_bmsg_services_status ON bmsg_services(status);

CREATE TABLE IF NOT EXISTS bmsg_nodes (
    id TEXT PRIMARY KEY,
    role TEXT NOT NULL CHECK (role IN ('master', 'slave')),
    platform TEXT NOT NULL,
    region TEXT NOT NULL,
    started_at BIGINT NOT NULL,
    last_heartbeat BIGINT NOT NULL
);
