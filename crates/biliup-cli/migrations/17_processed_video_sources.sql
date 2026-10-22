-- Native segment processing replaces the upload path but keeps its inputs on
-- disk. Persist their ownership so rm and deferred cleanup remove them together
-- with the final video, including after a restart. Do not infer old ownership
-- from filenames: existing untracked recordings must remain untouched.
CREATE TABLE processed_video_sources (
    output_path TEXT NOT NULL,
    source_path TEXT NOT NULL,
    PRIMARY KEY (output_path, source_path),
    CHECK (output_path != source_path)
);
CREATE INDEX idx_processed_video_sources_source ON processed_video_sources(source_path);
