-- Capture file-open time independently of the session's content timeline.
ALTER TABLE segments ADD COLUMN recorded_at_ms INTEGER;
