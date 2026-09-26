-- Fleet F2：房间分派。房间与投稿模板的「真身」在控制面，节点按期望状态落进本地
-- livestreamers / uploadstreamers，映射记在节点的 data/fleet-state.json。
-- 时间一律是 Unix 毫秒。

-- 投稿模板：与主库 uploadstreamers 同形，只是 user_cookie（节点本地的凭据文件路径）
-- 换成 account_mid：路径只在节点上有意义，跨节点只能按 B 站账号 mid 引用，节点自己解析成路径。
CREATE TABLE fleet_templates (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    template_name      TEXT    NOT NULL,
    title              TEXT,
    tid                INTEGER,
    tid_v2             INTEGER,
    copyright          INTEGER,
    copyright_source   TEXT,
    cover_path         TEXT,
    description        TEXT,
    dynamic            TEXT,
    dtime              INTEGER,
    dolby              INTEGER,
    hires              INTEGER,
    charging_pay       INTEGER,
    no_reprint         INTEGER,
    is_only_self       INTEGER,
    uploader           TEXT,
    account_mid        INTEGER,
    tags               TEXT    NOT NULL DEFAULT '[]',
    credits            TEXT,
    up_selection_reply INTEGER,
    up_close_reply     INTEGER,
    up_close_danmu     INTEGER,
    extra_fields       TEXT,
    created_at         INTEGER NOT NULL,
    updated_at         INTEGER NOT NULL
);

-- 房间：与主库 livestreamers 同形，另加分派状态。
-- node_id 为空表示未分派；epoch 是分派代号，每次改分派 +1，节点重连时按它对账。
-- 迁移时先由 releasing_node_id 那台释放，控制面收到它版本号大于 release_after 的 Ack、
-- 且 Ack 里不再持有这个房间，才把房间放进 node_id 那台的期望状态。
-- deleted_at 非空表示已删除、正在等 releasing_node_id 释放；释放后整行删掉。
CREATE TABLE fleet_rooms (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    url                  TEXT    NOT NULL UNIQUE,
    remark               TEXT    NOT NULL,
    filename_prefix      TEXT,
    time_range           TEXT,
    template_id          INTEGER REFERENCES fleet_templates (id),
    format               TEXT,
    override             TEXT,
    preprocessor         TEXT,
    segment_processor    TEXT,
    downloaded_processor TEXT,
    postprocessor        TEXT,
    opt_args             TEXT,
    excluded_keywords    TEXT,
    node_id              INTEGER REFERENCES fleet_nodes (id),
    epoch                INTEGER NOT NULL DEFAULT 1,
    paused               INTEGER NOT NULL DEFAULT 0 CHECK (paused IN (0, 1)),
    releasing_node_id    INTEGER REFERENCES fleet_nodes (id),
    release_after        INTEGER,
    deleted_at           INTEGER,
    created_at           INTEGER NOT NULL,
    updated_at           INTEGER NOT NULL
);

CREATE INDEX fleet_rooms_node ON fleet_rooms (node_id);
CREATE INDEX fleet_rooms_releasing ON fleet_rooms (releasing_node_id);

-- 节点上报的 B 站账号（只有 mid 与昵称，凭据不出节点），每次上报整份替换。
CREATE TABLE fleet_node_accounts (
    node_id     INTEGER NOT NULL REFERENCES fleet_nodes (id) ON DELETE CASCADE,
    mid         INTEGER NOT NULL,
    uname       TEXT    NOT NULL DEFAULT '',
    reported_at INTEGER NOT NULL,
    PRIMARY KEY (node_id, mid)
);
