-- Modelo de datos SCADA: jerarquía de planta, conexiones a PLC (OPC UA / MQTT / S7),
-- registro de tags (telemandos, consignas y medidas), programaciones, enclavamientos
-- y auditoría de ejecución de comandos.

CREATE TABLE IF NOT EXISTS sites (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE IF NOT EXISTS assets (
    id TEXT PRIMARY KEY,
    site_id TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    parent_asset_id TEXT REFERENCES assets(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_assets_site_id ON assets(site_id);
CREATE INDEX IF NOT EXISTS idx_assets_parent_asset_id ON assets(parent_asset_id);

-- Una conexión = un PLC o gateway accesible por un único protocolo. `config`
-- guarda en JSON los parámetros propios de ese protocolo, p. ej.:
--   opcua: { "endpoint_url", "security_policy", "credentials_ref" }
--   mqtt:  { "broker_url", "base_topic", "qos", "tls" }
--   s7:    { "ip", "rack", "slot" }
CREATE TABLE IF NOT EXISTS connections (
    id TEXT PRIMARY KEY,
    asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    protocol TEXT NOT NULL CHECK (protocol IN ('opcua', 'mqtt', 's7')),
    config TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'unknown' CHECK (status IN ('unknown', 'online', 'offline', 'error')),
    last_seen TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_connections_asset_id ON connections(asset_id);

-- Un tag es el punto SCADA lógico. `address` guarda en JSON la dirección física
-- dentro del protocolo de su `connection`, p. ej.:
--   opcua: { "node_id" }
--   mqtt:  { "topic", "json_pointer" }
--   s7:    { "db_number", "offset", "bit", "s7_type" }
CREATE TABLE IF NOT EXISTS tags (
    id TEXT PRIMARY KEY,
    connection_id TEXT NOT NULL REFERENCES connections(id) ON DELETE CASCADE,
    asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    tag_kind TEXT NOT NULL CHECK (tag_kind IN ('telemando', 'consigna', 'medida')),
    data_type TEXT NOT NULL CHECK (data_type IN ('bool', 'int', 'float', 'enum')),
    unit TEXT,
    address TEXT NOT NULL,
    read_write TEXT NOT NULL DEFAULT 'write' CHECK (read_write IN ('read', 'write', 'read_write')),
    min_value REAL,
    max_value REAL,
    allowed_values TEXT,
    requires_sbo INTEGER NOT NULL DEFAULT 0,
    requires_ack INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (connection_id, name)
);

CREATE INDEX IF NOT EXISTS idx_tags_connection_id ON tags(connection_id);
CREATE INDEX IF NOT EXISTS idx_tags_asset_id ON tags(asset_id);
CREATE INDEX IF NOT EXISTS idx_tags_tag_kind ON tags(tag_kind);

-- Generaliza el calendario existente: una programación apunta a un tag y a un
-- valor objetivo (JSON, ya que depende del data_type del tag), y se dispara
-- una vez, por cron, o ligada a un `events` del calendario ya existente.
CREATE TABLE IF NOT EXISTS schedules (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    target_value TEXT NOT NULL,
    trigger_type TEXT NOT NULL CHECK (trigger_type IN ('once', 'cron', 'calendar_event')),
    cron_expr TEXT,
    event_id TEXT REFERENCES events(id) ON DELETE CASCADE,
    start_date TEXT,
    end_date TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    requires_confirmation INTEGER NOT NULL DEFAULT 0,
    created_by TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_schedules_tag_id ON schedules(tag_id);
CREATE INDEX IF NOT EXISTS idx_schedules_enabled ON schedules(enabled);
CREATE INDEX IF NOT EXISTS idx_schedules_event_id ON schedules(event_id);

-- Enclavamiento: antes de ejecutar un comando sobre `tag_id`, se evalúa el
-- valor de `condition_tag_id` con `operator`; si se cumple, `action` bloquea
-- el envío o solo lo advierte.
CREATE TABLE IF NOT EXISTS interlocks (
    id TEXT PRIMARY KEY,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    condition_tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    operator TEXT NOT NULL CHECK (operator IN ('eq', 'ne', 'gt', 'gte', 'lt', 'lte')),
    condition_value TEXT NOT NULL,
    action TEXT NOT NULL DEFAULT 'block' CHECK (action IN ('block', 'warn')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_interlocks_tag_id ON interlocks(tag_id);

-- Auditoría: un registro por cada intento de escritura a un tag, ya sea
-- disparado por un `schedule` o manual (schedule_id nulo).
CREATE TABLE IF NOT EXISTS command_executions (
    id TEXT PRIMARY KEY,
    schedule_id TEXT REFERENCES schedules(id) ON DELETE SET NULL,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    requested_value TEXT NOT NULL,
    requested_by TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'sent', 'ack', 'failed', 'timeout')),
    sent_at TEXT,
    ack_at TEXT,
    response TEXT,
    error_message TEXT,
    latency_ms INTEGER,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_command_executions_tag_id ON command_executions(tag_id);
CREATE INDEX IF NOT EXISTS idx_command_executions_schedule_id ON command_executions(schedule_id);
CREATE INDEX IF NOT EXISTS idx_command_executions_status ON command_executions(status);
