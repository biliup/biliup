-- Web 用户与固定三角色（#1717）。
--
-- 老的单管理员原样迁过来：沿用 configuration 里的 id 与密码哈希，这样升级后
-- 会话里存的 user id 与 auth hash 都不变，已登录的浏览器不会掉线。
-- configuration 里 key = 'biliup' 那一行保留不删：回退到旧版本时仍能用原密码登录，
-- 新代码不再读写它。
CREATE TABLE web_users (
    id              INTEGER PRIMARY KEY,
    username        TEXT    NOT NULL COLLATE NOCASE UNIQUE,
    password_hash   TEXT    NOT NULL,
    role            TEXT    NOT NULL CHECK (role IN ('admin', 'operator', 'viewer')),
    disabled        INTEGER NOT NULL DEFAULT 0 CHECK (disabled IN (0, 1)),
    session_version INTEGER NOT NULL DEFAULT 0,
    created_at      INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at      INTEGER NOT NULL DEFAULT (unixepoch()),
    last_login_at   INTEGER
);

INSERT INTO web_users (id, username, password_hash, role)
SELECT id, 'biliup', value, 'admin'
FROM configuration
WHERE key = 'biliup';
