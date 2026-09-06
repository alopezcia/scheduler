use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use chrono::DateTime;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::events::models::{EventPayload, EventResponse, EventRow, EventUser};
use crate::state::AppState;

fn validate_payload(payload: &EventPayload) -> Result<(), AppError> {
    if payload.title.trim().is_empty() {
        return Err(AppError::BadRequest("El título es obligatorio".to_string()));
    }

    let start = DateTime::parse_from_rfc3339(&payload.start)
        .map_err(|_| AppError::BadRequest("La fecha de inicio no es válida".to_string()))?;
    let end = DateTime::parse_from_rfc3339(&payload.end)
        .map_err(|_| AppError::BadRequest("La fecha de fin no es válida".to_string()))?;

    if end <= start {
        return Err(AppError::BadRequest(
            "La fecha de fin debe ser mayor a la fecha de inicio".to_string(),
        ));
    }

    Ok(())
}

/// Devuelve todos los eventos del calendario (compartido entre usuarios),
/// tal como espera `onLoadEvents` en `calendarSlice.js`.
pub async fn list_events(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let rows = sqlx::query_as::<_, EventRow>(
        r#"
        SELECT e.id, e.title, e.notes, e.start_date, e.end_date, e.user_id, u.name AS user_name
        FROM events e
        JOIN users u ON u.id = e.user_id
        ORDER BY e.start_date ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    let eventos: Vec<EventResponse> = rows.into_iter().map(EventResponse::from).collect();

    Ok(Json(json!({ "ok": true, "eventos": eventos })))
}

pub async fn create_event(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<EventPayload>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    validate_payload(&payload)?;

    let id = Uuid::new_v4().to_string();
    let notes = payload.notes.unwrap_or_default();

    sqlx::query(
        "INSERT INTO events (id, title, notes, start_date, end_date, user_id) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&payload.title)
    .bind(&notes)
    .bind(&payload.start)
    .bind(&payload.end)
    .bind(&auth.uid)
    .execute(&state.pool)
    .await?;

    let evento = EventResponse {
        id,
        title: payload.title,
        notes,
        start: payload.start,
        end: payload.end,
        user: EventUser {
            uid: auth.uid,
            name: auth.name,
        },
    };

    Ok((StatusCode::CREATED, Json(json!({ "ok": true, "evento": evento }))))
}

async fn find_owner(state: &AppState, id: &str) -> Result<String, AppError> {
    sqlx::query_scalar::<_, String>("SELECT user_id FROM events WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Evento no existe por ese id".to_string()))
}

pub async fn update_event(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<EventPayload>,
) -> Result<Json<Value>, AppError> {
    validate_payload(&payload)?;

    let owner_id = find_owner(&state, &id).await?;

    if owner_id != auth.uid {
        return Err(AppError::Unauthorized(
            "No tiene privilegio de editar este evento".to_string(),
        ));
    }

    let notes = payload.notes.unwrap_or_default();

    sqlx::query("UPDATE events SET title = ?, notes = ?, start_date = ?, end_date = ? WHERE id = ?")
        .bind(&payload.title)
        .bind(&notes)
        .bind(&payload.start)
        .bind(&payload.end)
        .bind(&id)
        .execute(&state.pool)
        .await?;

    let evento = EventResponse {
        id,
        title: payload.title,
        notes,
        start: payload.start,
        end: payload.end,
        user: EventUser {
            uid: auth.uid,
            name: auth.name,
        },
    };

    Ok(Json(json!({ "ok": true, "evento": evento })))
}

pub async fn delete_event(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let owner_id = find_owner(&state, &id).await?;

    if owner_id != auth.uid {
        return Err(AppError::Unauthorized(
            "No tiene privilegio de eliminar este evento".to_string(),
        ));
    }

    sqlx::query("DELETE FROM events WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await?;

    Ok(Json(json!({ "ok": true })))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(title: &str, start: &str, end: &str) -> EventPayload {
        EventPayload {
            title: title.to_string(),
            notes: None,
            start: start.to_string(),
            end: end.to_string(),
        }
    }

    #[test]
    fn accepts_a_well_formed_payload() {
        let p = payload("Cumpleaños", "2026-09-10T15:00:00.000Z", "2026-09-10T17:00:00.000Z");
        assert!(validate_payload(&p).is_ok());
    }

    #[test]
    fn rejects_an_empty_title() {
        let p = payload("   ", "2026-09-10T15:00:00.000Z", "2026-09-10T17:00:00.000Z");
        assert!(validate_payload(&p).is_err());
    }

    #[test]
    fn rejects_dates_that_are_not_rfc3339() {
        let p = payload("Cumpleaños", "not-a-date", "2026-09-10T17:00:00.000Z");
        assert!(validate_payload(&p).is_err());
    }

    #[test]
    fn rejects_an_end_that_is_not_after_start() {
        let p = payload("Cumpleaños", "2026-09-10T17:00:00.000Z", "2026-09-10T15:00:00.000Z");
        assert!(validate_payload(&p).is_err());
    }
}
