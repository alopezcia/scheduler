-- no-transaction
-- Añade 'cancelled' a los estados de `command_executions`: una ejecución en curso puede
-- cancelarse desde el frontend (frontend -> backend por WebSocket -> scheduler por socket
-- Unix). SQLite no permite modificar un CHECK sin recrear la tabla; mismo patrón que la
-- migración 0004 (sin transacción implícita y con foreign keys apagadas mientras se reconstruye).

PRAGMA foreign_keys = OFF;

BEGIN;

CREATE TABLE command_executions_new (
    id TEXT PRIMARY KEY,
    schedule_id TEXT REFERENCES schedules(id) ON DELETE SET NULL,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    requested_value TEXT NOT NULL,
    requested_by TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'sent', 'ack', 'failed', 'timeout', 'cancelled')),
    sent_at TEXT,
    ack_at TEXT,
    response TEXT,
    error_message TEXT,
    latency_ms INTEGER,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    event_id TEXT REFERENCES events(id) ON DELETE SET NULL
);

INSERT INTO command_executions_new
    (id, schedule_id, tag_id, requested_value, requested_by, status, sent_at, ack_at, response, error_message, latency_ms, created_at, event_id)
SELECT id, schedule_id, tag_id, requested_value, requested_by, status, sent_at, ack_at, response, error_message, latency_ms, created_at, event_id
FROM command_executions;

DROP TABLE command_executions;
ALTER TABLE command_executions_new RENAME TO command_executions;

CREATE INDEX IF NOT EXISTS idx_command_executions_tag_id ON command_executions(tag_id);
CREATE INDEX IF NOT EXISTS idx_command_executions_schedule_id ON command_executions(schedule_id);
CREATE INDEX IF NOT EXISTS idx_command_executions_status ON command_executions(status);
CREATE UNIQUE INDEX IF NOT EXISTS idx_command_executions_event_id
    ON command_executions(event_id) WHERE event_id IS NOT NULL;

PRAGMA foreign_key_check;

COMMIT;

PRAGMA foreign_keys = ON;
