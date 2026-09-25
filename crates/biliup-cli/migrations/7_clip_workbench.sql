-- 场次与分段。
--
-- streamerinfo 本来就是「一场直播」（开播时插一行，filelist 与投稿都挂在它下面），改名为
-- stream_sessions 并加上切片工作台需要的列；不另建第二张场次表。ALTER TABLE ... RENAME
-- 会把 filelist 外键里对旧表名 / 旧列名的引用一并改掉，不重建表、不搬数据。
--
-- 新列的时间一律是 Unix 毫秒；segments 的 start_ms / end_ms 是场次时间轴上的位置，
-- 0 = started_at（场次第一个分段开写、第一个关键帧到达的墙钟时刻）。关键帧索引不进库，
-- 缓存在分段文件旁边的 `<分段>.idx`。
ALTER TABLE streamerinfo RENAME TO stream_sessions;

ALTER TABLE stream_sessions ADD COLUMN streamer_id INTEGER REFERENCES livestreamers (id) ON DELETE SET NULL;
-- NULL = 还没有分段（没有时间轴）
ALTER TABLE stream_sessions ADD COLUMN started_at INTEGER;
-- NULL = 正在录，或进程异常退出还没收尾（启动时补上）
ALTER TABLE stream_sessions ADD COLUMN ended_at INTEGER;
ALTER TABLE stream_sessions ADD COLUMN retain_until INTEGER;

-- 老数据：按 url 找回主播（找不到就是 NULL，永远不会被断流合并接上）；没有结束时间，
-- 记为开播时间，保证老行都算「已结束」；date 解析不了时记 0。
UPDATE stream_sessions
SET streamer_id = (SELECT l.id FROM livestreamers l WHERE l.url = stream_sessions.url),
    ended_at    = COALESCE(CAST(ROUND((julianday(date) - 2440587.5) * 86400000) AS INTEGER), 0);

CREATE INDEX idx_stream_sessions_streamer ON stream_sessions (streamer_id, id);

ALTER TABLE filelist RENAME COLUMN streamer_info_id TO session_id;

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
