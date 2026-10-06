-- 切片工作台的标记：看直播时按一下「标记」，记下场次时间轴上的一个时刻，之后在工作台里从这里剪。
--
-- at_ms 是场次时间轴上的位置（与 segments.start_ms 同一条轴，0 = stream_sessions.started_at）。
-- lookback_ms / lookahead_ms 是这个标记默认覆盖的范围：[at_ms - lookback_ms, at_ms + lookahead_ms]。
-- created_at 是 Unix 毫秒；created_by 是打标记的 Web 用户，未开 --auth 时为 NULL。
-- id 用 AUTOINCREMENT：撤销掉的标记 id 不会被下一个标记复用，手里还拿着旧列表的页面改名 / 删除只会落空。
CREATE TABLE markers (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id   INTEGER NOT NULL REFERENCES stream_sessions (id) ON DELETE CASCADE,
    at_ms        INTEGER NOT NULL CHECK (at_ms >= 0),
    label        TEXT    NOT NULL DEFAULT '',
    color        TEXT,
    created_by   INTEGER REFERENCES web_users (id) ON DELETE SET NULL,
    created_at   INTEGER NOT NULL,
    lookback_ms  INTEGER NOT NULL DEFAULT 60000 CHECK (lookback_ms >= 0),
    lookahead_ms INTEGER NOT NULL DEFAULT 0 CHECK (lookahead_ms >= 0)
);

CREATE INDEX idx_markers_session ON markers (session_id, at_ms);
