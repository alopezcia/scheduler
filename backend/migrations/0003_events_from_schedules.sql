-- Invierte la relación entre schedules y events: un evento de calendario deja
-- de ser una entidad libre (o el origen opcional de un schedule) para
-- convertirse siempre en una ocurrencia generada por un schedule (once o
-- cron). Se limpia el contenido previo de `events` porque su forma cambia por
-- completo (pierde title/notes/end_date propios y gana schedule_id).

DELETE FROM events;

ALTER TABLE events ADD COLUMN schedule_id TEXT REFERENCES schedules(id) ON DELETE CASCADE;
ALTER TABLE events DROP COLUMN title;
ALTER TABLE events DROP COLUMN notes;
ALTER TABLE events DROP COLUMN end_date;

CREATE INDEX IF NOT EXISTS idx_events_schedule_id ON events(schedule_id);

-- El schedule ya no referencia un event (era la relación inversa); ahora es
-- el event el que referencia al schedule que lo generó.
DROP INDEX IF EXISTS idx_schedules_event_id;
ALTER TABLE schedules DROP COLUMN event_id;

-- Nota: SQLite no permite modificar un CHECK existente sin recrear la tabla,
-- así que `schedules.trigger_type` sigue aceptando 'calendar_event' a nivel
-- de columna; TriggerType en el backend ya no tiene esa variante, así que
-- nunca se vuelve a escribir.
