-- Job queue. Workers claim pending jobs whose visible_at is in the past.
CREATE TABLE IF NOT EXISTS jobs (
    id              TEXT PRIMARY KEY,
    route_id        TEXT NOT NULL,
    source_path     TEXT NOT NULL,
    raw_payload     TEXT NOT NULL,
    status          TEXT NOT NULL CHECK (status IN ('pending', 'processing', 'done', 'dead')),
    attempts        INTEGER NOT NULL DEFAULT 0,
    last_error      TEXT,
    last_response_code INTEGER,
    visible_at      TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_jobs_pending
    ON jobs (status, visible_at)
    WHERE status = 'pending';

-- Audit log: one row per delivery attempt, success or failure.
CREATE TABLE IF NOT EXISTS deliveries (
    id                   TEXT PRIMARY KEY,
    job_id               TEXT NOT NULL REFERENCES jobs(id),
    route_id             TEXT NOT NULL,
    attempt              INTEGER NOT NULL,
    transformed_payload  TEXT,
    destination_url      TEXT NOT NULL,
    success              INTEGER NOT NULL,
    response_code        INTEGER,
    response_body        TEXT,
    error                TEXT,
    duration_ms          INTEGER NOT NULL,
    created_at           TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_deliveries_job ON deliveries (job_id, created_at DESC);
