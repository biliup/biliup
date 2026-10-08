-- Latest Web login cookie and passport pair are held apart from user config.
-- The original normalized source hash binds them to an exact configured account.
CREATE TABLE douyu_cookie_keeper (
    source_hash TEXT PRIMARY KEY NOT NULL,
    cookie TEXT NOT NULL,
    ltp0 TEXT NOT NULL,
    dy_did TEXT NOT NULL,
    account_id TEXT,
    login_state TEXT NOT NULL DEFAULT 'unknown',
    refresh_state TEXT NOT NULL DEFAULT 'scheduled',
    last_success_at INTEGER,
    last_checked_at INTEGER,
    last_attempt_at INTEGER,
    next_refresh_at INTEGER NOT NULL,
    failure_count INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    lease_token TEXT,
    lease_until INTEGER,
    updated_at INTEGER NOT NULL
);
CREATE INDEX idx_douyu_cookie_keeper_due ON douyu_cookie_keeper(next_refresh_at);
