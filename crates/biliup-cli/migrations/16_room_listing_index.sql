CREATE INDEX IF NOT EXISTS idx_sessions_room_latest
    ON stream_sessions(url, kind, date DESC, id DESC);
