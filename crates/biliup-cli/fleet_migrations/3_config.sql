-- Fleet F3：分层配置。控制面只存「可下发」的配置项（`redact::config` 白名单里的键），
-- Cookie、密码、主播列表这些白名单外的键一律不进这个库，只留在各节点本地。
-- 时间一律是 Unix 毫秒。

-- Fleet 全局配置：每保存一次新增一行，version 自增，最新一行生效；只保留最近若干版供查看。
-- config 是 JSON 对象，只含全局共享的白名单键（不含池大小、ffmpeg 路径这类按节点的键）。
-- updated_by 是主库 web_users.id（跨库，不设外键；无鉴权部署为 NULL）。
CREATE TABLE fleet_config (
    version    INTEGER PRIMARY KEY AUTOINCREMENT,
    config     TEXT    NOT NULL,
    updated_at INTEGER NOT NULL,
    updated_by INTEGER
);

-- 节点覆盖：ConfigPatch 语义的 JSON 对象，只含设置了的白名单键，空对象表示不覆盖。
ALTER TABLE fleet_nodes ADD COLUMN config_override TEXT NOT NULL DEFAULT '{}';
