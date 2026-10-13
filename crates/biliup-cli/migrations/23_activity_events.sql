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
