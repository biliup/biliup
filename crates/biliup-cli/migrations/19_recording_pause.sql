ALTER TABLE livestreamers ADD COLUMN recording_paused INTEGER NOT NULL DEFAULT 0
    CHECK (recording_paused IN (0, 1));
