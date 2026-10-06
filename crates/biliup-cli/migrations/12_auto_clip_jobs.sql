-- 自动切片（实验）的按场次后台任务。没配置 `auto_clip` 时这张表一直是空的。
--
-- trigger：auto = 下播后按主播开关自动排的，manual = 在界面上手动点的。
-- state：queued → running → done / failed / canceled；同一场同时只能有一个 queued / running。
-- stage：跑到哪一步（audio 抽音频与静音检测、asr 转写；signals / analyze 留给候选生成）；还没开始为 NULL。
--   服务重启时 running 回到 queued，按 stage 和场次目录下的缓存续跑，已转写的块不重传。
-- progress_done / progress_total：本阶段做完几项 / 共几项（抽音频按分段、转写按块）。
-- not_before：到这个时刻才跑（自动任务要等过断流合并窗口）。
-- asr_planned_seconds：静音跳过后这次要送转写的秒数；asr_seconds：实际已送的秒数。
-- tokens_in / tokens_out / images：chat 的实际用量（候选生成用）。
-- models：用的模型与服务地址的主机名（JSON，不含 key）；warnings：跳过的分段等提示（JSON 字符串数组）。
-- reuse_transcript：已有的转写结果直接用，不重复花转写的钱。
-- 时间一律是 Unix 毫秒。
CREATE TABLE auto_clip_jobs (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id          INTEGER NOT NULL REFERENCES stream_sessions (id) ON DELETE CASCADE,
    trigger             TEXT    NOT NULL CHECK (trigger IN ('auto', 'manual')),
    state               TEXT    NOT NULL DEFAULT 'queued'
        CHECK (state IN ('queued', 'running', 'done', 'failed', 'canceled')),
    stage               TEXT CHECK (stage IN ('audio', 'asr', 'signals', 'analyze')),
    progress_done       INTEGER NOT NULL DEFAULT 0,
    progress_total      INTEGER NOT NULL DEFAULT 0,
    not_before          INTEGER NOT NULL,
    error               TEXT,
    asr_planned_seconds INTEGER,
    asr_seconds         INTEGER NOT NULL DEFAULT 0,
    tokens_in           INTEGER NOT NULL DEFAULT 0,
    tokens_out          INTEGER NOT NULL DEFAULT 0,
    images              INTEGER NOT NULL DEFAULT 0,
    models              TEXT,
    warnings            TEXT,
    reuse_transcript    INTEGER NOT NULL DEFAULT 1,
    created_by          INTEGER REFERENCES web_users (id) ON DELETE SET NULL,
    created_at          INTEGER NOT NULL,
    started_at          INTEGER,
    finished_at         INTEGER
);

CREATE INDEX idx_auto_clip_jobs_queue ON auto_clip_jobs (state, not_before, id);
CREATE INDEX idx_auto_clip_jobs_session ON auto_clip_jobs (session_id, id);
CREATE UNIQUE INDEX idx_auto_clip_jobs_active ON auto_clip_jobs (session_id)
    WHERE state IN ('queued', 'running');
