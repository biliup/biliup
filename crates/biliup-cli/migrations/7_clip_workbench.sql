-- 切片工作台：场次与分段。
--
-- 一场直播 = 一个场次；同一主播下播后 `clip_session_merge_minutes` 分钟内再开播，
-- 复用上一场（中间的断流记在下一段的 gap_before_ms）。streamerinfo / filelist 与投稿
-- 语义不变，断流合并时一场挂多行 streamerinfo。
--
-- 时间一律是毫秒：*_at 是 Unix 毫秒时间戳；start_ms / end_ms 是场次时间轴上的位置，
-- 0 = 场次第一个分段开写（第一个关键帧到达）的墙钟时刻。关键帧索引不进库，
-- 缓存在分段文件旁边的 `<分段>.idx`。
CREATE TABLE stream_sessions (
    id           INTEGER PRIMARY KEY,
    streamer_id  INTEGER REFERENCES livestreamers (id) ON DELETE SET NULL,
    title        TEXT    NOT NULL,
    started_at   INTEGER NOT NULL,
    ended_at     INTEGER,
    retain_until INTEGER,
    created_at   INTEGER NOT NULL
);

CREATE INDEX idx_stream_sessions_streamer ON stream_sessions (streamer_id, id);

CREATE TABLE session_streamerinfo (
    session_id      INTEGER NOT NULL REFERENCES stream_sessions (id) ON DELETE CASCADE,
    streamerinfo_id INTEGER NOT NULL REFERENCES streamerinfo (id) ON DELETE CASCADE,
    PRIMARY KEY (session_id, streamerinfo_id)
);

CREATE TABLE segments (
    id            INTEGER PRIMARY KEY,
    session_id    INTEGER NOT NULL REFERENCES stream_sessions (id) ON DELETE CASCADE,
    path          TEXT    NOT NULL,
    container     TEXT    NOT NULL CHECK (container IN ('flv', 'ts', 'mp4', 'mkv')),
    state         TEXT    NOT NULL CHECK (state IN ('recording', 'finished', 'missing', 'deleted', 'pending_delete')),
    start_ms      INTEGER NOT NULL,
    end_ms        INTEGER,
    bytes         INTEGER,
    index_path    TEXT,
    danmaku_path  TEXT,
    gap_before_ms INTEGER NOT NULL DEFAULT 0,
    pin_count     INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_segments_session ON segments (session_id, start_ms);
CREATE INDEX idx_segments_state ON segments (state);
