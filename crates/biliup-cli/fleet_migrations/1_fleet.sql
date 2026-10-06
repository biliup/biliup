-- 控制面自己的库 data/fleet.sqlite3：只有 `biliup server --controller` 才会创建，
-- 与主库 data.sqlite3 的迁移各自编号、各自记账，互不影响。
-- 时间一律是 Unix 毫秒。

-- 控制面的 iroh 身份：ed25519 私钥，EndpointId 由它导出，写进每张 join 票据。
-- 只有一行；丢了它，所有已加入的节点都连不上控制面，只能重新加入。
CREATE TABLE fleet_identity (
    id         INTEGER PRIMARY KEY CHECK (id = 1),
    secret_key BLOB    NOT NULL CHECK (length(secret_key) = 32),
    created_at INTEGER NOT NULL
);

-- 已加入的节点。endpoint_id 是节点公钥（iroh EndpointId 的十六进制），连接身份以它为准。
-- 移除节点只写 revoked_at，不删行：被移除的公钥之后再连上来也会被拒。
-- last_* 是最近一次心跳的摘要，控制面至多每分钟落一次盘，节点离线时列表靠它显示。
CREATE TABLE fleet_nodes (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    name         TEXT    NOT NULL,
    endpoint_id  TEXT    NOT NULL UNIQUE,
    labels       TEXT    NOT NULL DEFAULT '[]',
    allow_hooks  INTEGER NOT NULL DEFAULT 0 CHECK (allow_hooks IN (0, 1)),
    created_at   INTEGER NOT NULL,
    last_seen_at INTEGER,
    last_version TEXT,
    last_summary TEXT,
    revoked_at   INTEGER
);

-- 一次性 join 令牌。票据里带 id 与 16 字节密钥，库里只存密钥的 SHA-256。
-- created_by 是主库 web_users.id（跨库，不设外键；无鉴权部署为 NULL）。
-- 用过的令牌保留 used_at / used_by_node 备查；作废即删行。
CREATE TABLE fleet_join_tokens (
    id           TEXT    PRIMARY KEY CHECK (length(id) = 6),
    secret_hash  TEXT    NOT NULL,
    created_by   INTEGER,
    created_at   INTEGER NOT NULL,
    expires_at   INTEGER NOT NULL,
    used_at      INTEGER,
    used_by_node INTEGER REFERENCES fleet_nodes (id) ON DELETE SET NULL
);
