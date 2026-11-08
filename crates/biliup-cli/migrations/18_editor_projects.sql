-- Editor workspace schema. This feature has not been released; keep its complete schema in this migration.

-- Editing projects and saved revisions are independent of clips/render recipes.
-- Keep every saved revision's media alive so restoring an earlier edit is safe.
CREATE TABLE editor_projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER REFERENCES stream_sessions(id) ON DELETE RESTRICT,
    name TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK (revision > 0),
    document_json TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE INDEX idx_editor_projects_session ON editor_projects(session_id, updated_at);

CREATE TABLE editor_revisions (
    project_id INTEGER NOT NULL REFERENCES editor_projects(id) ON DELETE CASCADE,
    revision INTEGER NOT NULL,
    document_json TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (project_id, revision)
);

CREATE TABLE editor_sources (
    project_id INTEGER NOT NULL REFERENCES editor_projects(id) ON DELETE CASCADE,
    segment_id INTEGER NOT NULL REFERENCES segments(id) ON DELETE RESTRICT,
    PRIMARY KEY (project_id, segment_id)
);
CREATE INDEX idx_editor_sources_segment ON editor_sources(segment_id);

-- Accepting an automatic suggestion saves ordinary timeline items in the same transaction.
-- The receipt survives project deletion just as an accepted clip suggestion does.
CREATE TABLE editor_suggestion_uses (
    suggestion_id INTEGER PRIMARY KEY REFERENCES clip_suggestions(id) ON DELETE CASCADE,
    project_id INTEGER REFERENCES editor_projects(id) ON DELETE SET NULL,
    revision INTEGER NOT NULL,
    item_ids_json TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE INDEX idx_editor_suggestion_project ON editor_suggestion_uses(project_id);

-- Imported files have immutable, content-addressed identities. Project history and
-- queued export snapshots can still resolve a file after it leaves the current library.
CREATE TABLE editor_assets (
    id TEXT PRIMARY KEY,
    path TEXT NOT NULL,
    file_name TEXT NOT NULL,
    kind TEXT NOT NULL CHECK(kind IN ('video','audio','image')),
    duration_ms INTEGER,
    probe_json TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

-- Brand kits, editable skills and brief templates share the existing editor database.
-- Global resources survive project deletion; project resources follow their owner.
CREATE TABLE editor_resources (
    scope TEXT NOT NULL,
    project_id INTEGER REFERENCES editor_projects(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK(kind IN ('kit', 'skill', 'template', 'active')),
    id TEXT NOT NULL,
    value_json TEXT NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY(scope, kind, id),
    CHECK ((scope = 'global' AND project_id IS NULL) OR scope = CAST(project_id AS TEXT))
);
CREATE INDEX editor_resources_project ON editor_resources(project_id);

CREATE TABLE editor_brand_files (
    scope TEXT NOT NULL,
    project_id INTEGER REFERENCES editor_projects(id) ON DELETE CASCADE,
    kit TEXT NOT NULL,
    name TEXT NOT NULL,
    hash TEXT NOT NULL,
    extension TEXT NOT NULL,
    PRIMARY KEY(scope, kit, name),
    CHECK ((scope = 'global' AND project_id IS NULL) OR scope = CAST(project_id AS TEXT))
);
CREATE INDEX editor_brand_files_project ON editor_brand_files(project_id);

CREATE TABLE editor_sound_folders (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE TABLE editor_sound_files (
    folder_id TEXT NOT NULL REFERENCES editor_sound_folders(id) ON DELETE CASCADE,
    asset_id TEXT NOT NULL REFERENCES editor_assets(id),
    name TEXT NOT NULL,
    relative_name TEXT NOT NULL,
    bytes INTEGER NOT NULL,
    PRIMARY KEY(folder_id, asset_id)
);

CREATE TABLE editor_provider_keys (
    provider TEXT PRIMARY KEY,
    ciphertext BLOB NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE editor_model_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    settings_json TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE editor_graphics (
    project_id INTEGER NOT NULL REFERENCES editor_projects(id) ON DELETE CASCADE,
    src TEXT NOT NULL,
    name TEXT NOT NULL,
    html TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (project_id, src)
);

CREATE TABLE editor_project_fonts (
    project_id INTEGER NOT NULL REFERENCES editor_projects(id) ON DELETE CASCADE,
    hash TEXT NOT NULL,
    extension TEXT NOT NULL,
    family TEXT NOT NULL,
    weight TEXT NOT NULL,
    style TEXT NOT NULL,
    name TEXT NOT NULL,
    bytes INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (project_id, hash, family, weight, style)
);

-- An index, not an owner: deleted projects may leave reusable managed files, while
-- recording retention remains governed by the projects that actually reference them.
CREATE TABLE editor_recent_media (
    id TEXT PRIMARY KEY,
    project_id INTEGER NOT NULL,
    project_name TEXT NOT NULL,
    media_json TEXT NOT NULL,
    source_json TEXT NOT NULL,
    origin TEXT NOT NULL CHECK (origin IN ('capture','import','generated')),
    created_at INTEGER NOT NULL
);
CREATE INDEX idx_editor_recent_created ON editor_recent_media(created_at DESC,id);

CREATE TABLE editor_agent_threads (
    project_id INTEGER PRIMARY KEY NOT NULL REFERENCES editor_projects(id) ON DELETE CASCADE,
    started_at INTEGER NOT NULL
);
CREATE TABLE editor_agent_turns (
    id TEXT PRIMARY KEY NOT NULL,
    project_id INTEGER NOT NULL REFERENCES editor_projects(id) ON DELETE CASCADE,
    owner_id INTEGER,
    status TEXT NOT NULL CHECK(status IN ('running','done','error','cancelled')),
    base_revision INTEGER NOT NULL,
    last_revision INTEGER,
    undo_blocked INTEGER NOT NULL DEFAULT 0,
    undone_at INTEGER,
    approval_json TEXT,
    data_json TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE UNIQUE INDEX editor_agent_one_running ON editor_agent_turns(project_id) WHERE status='running';
CREATE INDEX editor_agent_history ON editor_agent_turns(project_id,created_at DESC);
CREATE TABLE editor_agent_edits (
    turn_id TEXT NOT NULL REFERENCES editor_agent_turns(id) ON DELETE CASCADE,
    revision INTEGER NOT NULL,
    summary TEXT NOT NULL,
    lines_json TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY(turn_id,revision)
);

CREATE TABLE editor_agent_plans (
    project_id INTEGER PRIMARY KEY NOT NULL REFERENCES editor_projects(id) ON DELETE CASCADE,
    revision INTEGER NOT NULL,
    data_json TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE TABLE editor_agent_images (
    id TEXT PRIMARY KEY NOT NULL,
    project_id INTEGER NOT NULL REFERENCES editor_projects(id) ON DELETE CASCADE,
    turn_id TEXT NOT NULL REFERENCES editor_agent_turns(id) ON DELETE CASCADE,
    revision INTEGER NOT NULL,
    jpeg BLOB NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE INDEX editor_agent_images_project ON editor_agent_images(project_id,created_at DESC);

CREATE TABLE editor_agent_files (
    id TEXT PRIMARY KEY NOT NULL,
    project_id INTEGER NOT NULL REFERENCES editor_projects(id) ON DELETE CASCADE,
    turn_id TEXT NOT NULL REFERENCES editor_agent_turns(id) ON DELETE CASCADE,
    revision INTEGER NOT NULL,
    name TEXT COLLATE NOCASE NOT NULL,
    format TEXT NOT NULL CHECK(format IN ('srt', 'vtt')),
    cues INTEGER NOT NULL,
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE(project_id, name)
);
CREATE INDEX editor_agent_files_turn ON editor_agent_files(turn_id, created_at);

-- Export snapshots and completed files survive deletion of their editing project.
ALTER TABLE render_jobs ADD COLUMN retry_of INTEGER REFERENCES render_jobs(id) ON DELETE SET NULL;
CREATE UNIQUE INDEX render_jobs_retry ON render_jobs(retry_of) WHERE retry_of IS NOT NULL;
CREATE TABLE editor_exports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL,
    revision INTEGER NOT NULL,
    phase TEXT NOT NULL,
    job_json TEXT NOT NULL,
    document_json TEXT NOT NULL,
    options_json TEXT NOT NULL,
    brand_json TEXT,
    agent_turn_id TEXT REFERENCES editor_agent_turns(id) ON DELETE SET NULL,
    request_key TEXT,
    retry_of INTEGER UNIQUE REFERENCES editor_exports(id),
    created_at INTEGER NOT NULL
);
CREATE INDEX idx_editor_exports_project ON editor_exports(project_id, id DESC);
CREATE INDEX idx_editor_exports_created ON editor_exports(created_at DESC, id DESC);
CREATE UNIQUE INDEX editor_exports_agent_request ON editor_exports(project_id,agent_turn_id,request_key)
    WHERE agent_turn_id IS NOT NULL AND request_key IS NOT NULL;

-- A confirmed output publication has one durable receipt and uses the shared uploader.
-- Drafts freeze the exact metadata reviewed by the user; they never contain account secrets.
CREATE TABLE output_covers (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL CHECK(kind IN ('editor','render')),
    output_id INTEGER NOT NULL,
    source_stamp TEXT NOT NULL,
    source TEXT NOT NULL CHECK(source IN ('template','upload','frame')),
    name TEXT NOT NULL,
    at_ms INTEGER,
    jpeg BLOB NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE INDEX output_covers_output ON output_covers(kind,output_id,updated_at);
CREATE TABLE output_publications (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL CHECK(kind IN ('editor','render')),
    output_id INTEGER NOT NULL,
    template_id INTEGER NOT NULL,
    template_hash TEXT NOT NULL,
    source_stamp TEXT NOT NULL,
    cover_id TEXT REFERENCES output_covers(id) ON DELETE RESTRICT,
    studio_json TEXT NOT NULL,
    previous_id TEXT,
    state TEXT NOT NULL CHECK(state IN ('draft','uploading','submitting','succeeded','failed','unknown')),
    task_id TEXT,
    bvid TEXT,
    error TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE INDEX output_publications_output ON output_publications(kind,output_id,created_at DESC,id DESC);
CREATE UNIQUE INDEX output_publications_active ON output_publications(kind,output_id)
    WHERE state IN ('uploading','submitting');

CREATE TABLE editor_generation_jobs (
    id TEXT PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES editor_projects(id) ON DELETE CASCADE,
    agent_turn_id TEXT REFERENCES editor_agent_turns(id) ON DELETE SET NULL,
    request_json TEXT NOT NULL,
    phase TEXT NOT NULL CHECK (phase IN ('running','importing','done','failed','cancelled')),
    status TEXT NOT NULL,
    error TEXT,
    result_json TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE INDEX editor_generation_project ON editor_generation_jobs(project_id, created_at DESC);
CREATE INDEX editor_generation_agent_turn ON editor_generation_jobs(agent_turn_id);

CREATE TABLE editor_shaders (
    project_id INTEGER NOT NULL REFERENCES editor_projects(id) ON DELETE CASCADE,
    id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    definition_json TEXT NOT NULL,
    body TEXT NOT NULL,
    tested INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY(project_id,id)
);

CREATE TABLE editor_analysis_profiles (
  project_id INTEGER NOT NULL REFERENCES editor_projects(id) ON DELETE CASCADE,
  media_id TEXT NOT NULL,
  version TEXT NOT NULL,
  analyzer TEXT NOT NULL,
  model TEXT NOT NULL DEFAULT '',
  language TEXT NOT NULL DEFAULT 'auto',
  cache_key TEXT NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(project_id,media_id,version,analyzer,model,language)
);
CREATE TABLE editor_analysis_restores (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id INTEGER NOT NULL REFERENCES editor_projects(id) ON DELETE CASCADE,
  media_id TEXT NOT NULL,
  version TEXT NOT NULL,
  previous_version TEXT NOT NULL,
  previous_source_json TEXT NOT NULL,
  phase TEXT NOT NULL DEFAULT 'queued',
  error TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE INDEX editor_analysis_restores_pending ON editor_analysis_restores(phase,id);

-- Shared brand/skill changes must refresh every editor that uses the library.
CREATE TABLE editor_resource_state (id INTEGER PRIMARY KEY CHECK(id=1), version INTEGER NOT NULL);
INSERT INTO editor_resource_state(id,version) VALUES(1,0);
CREATE TRIGGER editor_resource_insert AFTER INSERT ON editor_resources BEGIN
    UPDATE editor_resource_state SET version=version+1 WHERE id=1;
END;
CREATE TRIGGER editor_resource_update AFTER UPDATE ON editor_resources BEGIN
    UPDATE editor_resource_state SET version=version+1 WHERE id=1;
END;
CREATE TRIGGER editor_resource_delete AFTER DELETE ON editor_resources BEGIN
    UPDATE editor_resource_state SET version=version+1 WHERE id=1;
END;
CREATE TRIGGER editor_brand_file_insert AFTER INSERT ON editor_brand_files BEGIN
    UPDATE editor_resource_state SET version=version+1 WHERE id=1;
END;
CREATE TRIGGER editor_brand_file_update AFTER UPDATE ON editor_brand_files BEGIN
    UPDATE editor_resource_state SET version=version+1 WHERE id=1;
END;
CREATE TRIGGER editor_brand_file_delete AFTER DELETE ON editor_brand_files BEGIN
    UPDATE editor_resource_state SET version=version+1 WHERE id=1;
END;

-- Immutable, bounded source snapshots for paged assistant reading.
CREATE TABLE editor_web_sources (
    id TEXT PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES editor_projects(id) ON DELETE CASCADE,
    turn_id TEXT REFERENCES editor_agent_turns(id) ON DELETE SET NULL,
    requested_url TEXT NOT NULL,
    final_url TEXT NOT NULL,
    content_type TEXT NOT NULL,
    data_json TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE INDEX editor_web_sources_project ON editor_web_sources(project_id,created_at);

CREATE TABLE editor_downloads (
    id TEXT PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES editor_projects(id) ON DELETE CASCADE,
    agent_turn_id TEXT REFERENCES editor_agent_turns(id) ON DELETE SET NULL,
    request_key TEXT NOT NULL,
    source TEXT NOT NULL,
    name TEXT,
    phase TEXT NOT NULL DEFAULT 'queued',
    result_json TEXT,
    bytes INTEGER NOT NULL DEFAULT 0,
    error TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE INDEX editor_downloads_project ON editor_downloads(project_id,created_at);
CREATE INDEX editor_downloads_request ON editor_downloads(project_id,agent_turn_id,request_key);
CREATE TRIGGER editor_project_font_insert AFTER INSERT ON editor_project_fonts BEGIN
    UPDATE editor_resource_state SET version=version+1 WHERE id=1;
END;
CREATE TRIGGER editor_project_font_delete AFTER DELETE ON editor_project_fonts BEGIN
    UPDATE editor_resource_state SET version=version+1 WHERE id=1;
END;

-- Human-readable history belongs to the saved revision, not a browser session.
CREATE TABLE editor_revision_notes (
    project_id INTEGER NOT NULL,
    revision INTEGER NOT NULL,
    kind TEXT NOT NULL,
    author TEXT NOT NULL,
    label TEXT NOT NULL,
    lines_json TEXT NOT NULL DEFAULT '[]',
    turn_id TEXT REFERENCES editor_agent_turns(id) ON DELETE SET NULL,
    PRIMARY KEY (project_id, revision),
    FOREIGN KEY (project_id, revision) REFERENCES editor_revisions(project_id, revision) ON DELETE CASCADE
);
CREATE INDEX editor_agent_history_cursor ON editor_agent_turns(project_id,created_at DESC,id DESC);

CREATE TABLE editor_history_cursor (
    project_id INTEGER PRIMARY KEY REFERENCES editor_projects(id) ON DELETE CASCADE,
    head_revision INTEGER NOT NULL,
    branch_revision INTEGER NOT NULL DEFAULT 1,
    past_json TEXT NOT NULL DEFAULT '[]',
    future_json TEXT NOT NULL DEFAULT '[]'
);
CREATE TABLE editor_history_links (
    project_id INTEGER NOT NULL,
    revision INTEGER NOT NULL,
    parent_revision INTEGER,
    is_alias INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (project_id,revision),
    FOREIGN KEY (project_id,revision) REFERENCES editor_revisions(project_id,revision) ON DELETE CASCADE
);
