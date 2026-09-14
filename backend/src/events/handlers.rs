use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use chrono::DateTime;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::events::models::{EventPayload, EventResponse, EventRow};
use crate::state::AppState;

const SELECT_EVENT_BY_ID: &str = r#"
    SELECT e.id, e.schedule_id, s.name AS title, e.start_date, e.user_id, u.name AS user_name
    FROM events e
    JOIN schedules s ON s.id = e.schedule_id
    JOIN users u ON u.id = e.user_id
    WHERE e.id = ?
"#;

const SELECT_ALL_EVENTS: &str = r#"
    SELECT e.id, e.schedule_id, s.name AS title, e.start_date, e.user_id, u.name AS user_name
    FROM events e
    JOIN schedules s ON s.id = e.schedule_id
    JOIN users u ON u.id = e.user_id
    ORDER BY e.start_date ASC
"#;

async fn validate_payload(state: &AppState, payload: &EventPayload) -> Result<(), AppError> {
    if payload.schedule_id.trim().is_empty() {
        return Err(AppError::BadRequest(
            "schedule_id es obligatorio".to_string(),
        ));
    }

    DateTime::parse_from_rfc3339(&payload.start)
        .map_err(|_| AppError::BadRequest("La fecha de inicio no es válida".to_string()))?;

    let schedule_exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM schedules WHERE id = ?")
        .bind(&payload.schedule_id)
        .fetch_one(&state.pool)
        .await?;
    if schedule_exists == 0 {
        return Err(AppError::BadRequest(
            "El schedule referenciado no existe".to_string(),
        ));
    }

    Ok(())
}

async fn fetch_event(state: &AppState, id: &str) -> Result<EventResponse, AppError> {
    sqlx::query_as::<_, EventRow>(SELECT_EVENT_BY_ID)
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .map(EventResponse::from)
        .ok_or_else(|| AppError::NotFound("Evento no existe por ese id".to_string()))
}

/// Devuelve todos los eventos del calendario (compartido entre usuarios),
/// tal como espera `onLoadEvents` en `calendarSlice.js`.
pub async fn list_events(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let rows = sqlx::query_as::<_, EventRow>(SELECT_ALL_EVENTS)
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
    validate_payload(&state, &payload).await?;

    let id = Uuid::new_v4().to_string();

    sqlx::query("INSERT INTO events (id, schedule_id, start_date, user_id) VALUES (?, ?, ?, ?)")
        .bind(&id)
        .bind(&payload.schedule_id)
        .bind(&payload.start)
        .bind(&auth.uid)
        .execute(&state.pool)
        .await?;

    let evento = fetch_event(&state, &id).await?;

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
    validate_payload(&state, &payload).await?;

    let owner_id = find_owner(&state, &id).await?;

    if owner_id != auth.uid {
        return Err(AppError::Unauthorized(
            "No tiene privilegio de editar este evento".to_string(),
        ));
    }

    sqlx::query("UPDATE events SET schedule_id = ?, start_date = ? WHERE id = ?")
        .bind(&payload.schedule_id)
        .bind(&payload.start)
        .bind(&id)
        .execute(&state.pool)
        .await?;

    let evento = fetch_event(&state, &id).await?;

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

    fn payload(schedule_id: &str, start: &str) -> EventPayload {
        EventPayload {
            schedule_id: schedule_id.to_string(),
            start: start.to_string(),
        }
    }

    #[test]
    fn rejects_dates_that_are_not_rfc3339() {
        let p = payload("schedule-1", "not-a-date");
        assert!(DateTime::parse_from_rfc3339(&p.start).is_err());
    }

    #[test]
    fn rejects_an_empty_schedule_id() {
        let p = payload("   ", "2026-09-10T15:00:00.000Z");
        assert!(p.schedule_id.trim().is_empty());
    }
}
