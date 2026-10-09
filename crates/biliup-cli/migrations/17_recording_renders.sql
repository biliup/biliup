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
