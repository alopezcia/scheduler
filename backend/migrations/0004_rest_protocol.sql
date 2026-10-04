-- no-transaction
-- Añade 'rest' a los protocolos permitidos de `connections`. A diferencia de
-- opcua/mqtt/s7 (que hablan con el PLC), 'rest' habla con un sistema SCADA
-- (Ignition, WinCC, AVEVA, ...) a través de su API REST; el SCADA es quien
-- habla con los PLCs.
--   rest: config  { "scada_vendor", "base_url", "auth_type", "credentials_ref", "timeout_ms" }
--         address { "method", "path", "json_pointer" }  (los tags leen con GET `path`
--         y escriben con `method` sobre `path`; `json_pointer` localiza el valor en el JSON)
--
-- SQLite no permite modificar un CHECK sin recrear la tabla. Como `tags` referencia
-- `connections` con ON DELETE CASCADE, hay que apagar las foreign keys mientras se
-- reconstruye (si no, el DROP TABLE borraría en cascada todos los tags). Por eso esta
-- migración va sin transacción implícita y gestiona la suya propia: PRAGMA foreign_keys
-- no tiene efecto dentro de una transacción.

PRAGMA foreign_keys = OFF;

BEGIN;

CREATE TABLE connections_new (
    id TEXT PRIMARY KEY,
    asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    protocol TEXT NOT NULL CHECK (protocol IN ('opcua', 'mqtt', 's7', 'rest')),
    config TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'unknown' CHECK (status IN ('unknown', 'online', 'offline', 'error')),
    last_seen TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

INSERT INTO connections_new (id, asset_id, name, protocol, config, status, last_seen, created_at)
SELECT id, asset_id, name, protocol, config, status, last_seen, created_at FROM connections;

DROP TABLE connections;
ALTER TABLE connections_new RENAME TO connections;

CREATE INDEX IF NOT EXISTS idx_connections_asset_id ON connections(asset_id);

PRAGMA foreign_key_check;

COMMIT;

PRAGMA foreign_keys = ON;
