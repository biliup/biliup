-- Fleet H1：一主一备（HA Pair）。主机是控制面的「本机」节点（F5），备机是一台普通节点。
-- 时间一律是 Unix 毫秒，时长一律是秒。

-- 主副配对，至多一行。主机上「本机」持有的房间与模板自动镜像给备机，不改 fleet_rooms 的分派。
-- 参数的含义见 ha-pair 方案 §5.7 与 §6 C；随期望状态下发给备机，不进主库配置。
CREATE TABLE ha_pair (
    id                   INTEGER PRIMARY KEY CHECK (id = 1),
    primary_node_id      INTEGER NOT NULL REFERENCES fleet_nodes (id),
    standby_node_id      INTEGER NOT NULL REFERENCES fleet_nodes (id),
    mode                 INTEGER NOT NULL CHECK (mode IN (1, 2)),
    offline_grace        INTEGER NOT NULL,
    standby_upload_delay INTEGER NOT NULL,
    upload_start_timeout INTEGER NOT NULL,
    upload_stall_timeout INTEGER NOT NULL,
    progress_interval    INTEGER NOT NULL,
    manual_timeout       INTEGER NOT NULL,
    delete_standby_copy  INTEGER NOT NULL DEFAULT 0 CHECK (delete_standby_copy IN (0, 1)),
    created_at           INTEGER NOT NULL,
    updated_at           INTEGER NOT NULL,
    CHECK (primary_node_id <> standby_node_id)
);

-- 主机侧的场次记录：一场 = 一个房间的一次开播（断流合并算同一场），键见 ha-pair 方案 §2。
-- primary_state 是主机自己那份的进度，standby_state 是备机最近一次上报的；uploader 是这一场由谁投。
-- local_session_id / unit_started_at 指向主机主库 stream_sessions 里的那一场，
-- 主机中途崩溃、模式 2 要补投自己那半时靠它找分段。room_id 不设外键：房间删了记录照留。
CREATE TABLE ha_sessions (
    session_key      TEXT    PRIMARY KEY,
    room_id          INTEGER NOT NULL,
    started_at       INTEGER NOT NULL,
    ended_at         INTEGER,
    primary_state    TEXT    NOT NULL CHECK (primary_state IN (
        'none', 'recording', 'recorded', 'uploading', 'uploaded', 'failed', 'skipped',
        'interrupted', 'handed_over')),
    standby_state    TEXT,
    uploader         TEXT CHECK (uploader IN ('primary', 'standby')),
    bvid             TEXT,
    upload_bytes     INTEGER NOT NULL DEFAULT 0,
    progress_at      INTEGER,
    reason           TEXT,
    local_session_id INTEGER,
    unit_started_at  INTEGER,
    updated_at       INTEGER NOT NULL
);

CREATE INDEX ha_sessions_room ON ha_sessions (room_id, started_at);
CREATE INDEX ha_sessions_updated ON ha_sessions (updated_at);
