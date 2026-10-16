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
