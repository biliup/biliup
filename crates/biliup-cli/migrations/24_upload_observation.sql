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
