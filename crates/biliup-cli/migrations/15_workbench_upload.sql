-- 录后合成工作台、会话合并与上传队列的 schema，一次建齐。
--
-- 本文件由分支上从未发布的 15~24 号迁移合并而来，内容与它们按序执行完全一致：
--   15_douyu_cookie_keeper, 16_override_segment_time_placeholder_null, 17_recording_renders,
--   18_recording_time_origins, 19_render_telemetry, 20_render_cancelled_timestamps,
--   21_session_id_sequence, 22_session_groups, 23_activity_events, 24_upload_observation
-- 已发布过的 1~14 号迁移是只读历史：sqlx 用文件内容的 SHA-384 校验已应用的迁移，
-- 就地改写它们会让所有老库启动失败。后续改动一律新增迁移文件。
--
-- 其中的数据订正语句（UPDATE / INSERT ... SELECT）针对的是分支上更早构建留下的数据。
-- 对上游 v1.2.x 的库来说，这些表都是本文件新建且为空的，语句不会命中任何行。


-- ===== 15_douyu_cookie_keeper.sql =====
-- Latest Web login cookie and passport pair are held apart from user config.
-- The original normalized source hash binds them to an exact configured account.
CREATE TABLE douyu_cookie_keeper (
    source_hash TEXT PRIMARY KEY NOT NULL,
    cookie TEXT NOT NULL,
    ltp0 TEXT NOT NULL,
    dy_did TEXT NOT NULL,
    account_id TEXT,
    login_state TEXT NOT NULL DEFAULT 'unknown',
    refresh_state TEXT NOT NULL DEFAULT 'scheduled',
    last_success_at INTEGER,
    last_checked_at INTEGER,
    last_attempt_at INTEGER,
    next_refresh_at INTEGER NOT NULL,
    failure_count INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    lease_token TEXT,
    lease_until INTEGER,
    updated_at INTEGER NOT NULL
);
CREATE INDEX idx_douyu_cookie_keeper_due ON douyu_cookie_keeper(next_refresh_at);

-- ===== 16_override_segment_time_placeholder_null.sql =====
-- 历史 ConfigPatch 会把未设置的 segment_time 写成 null，旧语义下这些值继承全局。
-- 新版本允许显式 null 按主播关闭时长分段，并省略未设置的 segment_time。
-- 升级前的 null 只能是旧语义的占位值，删除它们以继续继承全局。
UPDATE livestreamers
SET override = json_remove(override, '$.segment_time')
WHERE override IS NOT NULL
  AND json_valid(override)
  AND json_type(override, '$.segment_time') = 'null';

-- ===== 17_recording_renders.sql =====
-- Immutable post-recording composition recipes, assets and queued jobs.
CREATE TABLE render_recipes (
    session_id INTEGER PRIMARY KEY REFERENCES stream_sessions(id) ON DELETE CASCADE,
    recipe_json TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE TABLE render_assets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL REFERENCES stream_sessions(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    mime TEXT NOT NULL CHECK (mime IN ('image/png','image/jpeg')),
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    sha256 TEXT NOT NULL,
    bytes INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE INDEX idx_render_assets_session ON render_assets(session_id);
CREATE TABLE render_recipe_assets (
    session_id INTEGER NOT NULL REFERENCES render_recipes(session_id) ON DELETE CASCADE,
    asset_id INTEGER NOT NULL REFERENCES render_assets(id) ON DELETE RESTRICT,
    PRIMARY KEY(session_id,asset_id)
);
CREATE TABLE render_jobs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL REFERENCES stream_sessions(id) ON DELETE CASCADE,
    clip_id INTEGER REFERENCES clips(id) ON DELETE SET NULL,
    state TEXT NOT NULL CHECK (state IN ('queued','running','ready','failed','cancelled')),
    phase TEXT NOT NULL,
    ratio REAL,
    spec_json TEXT NOT NULL,
    output_path TEXT,
    output_bytes INTEGER,
    duration_ms INTEGER,
    error TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE INDEX idx_render_jobs_session ON render_jobs(session_id,id);
CREATE UNIQUE INDEX idx_render_jobs_active_clip ON render_jobs(clip_id)
    WHERE clip_id IS NOT NULL AND state IN ('queued','running');
CREATE TABLE render_sources (
    job_id INTEGER NOT NULL REFERENCES render_jobs(id) ON DELETE CASCADE,
    segment_id INTEGER NOT NULL REFERENCES segments(id) ON DELETE RESTRICT,
    PRIMARY KEY(job_id,segment_id)
);
CREATE INDEX idx_render_sources_segment ON render_sources(segment_id,job_id);
CREATE TABLE render_job_assets (
    job_id INTEGER NOT NULL REFERENCES render_jobs(id) ON DELETE CASCADE,
    asset_id INTEGER NOT NULL REFERENCES render_assets(id) ON DELETE RESTRICT,
    PRIMARY KEY(job_id,asset_id)
);

ALTER TABLE clips ADD COLUMN active_render_id INTEGER;

-- ===== 18_recording_time_origins.sql =====
-- Capture file-open time independently of the session's content timeline.
ALTER TABLE segments ADD COLUMN recorded_at_ms INTEGER;

-- ===== 19_render_telemetry.sql =====
-- Keep completed job diagnostics and timing across restarts.
ALTER TABLE render_jobs ADD COLUMN encoder TEXT;
ALTER TABLE render_jobs ADD COLUMN encoder_device TEXT;
ALTER TABLE render_jobs ADD COLUMN fallback_reason TEXT;
ALTER TABLE render_jobs ADD COLUMN fps REAL;
ALTER TABLE render_jobs ADD COLUMN speed REAL;
ALTER TABLE render_jobs ADD COLUMN processed_ms INTEGER;
ALTER TABLE render_jobs ADD COLUMN total_ms INTEGER;
ALTER TABLE render_jobs ADD COLUMN started_at INTEGER;
ALTER TABLE render_jobs ADD COLUMN finished_at INTEGER;
-- Existing jobs have no trustworthy start time; retain null rather than inventing elapsed time.
UPDATE render_jobs SET finished_at=updated_at WHERE state IN ('ready','failed','cancelled');
CREATE INDEX idx_render_jobs_state_id ON render_jobs(state,id DESC);

-- ===== 20_render_cancelled_timestamps.sql =====
-- Legacy cancelled jobs have no trustworthy start time, so do not report a fabricated elapsed time.
UPDATE render_jobs
SET finished_at=NULL
WHERE state='cancelled' AND started_at IS NULL;

-- ===== 21_session_id_sequence.sql =====
-- Session media is stored under paths that may include the session ID. Keep a durable
-- AUTOINCREMENT high-water mark so deleting a history row cannot reuse its file namespace.
CREATE TABLE stream_session_ids (
    id INTEGER PRIMARY KEY AUTOINCREMENT
);

INSERT INTO stream_session_ids (id)
SELECT id FROM stream_sessions ORDER BY id;

-- Keep the sequence ahead of IDs written by legacy code, imports, and test fixtures that insert
-- directly into stream_sessions instead of going through open_session().
CREATE TRIGGER reserve_stream_session_id
AFTER INSERT ON stream_sessions
WHEN NEW.id > COALESCE((SELECT seq FROM sqlite_sequence WHERE name = 'stream_session_ids'), 0)
BEGIN
    INSERT INTO stream_session_ids (id) VALUES (NEW.id);
END;

-- ===== 22_session_groups.sql =====
ALTER TABLE stream_sessions ADD COLUMN kind TEXT NOT NULL DEFAULT 'source'
    CHECK (kind IN ('source', 'group'));

CREATE TABLE session_group_members (
    group_session_id INTEGER NOT NULL REFERENCES stream_sessions(id) ON DELETE CASCADE,
    source_session_id INTEGER NOT NULL UNIQUE REFERENCES stream_sessions(id) ON DELETE RESTRICT,
    ordinal INTEGER NOT NULL,
    offset_ms INTEGER NOT NULL,
    PRIMARY KEY(group_session_id, ordinal),
    CHECK(group_session_id != source_session_id)
);

CREATE TABLE session_merge_candidates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    previous_session_id INTEGER NOT NULL REFERENCES stream_sessions(id) ON DELETE CASCADE,
    source_session_id INTEGER NOT NULL UNIQUE REFERENCES stream_sessions(id) ON DELETE CASCADE,
    state TEXT NOT NULL DEFAULT 'pending' CHECK(state IN ('pending','confirmed','dismissed')),
    group_session_id INTEGER REFERENCES stream_sessions(id) ON DELETE SET NULL,
    created_at INTEGER NOT NULL,
    resolved_at INTEGER
);

CREATE TRIGGER session_group_members_are_sources
BEFORE INSERT ON session_group_members
WHEN (SELECT kind FROM stream_sessions WHERE id=NEW.group_session_id) != 'group'
 OR (SELECT kind FROM stream_sessions WHERE id=NEW.source_session_id) != 'source'
BEGIN
    SELECT RAISE(ABORT, 'session groups must contain real source sessions');
END;

-- ===== 23_activity_events.sql =====
-- Durable activity history. Deliberately no foreign keys: deleting a business
-- object must not erase the history of what the service actually did.
CREATE TABLE activity_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_key TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL,
    ts INTEGER NOT NULL,
    text TEXT NOT NULL,
    sub TEXT,
    session_id INTEGER,
    entity_id INTEGER
);
CREATE INDEX activity_events_recent ON activity_events(ts DESC,id DESC);

-- Only database timestamps are suitable for historical events. A file's mtime
-- can change after recording and is not an activity timestamp.
INSERT OR IGNORE INTO activity_events(event_key,kind,ts,text,sub,session_id,entity_id)
SELECT 'session:' || id || ':start', 'start',
    COALESCE(started_at,CAST(ROUND((julianday(date)-2440587.5)*86400000) AS INTEGER)),
    name || ' 开始录制', NULL, id, id
FROM stream_sessions WHERE kind='source'
    AND COALESCE(started_at,CAST(ROUND((julianday(date)-2440587.5)*86400000) AS INTEGER)) > 0;
INSERT OR IGNORE INTO activity_events(event_key,kind,ts,text,sub,session_id,entity_id)
SELECT 'session:' || id || ':end:' || ended_at, 'end', ended_at,
    name || ' 结束录制', NULL, id, id
FROM stream_sessions WHERE kind='source' AND ended_at > 0;
INSERT OR IGNORE INTO activity_events(event_key,kind,ts,text,sub,session_id,entity_id)
SELECT 'segment:' || g.session_id || ':' || g.id || ':finished:' || g.start_ms || ':' || g.path,
    'file', s.started_at + g.end_ms, '录制分段已完成', g.path, g.session_id, g.id
FROM segments g JOIN stream_sessions s ON s.id=g.session_id
WHERE s.kind='source' AND g.state='finished' AND s.started_at > 0 AND g.end_ms IS NOT NULL;
INSERT OR IGNORE INTO activity_events(event_key,kind,ts,text,sub,session_id,entity_id)
SELECT 'render:' || id || ':' || state || ':' || updated_at, 'render',
    COALESCE(finished_at,updated_at), '压制任务 #' || id,
    CASE state WHEN 'queued' THEN '等待压制' WHEN 'running' THEN '正在压制'
        WHEN 'ready' THEN '压制完成' WHEN 'failed' THEN '压制失败' WHEN 'cancelled' THEN '压制已取消' END,
    session_id, id
FROM render_jobs WHERE COALESCE(finished_at,updated_at) > 0;

CREATE TRIGGER activity_session_started_insert AFTER INSERT ON stream_sessions
WHEN NEW.kind='source' AND NEW.started_at IS NOT NULL
BEGIN
    INSERT OR IGNORE INTO activity_events(event_key,kind,ts,text,session_id,entity_id)
    VALUES('session:' || NEW.id || ':start','start',NEW.started_at,NEW.name || ' 开始录制',NEW.id,NEW.id);
END;
CREATE TRIGGER activity_session_started_update AFTER UPDATE OF started_at ON stream_sessions
WHEN NEW.kind='source' AND OLD.started_at IS NULL AND NEW.started_at IS NOT NULL
BEGIN
    INSERT OR IGNORE INTO activity_events(event_key,kind,ts,text,session_id,entity_id)
    VALUES('session:' || NEW.id || ':start','start',NEW.started_at,NEW.name || ' 开始录制',NEW.id,NEW.id);
END;
CREATE TRIGGER activity_session_ended_insert AFTER INSERT ON stream_sessions
WHEN NEW.kind='source' AND NEW.ended_at IS NOT NULL
BEGIN
    INSERT OR IGNORE INTO activity_events(event_key,kind,ts,text,session_id,entity_id)
    VALUES('session:' || NEW.id || ':end:' || NEW.ended_at,'end',NEW.ended_at,NEW.name || ' 结束录制',NEW.id,NEW.id);
END;
CREATE TRIGGER activity_session_ended_update AFTER UPDATE OF ended_at ON stream_sessions
WHEN NEW.kind='source' AND NEW.ended_at IS NOT NULL AND OLD.ended_at IS NOT NEW.ended_at
BEGIN
    INSERT OR IGNORE INTO activity_events(event_key,kind,ts,text,session_id,entity_id)
    VALUES('session:' || NEW.id || ':end:' || NEW.ended_at,'end',NEW.ended_at,NEW.name || ' 结束录制',NEW.id,NEW.id);
END;
CREATE TRIGGER activity_segment_finished_insert AFTER INSERT ON segments
WHEN NEW.state='finished' AND (SELECT kind FROM stream_sessions WHERE id=NEW.session_id)='source'
BEGIN
    INSERT OR IGNORE INTO activity_events(event_key,kind,ts,text,sub,session_id,entity_id)
    VALUES('segment:' || NEW.session_id || ':' || NEW.id || ':finished:' || NEW.start_ms || ':' || NEW.path,
        'file',CAST(ROUND((julianday('now')-2440587.5)*86400000) AS INTEGER),
        '录制分段已完成',NEW.path,NEW.session_id,NEW.id);
END;
CREATE TRIGGER activity_segment_finished_update AFTER UPDATE OF state ON segments
WHEN OLD.state<>'finished' AND NEW.state='finished'
    AND (SELECT kind FROM stream_sessions WHERE id=NEW.session_id)='source'
BEGIN
    INSERT OR IGNORE INTO activity_events(event_key,kind,ts,text,sub,session_id,entity_id)
    VALUES('segment:' || NEW.session_id || ':' || NEW.id || ':finished:' || NEW.start_ms || ':' || NEW.path,
        'file',CAST(ROUND((julianday('now')-2440587.5)*86400000) AS INTEGER),
        '录制分段已完成',NEW.path,NEW.session_id,NEW.id);
END;
CREATE TRIGGER activity_render_created AFTER INSERT ON render_jobs
BEGIN
    INSERT OR IGNORE INTO activity_events(event_key,kind,ts,text,sub,session_id,entity_id)
    VALUES('render:' || NEW.id || ':' || NEW.state || ':' || NEW.updated_at,'render',NEW.updated_at,
        '压制任务 #' || NEW.id,
        CASE NEW.state WHEN 'queued' THEN '等待压制' WHEN 'running' THEN '正在压制'
            WHEN 'ready' THEN '压制完成' WHEN 'failed' THEN '压制失败' WHEN 'cancelled' THEN '压制已取消' END,
        NEW.session_id,NEW.id);
END;
CREATE TRIGGER activity_render_state AFTER UPDATE OF state ON render_jobs
WHEN OLD.state<>NEW.state
BEGIN
    INSERT OR IGNORE INTO activity_events(event_key,kind,ts,text,sub,session_id,entity_id)
    VALUES('render:' || NEW.id || ':' || NEW.state || ':' || NEW.updated_at,'render',NEW.updated_at,
        '压制任务 #' || NEW.id,
        CASE NEW.state WHEN 'queued' THEN '等待压制' WHEN 'running' THEN '正在压制'
            WHEN 'ready' THEN '压制完成' WHEN 'failed' THEN '压制失败' WHEN 'cancelled' THEN '压制已取消' END,
        NEW.session_id,NEW.id);
END;

-- ===== 24_upload_observation.sql =====
CREATE TABLE upload_observation_tasks (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL,
    scheduler_group TEXT NOT NULL,
    task_order INTEGER NOT NULL,
    state TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    snapshot TEXT NOT NULL
);
CREATE INDEX upload_observation_history ON upload_observation_tasks(updated_at DESC);
CREATE INDEX upload_observation_filter ON upload_observation_tasks(source,state);
CREATE TABLE upload_observation_files (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES upload_observation_tasks(id) ON DELETE CASCADE,
    file_order INTEGER NOT NULL,
    snapshot TEXT NOT NULL
);
CREATE INDEX upload_observation_file_order ON upload_observation_files(task_id,file_order);

CREATE TRIGGER upload_observation_created AFTER INSERT ON upload_observation_tasks
BEGIN
    INSERT OR IGNORE INTO activity_events(event_key,kind,ts,text,sub,session_id)
    VALUES('upload:' || NEW.id || ':created', 'upload_queued', NEW.created_at,
        json_extract(NEW.snapshot,'$.title'), '上传任务已登记', json_extract(NEW.snapshot,'$.session_id'));
END;
CREATE TRIGGER upload_observation_terminal_insert AFTER INSERT ON upload_observation_tasks
WHEN NEW.state IN ('succeeded','partial','failed','cancelled','interrupted','unknown','paused','skipped')
BEGIN
    INSERT OR IGNORE INTO activity_events(event_key,kind,ts,text,sub,session_id)
    VALUES('upload:' || NEW.id || ':' || NEW.state || ':' || NEW.updated_at,
        'upload_' || NEW.state, NEW.updated_at, json_extract(NEW.snapshot,'$.title'),
        CASE NEW.state
            WHEN 'succeeded' THEN '投稿成功'
            WHEN 'partial' THEN '部分文件投稿成功'
            WHEN 'failed' THEN '上传失败'
            WHEN 'cancelled' THEN '上传取消'
            WHEN 'interrupted' THEN '上传中断'
            WHEN 'unknown' THEN '投稿结果未知'
            WHEN 'paused' THEN '上传暂停'
            WHEN 'skipped' THEN '未执行投稿'
            ELSE NEW.state END,
        json_extract(NEW.snapshot,'$.session_id'));
END;
CREATE TRIGGER upload_observation_state AFTER UPDATE OF state ON upload_observation_tasks
WHEN OLD.state <> NEW.state
BEGIN
    INSERT OR IGNORE INTO activity_events(event_key,kind,ts,text,sub,session_id)
    VALUES('upload:' || NEW.id || ':' || NEW.state || ':' || NEW.updated_at,
        'upload_' || NEW.state, NEW.updated_at, json_extract(NEW.snapshot,'$.title'),
        CASE NEW.state
            WHEN 'queued' THEN '等待上传'
            WHEN 'running' THEN '正在处理上传'
            WHEN 'retrying' THEN '上传重试'
            WHEN 'submitting' THEN '正在提交稿件'
            WHEN 'succeeded' THEN '投稿成功'
            WHEN 'partial' THEN '部分文件投稿成功'
            WHEN 'failed' THEN '上传失败'
            WHEN 'cancelled' THEN '上传取消'
            WHEN 'interrupted' THEN '上传中断'
            WHEN 'unknown' THEN '投稿结果未知'
            WHEN 'paused' THEN '上传暂停'
            WHEN 'skipped' THEN '未执行投稿'
            ELSE NEW.state END,
        json_extract(NEW.snapshot,'$.session_id'));
END;
