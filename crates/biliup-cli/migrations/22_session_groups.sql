ALTER TABLE stream_sessions ADD COLUMN kind TEXT NOT NULL DEFAULT 'source'
    CHECK (kind IN ('source', 'group'));

CREATE TABLE session_group_members (
    group_session_id INTEGER NOT NULL REFERENCES stream_sessions(id) ON DELETE CASCADE,
    source_session_id INTEGER NOT NULL UNIQUE REFERENCES stream_sessions(id) ON DELETE RESTRICT,
    ordinal INTEGER NOT NULL,
    offset_ms INTEGER NOT NULL,
    PRIMARY KEY(group_session_id, ordinal),
    CHECK(group_session_id != source_session_id)
);

CREATE TABLE session_merge_candidates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    previous_session_id INTEGER NOT NULL REFERENCES stream_sessions(id) ON DELETE CASCADE,
    source_session_id INTEGER NOT NULL UNIQUE REFERENCES stream_sessions(id) ON DELETE CASCADE,
    state TEXT NOT NULL DEFAULT 'pending' CHECK(state IN ('pending','confirmed','dismissed')),
    group_session_id INTEGER REFERENCES stream_sessions(id) ON DELETE SET NULL,
    created_at INTEGER NOT NULL,
    resolved_at INTEGER
);

CREATE TRIGGER session_group_members_are_sources
BEFORE INSERT ON session_group_members
WHEN (SELECT kind FROM stream_sessions WHERE id=NEW.group_session_id) != 'group'
 OR (SELECT kind FROM stream_sessions WHERE id=NEW.source_session_id) != 'source'
BEGIN
    SELECT RAISE(ABORT, 'session groups must contain real source sessions');
END;
