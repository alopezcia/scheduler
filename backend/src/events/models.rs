use chrono::{DateTime, Duration};
use serde::{Deserialize, Serialize};

/// Duración fija que se usa únicamente para poder dibujar el evento como un
/// bloque en `react-big-calendar` (que exige start+end); no se persiste ni
/// es configurable, ya que un event es una ocurrencia puntual de un schedule.
const DISPLAY_DURATION_MINUTES: i64 = 30;

/// El frontend representa a un usuario autenticado como `{ uid, name }`
/// (ver `authSlice.js`), así que el evento embebe el mismo shape para que
/// `eventStyleGetter` en `CalendarPage.jsx` pueda comparar `user.uid`.
#[derive(Debug, Serialize)]
pub struct EventUser {
    pub uid: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct EventResponse {
    pub id: String,
    pub schedule_id: String,
    pub title: String,
    pub start: String,
    pub end: String,
    pub user: EventUser,
}

#[derive(Debug, sqlx::FromRow)]
pub struct EventRow {
    pub id: String,
    pub schedule_id: String,
    /// Viene de `schedules.name` vía JOIN: el evento no guarda su propio
    /// título, siempre refleja el nombre del schedule que lo generó.
    pub title: String,
    pub start_date: String,
    pub user_id: String,
    pub user_name: String,
}

impl From<EventRow> for EventResponse {
    fn from(row: EventRow) -> Self {
        let end = DateTime::parse_from_rfc3339(&row.start_date)
            .map(|start| (start + Duration::minutes(DISPLAY_DURATION_MINUTES)).to_rfc3339())
            .unwrap_or_else(|_| row.start_date.clone());

        EventResponse {
            id: row.id,
            schedule_id: row.schedule_id,
            title: row.title,
            start: row.start_date,
            end,
            user: EventUser {
                uid: row.user_id,
                name: row.user_name,
            },
        }
    }
}

/// Un evento es siempre la ocurrencia de un `schedule` en una fecha dada
/// (generada automáticamente al guardar el schedule, o añadida a mano desde
/// el calendario eligiendo el schedule y la fecha).
#[derive(Debug, Deserialize)]
pub struct EventPayload {
    pub schedule_id: String,
    pub start: String,
}
