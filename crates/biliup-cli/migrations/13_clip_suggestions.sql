-- 自动切片（实验）给出的候选片段。没配置 `auto_clip` 时这张表一直是空的。
--
-- 候选不是切片：不进集中发布、不能导出；接受时才用 `clips` 建一条草稿，记在 clip_id 上。
-- in_ms / out_ms：场次时间，已吸附到关键帧（入点不晚于、出点不早于模型给的时间）。
-- title / reason：模型给的标题与理由；confidence：模型自评的把握（0–1），没给时为 NULL。
-- tags：标签（JSON 字符串数组）；evidence：服务端核对过的依据（JSON：区间里的转写句数、
--   弹幕高峰条数、引用的缩图时间）。
-- state：pending → accepted（建了草稿）/ dismissed（丢弃）/ expired（72 小时没处理）。
--   pending 的候选以 `suggestion:<id>` 引用自己的区间（segment_pins），离开 pending 时撤销。
-- 时间一律是 Unix 毫秒。
CREATE TABLE clip_suggestions (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id  INTEGER NOT NULL REFERENCES stream_sessions (id) ON DELETE CASCADE,
    job_id      INTEGER REFERENCES auto_clip_jobs (id) ON DELETE SET NULL,
    in_ms       INTEGER NOT NULL,
    out_ms      INTEGER NOT NULL,
    title       TEXT    NOT NULL DEFAULT '',
    reason      TEXT    NOT NULL DEFAULT '',
    confidence  REAL,
    tags        TEXT    NOT NULL DEFAULT '[]',
    evidence    TEXT    NOT NULL DEFAULT '{}',
    state       TEXT    NOT NULL DEFAULT 'pending'
        CHECK (state IN ('pending', 'accepted', 'dismissed', 'expired')),
    clip_id     INTEGER REFERENCES clips (id) ON DELETE SET NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    CHECK (in_ms >= 0 AND out_ms > in_ms)
);

CREATE INDEX idx_clip_suggestions_session ON clip_suggestions (session_id, in_ms);
CREATE INDEX idx_clip_suggestions_pending ON clip_suggestions (state, created_at);
