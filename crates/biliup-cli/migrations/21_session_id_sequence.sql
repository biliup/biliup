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
