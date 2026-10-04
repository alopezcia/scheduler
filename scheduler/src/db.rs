//! Acceso al SQLite compartido con el backend (que es quien aplica las migraciones).

use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

/// Una ocurrencia (evento) de un schedule SCADA sobre un tag de una conexión REST.
#[derive(Debug, FromRow)]
pub struct Task {
    pub event_id: String,
    pub start_date: String,
    pub schedule_id: String,
    pub schedule_name: String,
    pub target_value: String,
    pub requested_by: String,
    pub tag_id: String,
    pub tag_name: String,
    pub read_write: String,
    pub requires_sbo: bool,
    pub requires_ack: bool,
    pub address: String,
    pub connection_config: String,
    pub has_interlocks: bool,
}

/// Eventos de schedules habilitados, sin confirmación manual, sobre conexiones REST y
/// que todavía no tienen ninguna ejecución. El filtro de "vencido" se hace en Rust porque
/// `events.start_date` es el texto ISO-8601 que envió el frontend (formatos no uniformes).
const PENDING_TASKS: &str = "
    SELECT e.id AS event_id, e.start_date,
           s.id AS schedule_id, s.name AS schedule_name, s.target_value, s.created_by AS requested_by,
           t.id AS tag_id, t.name AS tag_name, t.read_write, t.requires_sbo, t.requires_ack, t.address,
           c.config AS connection_config,
           EXISTS (SELECT 1 FROM interlocks i WHERE i.tag_id = t.id) AS has_interlocks
    FROM events e
    JOIN schedules s ON s.id = e.schedule_id
    JOIN tags t ON t.id = s.tag_id
    JOIN connections c ON c.id = t.connection_id
    WHERE c.protocol = 'rest'
      AND s.enabled = 1
      AND s.requires_confirmation = 0
      AND NOT EXISTS (SELECT 1 FROM command_executions x WHERE x.event_id = e.id)
    ORDER BY e.start_date ASC";

pub async fn pending_tasks(pool: &SqlitePool) -> Result<Vec<Task>, sqlx::Error> {
    sqlx::query_as::<_, Task>(PENDING_TASKS).fetch_all(pool).await
}

/// Registra la ejecución del evento. Devuelve `None` si el evento ya tenía una (índice único
/// sobre `event_id`), p. ej. otra instancia de scheduler se adelantó: no hay que ejecutarlo.
pub async fn claim(
    pool: &SqlitePool,
    task: &Task,
    status: &str,
    error_message: Option<&str>,
) -> Result<Option<String>, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let result = sqlx::query(
        "INSERT INTO command_executions
            (id, event_id, schedule_id, tag_id, requested_value, requested_by, status, error_message)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT DO NOTHING",
    )
    .bind(&id)
    .bind(&task.event_id)
    .bind(&task.schedule_id)
    .bind(&task.tag_id)
    .bind(&task.target_value)
    .bind(&task.requested_by)
    .bind(status)
    .bind(error_message)
    .execute(pool)
    .await?;

    Ok((result.rows_affected() == 1).then_some(id))
}

pub struct Outcome<'a> {
    pub status: &'a str,
    pub sent: bool,
    pub acked: bool,
    pub response: Option<&'a str>,
    pub error_message: Option<&'a str>,
    pub latency_ms: Option<i64>,
}

pub async fn finish(pool: &SqlitePool, execution_id: &str, outcome: &Outcome<'_>) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE command_executions
         SET status = ?,
             sent_at = CASE WHEN ? THEN strftime('%Y-%m-%dT%H:%M:%fZ', 'now') END,
             ack_at = CASE WHEN ? THEN strftime('%Y-%m-%dT%H:%M:%fZ', 'now') END,
             response = ?, error_message = ?, latency_ms = ?
         WHERE id = ?",
    )
        .bind(outcome.status)
        .bind(outcome.sent)
        .bind(outcome.acked)
        .bind(outcome.response)
        .bind(outcome.error_message)
        .bind(outcome.latency_ms)
        .bind(execution_id)
        .execute(pool)
        .await?;
    Ok(())
}
