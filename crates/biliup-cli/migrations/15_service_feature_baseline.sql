-- 本轮服务端功能的合并基线：配置、斗鱼凭据、录制场次、压制、上传观察和活动历史。
-- 1~14 保留原样；新增表直接定义最终结构，不保留开发过程中的加列和修补步骤。
-- 已有的 livestreamers、stream_sessions、segments 仍可能有数据：配置清理、场次 ID
-- 初始化和录制历史回填必须保留。render_jobs 在本迁移中新建，无需回填历史压制任务。

-- 配置语义：清理历史分段时长占位值
-- 历史 ConfigPatch 会把未设置的 segment_time 写成 null，旧语义下这些值继承全局。
-- 新版本允许显式 null 按主播关闭时长分段，并省略未设置的 segment_time。
-- 升级前的 null 只能是旧语义的占位值，删除它们以继续继承全局。
UPDATE livestreamers
SET override = json_remove(override, '$.segment_time')
WHERE override IS NOT NULL
  AND json_valid(override)
  AND json_type(override, '$.segment_time') = 'null';

-- 斗鱼账号：续期凭据与调度状态
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

-- 录制基础：文件起点、场次编号与场次合并
-- Capture file-open time independently of the session's content timeline.
ALTER TABLE segments ADD COLUMN recorded_at_ms INTEGER;

-- Session media is stored under paths that may include the session ID. Keep a durable
-- AUTOINCREMENT high-water mark so deleting a history row cannot reuse its file namespace.
CREATE TABLE stream_session_ids (
    id INTEGER PRIMARY KEY AUTOINCREMENT
);

INSERT INTO stream_session_ids (id)
SELECT id FROM stream_sessions ORDER BY id;

-- Session IDs come from stream_session_ids: open_session() and the group merge allocate through it,
-- so the sequence stays ahead of every row the service writes. No trigger is needed for that.

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

-- 群组与成员的 kind 由写入它们的语句保证（候选查询只取 kind='source'，群组行写成 kind='group'）；
-- 原先用 BEFORE INSERT 触发器兜底，现在由应用侧维持。

-- 录后压制：配方、资源、任务及其引用
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
    updated_at INTEGER NOT NULL,
    encoder TEXT,
    encoder_device TEXT,
    fallback_reason TEXT,
    fps REAL,
    speed REAL,
    processed_ms INTEGER,
    total_ms INTEGER,
    started_at INTEGER,
    finished_at INTEGER
);
CREATE INDEX idx_render_jobs_session ON render_jobs(session_id,id);
CREATE INDEX idx_render_jobs_state_id ON render_jobs(state,id DESC);
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

-- 上传观察：任务与文件快照
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

-- 活动历史：不落表，读取时从业务记录推导
-- 事件（场次开始/结束、分段完成、压制任务的排队/开始/结束、上传任务的登记与当前状态）由
-- GET /v1/activity-events 在读取时从 stream_sessions、segments、render_jobs 和
-- upload_observation_tasks 推导，写入路径不登记、也不需要触发器。代价是历史不再独立于业务
-- 记录：被删掉的记录连同它的事件一起消失，上传任务只知道登记时间与当前状态 —— 压制任务能靠
-- created_at / started_at / finished_at 还原三个节点，是因为它的状态机是单向的。
