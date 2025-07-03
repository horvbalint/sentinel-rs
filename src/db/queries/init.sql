BEGIN;

CREATE TABLE IF NOT EXISTS usage (
    timestamp DATETIME NOT NULL,
    container CHAR(64),
    cpu_percentage REAL NOT NULL,
    memory_total INTEGER NOT NULL,
    memory_used INTEGER NOT NULL,
    memory_percentage REAL NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_timestamp_container_desc ON usage(timestamp DESC, container);

CREATE INDEX IF NOT EXISTS idx_timestamp_container_asc ON usage(timestamp ASC, container);

COMMIT;