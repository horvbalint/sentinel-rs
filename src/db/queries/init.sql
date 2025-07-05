BEGIN;

CREATE TABLE IF NOT EXISTS usage (
    container CHAR(64),
    cpu_percentage TINYINT NOT NULL,
    memory_total INTEGER NOT NULL,
    memory_used INTEGER NOT NULL,
    memory_percentage TINYINT NOT NULL,
    timestamp DATETIME NOT NULL,
    timestamp_5m TEXT GENERATED ALWAYS AS (datetime(strftime('%s', timestamp) / 10 * 10, 'unixepoch')) STORED,
    timestamp_1h TEXT GENERATED ALWAYS AS (datetime(strftime('%s', timestamp) / 120 * 120, 'unixepoch')) STORED,
    timestamp_1d TEXT GENERATED ALWAYS AS (datetime(strftime('%s', timestamp) / 2880 * 2880, 'unixepoch')) STORED,
    timestamp_1w TEXT GENERATED ALWAYS AS (datetime(strftime('%s', timestamp) / 20160 * 20160, 'unixepoch')) STORED,
    timestamp_30d TEXT GENERATED ALWAYS AS (datetime(strftime('%s', timestamp) / 86400 * 86400, 'unixepoch')) STORED
);

CREATE INDEX IF NOT EXISTS idx_timestamp_container_desc ON usage(timestamp DESC, container);

CREATE INDEX IF NOT EXISTS idx_timestamp_container_asc ON usage(timestamp ASC, container);

COMMIT;