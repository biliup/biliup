-- 切片：在场次时间轴上选出的一段，导出成可以直接投稿的文件。
--
-- in_ms / out_ms 是用户选的入点、出点（场次时间轴，与 segments.start_ms 同一条轴）；
-- cut_in_ms / cut_out_ms 是最近一次导出实际切在哪里：快速剪按关键帧切，入点取之前（含）最近的关键帧、
-- 出点取之后（含）最近的关键帧或分段末尾；精确剪与入点、出点相同。没导出过为 NULL。
-- mode 是最近一次导出的方式，没导出过为 NULL。
-- output_path 是产物相对服务工作目录的路径（clips/<场次>/<切片>.<扩展名>），duration_ms 是产物的媒体时长。
-- template_id / studio_override / archive_bvid / published_at 给发布用。
-- 时间一律是 Unix 毫秒；id 用 AUTOINCREMENT，删掉的切片 id 不复用（产物文件名用它）。
CREATE TABLE clips (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id      INTEGER NOT NULL REFERENCES stream_sessions (id) ON DELETE CASCADE,
    marker_id       INTEGER REFERENCES markers (id) ON DELETE SET NULL,
    in_ms           INTEGER NOT NULL CHECK (in_ms >= 0),
    out_ms          INTEGER NOT NULL CHECK (out_ms > in_ms),
    cut_in_ms       INTEGER,
    cut_out_ms      INTEGER,
    mode            TEXT CHECK (mode IN ('quick', 'precise')),
    title           TEXT    NOT NULL DEFAULT '',
    state           TEXT    NOT NULL DEFAULT 'draft'
        CHECK (state IN ('draft', 'exporting', 'ready', 'failed', 'published', 'discarded')),
    output_path     TEXT,
    output_bytes    INTEGER,
    duration_ms     INTEGER,
    error           TEXT,
    template_id     INTEGER REFERENCES uploadstreamers (id) ON DELETE SET NULL,
    studio_override TEXT,
    archive_bvid    TEXT,
    published_at    INTEGER,
    created_by      INTEGER REFERENCES web_users (id) ON DELETE SET NULL,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

CREATE INDEX idx_clips_session ON clips (session_id, in_ms);
