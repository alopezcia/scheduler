-- La app `scheduler` ejecuta una vez cada evento del calendario (una ocurrencia de un
-- schedule). Para que no lo repita tras un reinicio, cada ejecución apunta al evento que
-- la originó y el índice único garantiza como mucho una ejecución por evento.
-- ON DELETE SET NULL: borrar el evento no borra la auditoría del comando.
ALTER TABLE command_executions ADD COLUMN event_id TEXT REFERENCES events(id) ON DELETE SET NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_command_executions_event_id
    ON command_executions(event_id) WHERE event_id IS NOT NULL;
