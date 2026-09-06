use serde::{Deserialize, Serialize};

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
    pub title: String,
    pub notes: String,
    pub start: String,
    pub end: String,
    pub user: EventUser,
}

#[derive(Debug, sqlx::FromRow)]
pub struct EventRow {
    pub id: String,
    pub title: String,
    pub notes: String,
    pub start_date: String,
    pub end_date: String,
    pub user_id: String,
    pub user_name: String,
}

impl From<EventRow> for EventResponse {
    fn from(row: EventRow) -> Self {
        EventResponse {
            id: row.id,
            title: row.title,
            notes: row.notes,
            start: row.start_date,
            end: row.end_date,
            user: EventUser {
                uid: row.user_id,
                name: row.user_name,
            },
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct EventPayload {
    pub title: String,
    #[serde(default)]
    pub notes: Option<String>,
    pub start: String,
    pub end: String,
}
