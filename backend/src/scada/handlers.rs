use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::types::Json as SqlxJson;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::scada::models::{
    AssetPayload, AssetRow, ConnectionPayload, ConnectionResponse, ConnectionRow, DataType,
    Protocol, SchedulePayload, ScheduleResponse, ScheduleRow, SitePayload, SiteRow, TagPayload,
    TagResponse, TagRow, TriggerType,
};
use crate::state::AppState;

// ================= Sites =================

fn validate_site_payload(payload: &SitePayload) -> Result<(), AppError> {
    if payload.name.trim().is_empty() {
        return Err(AppError::BadRequest(
            "El nombre del site es obligatorio".to_string(),
        ));
    }
    Ok(())
}

async fn fetch_site(state: &AppState, id: &str) -> Result<SiteRow, AppError> {
    sqlx::query_as::<_, SiteRow>("SELECT id, name, created_at FROM sites WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("El site no existe".to_string()))
}

pub async fn list_sites(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let rows = sqlx::query_as::<_, SiteRow>(
        "SELECT id, name, created_at FROM sites ORDER BY created_at ASC",
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(json!({ "ok": true, "sites": rows })))
}

pub async fn create_site(
    _auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<SitePayload>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    validate_site_payload(&payload)?;

    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO sites (id, name) VALUES (?, ?)")
        .bind(&id)
        .bind(payload.name.trim())
        .execute(&state.pool)
        .await?;

    let site = fetch_site(&state, &id).await?;
    Ok((StatusCode::CREATED, Json(json!({ "ok": true, "site": site }))))
}

pub async fn update_site(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<SitePayload>,
) -> Result<Json<Value>, AppError> {
    validate_site_payload(&payload)?;

    let result = sqlx::query("UPDATE sites SET name = ? WHERE id = ?")
        .bind(payload.name.trim())
        .bind(&id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("El site no existe".to_string()));
    }

    let site = fetch_site(&state, &id).await?;
    Ok(Json(json!({ "ok": true, "site": site })))
}

pub async fn delete_site(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let result = sqlx::query("DELETE FROM sites WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("El site no existe".to_string()));
    }

    Ok(Json(json!({ "ok": true })))
}

// ================= Assets =================

#[derive(Debug, Deserialize)]
pub struct AssetFilter {
    pub site_id: Option<String>,
    pub parent_asset_id: Option<String>,
}

fn validate_asset_payload(payload: &AssetPayload) -> Result<(), AppError> {
    if payload.name.trim().is_empty() {
        return Err(AppError::BadRequest(
            "El nombre del asset es obligatorio".to_string(),
        ));
    }
    if payload.kind.trim().is_empty() {
        return Err(AppError::BadRequest(
            "El kind del asset es obligatorio".to_string(),
        ));
    }
    Ok(())
}

/// Verifica que `site_id` exista y que, si hay `parent_asset_id`, el padre
/// exista, no sea el propio asset que se está editando, y pertenezca al mismo site.
async fn ensure_asset_hierarchy(
    state: &AppState,
    payload: &AssetPayload,
    self_id: Option<&str>,
) -> Result<(), AppError> {
    let site_exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sites WHERE id = ?")
        .bind(&payload.site_id)
        .fetch_one(&state.pool)
        .await?;
    if site_exists == 0 {
        return Err(AppError::BadRequest(
            "El site referenciado no existe".to_string(),
        ));
    }

    if let Some(parent_id) = &payload.parent_asset_id {
        if Some(parent_id.as_str()) == self_id {
            return Err(AppError::BadRequest(
                "Un asset no puede ser su propio padre".to_string(),
            ));
        }

        let parent_site: Option<String> =
            sqlx::query_scalar("SELECT site_id FROM assets WHERE id = ?")
                .bind(parent_id)
                .fetch_optional(&state.pool)
                .await?;

        match parent_site {
            None => {
                return Err(AppError::BadRequest(
                    "El asset padre referenciado no existe".to_string(),
                ));
            }
            Some(site_id) if site_id != payload.site_id => {
                return Err(AppError::BadRequest(
                    "El asset padre debe pertenecer al mismo site".to_string(),
                ));
            }
            _ => {}
        }
    }

    Ok(())
}

async fn fetch_asset(state: &AppState, id: &str) -> Result<AssetRow, AppError> {
    sqlx::query_as::<_, AssetRow>(
        "SELECT id, site_id, parent_asset_id, name, kind, created_at FROM assets WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("El asset no existe".to_string()))
}

pub async fn list_assets(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(filter): Query<AssetFilter>,
) -> Result<Json<Value>, AppError> {
    let rows = match (&filter.site_id, &filter.parent_asset_id) {
        (Some(site_id), Some(parent_id)) => sqlx::query_as::<_, AssetRow>(
            "SELECT id, site_id, parent_asset_id, name, kind, created_at FROM assets \
             WHERE site_id = ? AND parent_asset_id = ? ORDER BY created_at ASC",
        )
        .bind(site_id)
        .bind(parent_id)
        .fetch_all(&state.pool)
        .await?,
        (Some(site_id), None) => sqlx::query_as::<_, AssetRow>(
            "SELECT id, site_id, parent_asset_id, name, kind, created_at FROM assets \
             WHERE site_id = ? ORDER BY created_at ASC",
        )
        .bind(site_id)
        .fetch_all(&state.pool)
        .await?,
        (None, Some(parent_id)) => sqlx::query_as::<_, AssetRow>(
            "SELECT id, site_id, parent_asset_id, name, kind, created_at FROM assets \
             WHERE parent_asset_id = ? ORDER BY created_at ASC",
        )
        .bind(parent_id)
        .fetch_all(&state.pool)
        .await?,
        (None, None) => sqlx::query_as::<_, AssetRow>(
            "SELECT id, site_id, parent_asset_id, name, kind, created_at FROM assets \
             ORDER BY created_at ASC",
        )
        .fetch_all(&state.pool)
        .await?,
    };

    Ok(Json(json!({ "ok": true, "assets": rows })))
}

pub async fn create_asset(
    _auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<AssetPayload>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    validate_asset_payload(&payload)?;
    ensure_asset_hierarchy(&state, &payload, None).await?;

    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO assets (id, site_id, parent_asset_id, name, kind) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&payload.site_id)
    .bind(&payload.parent_asset_id)
    .bind(payload.name.trim())
    .bind(payload.kind.trim())
    .execute(&state.pool)
    .await?;

    let asset = fetch_asset(&state, &id).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "ok": true, "asset": asset })),
    ))
}

pub async fn update_asset(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<AssetPayload>,
) -> Result<Json<Value>, AppError> {
    validate_asset_payload(&payload)?;
    ensure_asset_hierarchy(&state, &payload, Some(id.as_str())).await?;

    let result = sqlx::query(
        "UPDATE assets SET site_id = ?, parent_asset_id = ?, name = ?, kind = ? WHERE id = ?",
    )
    .bind(&payload.site_id)
    .bind(&payload.parent_asset_id)
    .bind(payload.name.trim())
    .bind(payload.kind.trim())
    .bind(&id)
    .execute(&state.pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("El asset no existe".to_string()));
    }

    let asset = fetch_asset(&state, &id).await?;
    Ok(Json(json!({ "ok": true, "asset": asset })))
}

pub async fn delete_asset(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let result = sqlx::query("DELETE FROM assets WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("El asset no existe".to_string()));
    }

    Ok(Json(json!({ "ok": true })))
}

// ================= Connections =================

#[derive(Debug, Deserialize)]
pub struct ConnectionFilter {
    pub asset_id: Option<String>,
}

fn validate_connection_config(protocol: Protocol, config: &Value) -> Result<(), AppError> {
    let obj = config.as_object().ok_or_else(|| {
        AppError::BadRequest("config debe ser un objeto JSON".to_string())
    })?;

    let required: &[&str] = match protocol {
        Protocol::Opcua => &["endpoint_url", "security_policy", "credentials_ref"],
        Protocol::Mqtt => &["broker_url", "base_topic", "qos", "tls"],
        Protocol::S7 => &["ip", "rack", "slot"],
    };

    for key in required {
        if !obj.contains_key(*key) {
            return Err(AppError::BadRequest(format!(
                "config.{key} es obligatorio para el protocolo {protocol:?}"
            )));
        }
    }

    Ok(())
}

fn validate_connection_payload(payload: &ConnectionPayload) -> Result<(), AppError> {
    if payload.name.trim().is_empty() {
        return Err(AppError::BadRequest(
            "El nombre de la conexión es obligatorio".to_string(),
        ));
    }
    validate_connection_config(payload.protocol, &payload.config)
}

async fn ensure_asset_exists(state: &AppState, id: &str) -> Result<(), AppError> {
    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM assets WHERE id = ?")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;
    if exists == 0 {
        return Err(AppError::BadRequest(
            "El asset referenciado no existe".to_string(),
        ));
    }
    Ok(())
}

async fn fetch_connection(state: &AppState, id: &str) -> Result<ConnectionResponse, AppError> {
    sqlx::query_as::<_, ConnectionRow>(
        "SELECT id, asset_id, name, protocol, config, status, last_seen, created_at \
         FROM connections WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .map(ConnectionResponse::from)
    .ok_or_else(|| AppError::NotFound("La conexión no existe".to_string()))
}

pub async fn list_connections(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(filter): Query<ConnectionFilter>,
) -> Result<Json<Value>, AppError> {
    let rows = match &filter.asset_id {
        Some(asset_id) => {
            sqlx::query_as::<_, ConnectionRow>(
                "SELECT id, asset_id, name, protocol, config, status, last_seen, created_at \
                 FROM connections WHERE asset_id = ? ORDER BY created_at ASC",
            )
            .bind(asset_id)
            .fetch_all(&state.pool)
            .await?
        }
        None => {
            sqlx::query_as::<_, ConnectionRow>(
                "SELECT id, asset_id, name, protocol, config, status, last_seen, created_at \
                 FROM connections ORDER BY created_at ASC",
            )
            .fetch_all(&state.pool)
            .await?
        }
    };

    let connections: Vec<ConnectionResponse> =
        rows.into_iter().map(ConnectionResponse::from).collect();

    Ok(Json(json!({ "ok": true, "connections": connections })))
}

pub async fn create_connection(
    _auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<ConnectionPayload>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    validate_connection_payload(&payload)?;
    ensure_asset_exists(&state, &payload.asset_id).await?;

    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO connections (id, asset_id, name, protocol, config) VALUES (?, ?, ?, ?, ?)")
        .bind(&id)
        .bind(&payload.asset_id)
        .bind(payload.name.trim())
        .bind(payload.protocol)
        .bind(SqlxJson(&payload.config))
        .execute(&state.pool)
        .await?;

    let connection = fetch_connection(&state, &id).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "ok": true, "connection": connection })),
    ))
}

pub async fn update_connection(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<ConnectionPayload>,
) -> Result<Json<Value>, AppError> {
    validate_connection_payload(&payload)?;
    ensure_asset_exists(&state, &payload.asset_id).await?;

    let result = sqlx::query(
        "UPDATE connections SET asset_id = ?, name = ?, protocol = ?, config = ? WHERE id = ?",
    )
    .bind(&payload.asset_id)
    .bind(payload.name.trim())
    .bind(payload.protocol)
    .bind(SqlxJson(&payload.config))
    .bind(&id)
    .execute(&state.pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("La conexión no existe".to_string()));
    }

    let connection = fetch_connection(&state, &id).await?;
    Ok(Json(json!({ "ok": true, "connection": connection })))
}

pub async fn delete_connection(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let result = sqlx::query("DELETE FROM connections WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("La conexión no existe".to_string()));
    }

    Ok(Json(json!({ "ok": true })))
}

// ================= Tags =================

#[derive(Debug, Deserialize)]
pub struct TagFilter {
    pub connection_id: Option<String>,
    pub asset_id: Option<String>,
}

fn validate_tag_address(protocol: Protocol, address: &Value) -> Result<(), AppError> {
    let obj = address
        .as_object()
        .ok_or_else(|| AppError::BadRequest("address debe ser un objeto JSON".to_string()))?;

    let required: &[&str] = match protocol {
        Protocol::Opcua => &["node_id"],
        Protocol::Mqtt => &["topic", "json_pointer"],
        Protocol::S7 => &["db_number", "offset", "bit", "s7_type"],
    };

    for key in required {
        if !obj.contains_key(*key) {
            return Err(AppError::BadRequest(format!(
                "address.{key} es obligatorio para el protocolo {protocol:?}"
            )));
        }
    }

    Ok(())
}

fn validate_tag_payload(payload: &TagPayload) -> Result<(), AppError> {
    if payload.name.trim().is_empty() {
        return Err(AppError::BadRequest(
            "El nombre del tag es obligatorio".to_string(),
        ));
    }

    if let (Some(min), Some(max)) = (payload.min_value, payload.max_value)
        && min > max
    {
        return Err(AppError::BadRequest(
            "min_value no puede ser mayor a max_value".to_string(),
        ));
    }

    if payload.data_type == DataType::Enum && payload.allowed_values.is_none() {
        return Err(AppError::BadRequest(
            "Los tags de tipo enum requieren allowed_values".to_string(),
        ));
    }

    Ok(())
}

/// Verifica que la `connection_id` exista y que su `asset_id` coincida con el
/// del payload; devuelve el protocolo de la conexión para validar `address`.
async fn ensure_tag_connection_asset(
    state: &AppState,
    connection_id: &str,
    asset_id: &str,
) -> Result<Protocol, AppError> {
    let row: Option<(String, Protocol)> =
        sqlx::query_as("SELECT asset_id, protocol FROM connections WHERE id = ?")
            .bind(connection_id)
            .fetch_optional(&state.pool)
            .await?;

    let (connection_asset_id, protocol) = row.ok_or_else(|| {
        AppError::BadRequest("La conexión referenciada no existe".to_string())
    })?;

    if connection_asset_id != asset_id {
        return Err(AppError::BadRequest(
            "El tag debe pertenecer al mismo asset que su conexión".to_string(),
        ));
    }

    Ok(protocol)
}

async fn ensure_tag_name_available(
    state: &AppState,
    connection_id: &str,
    name: &str,
    self_id: Option<&str>,
) -> Result<(), AppError> {
    let duplicate: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tags WHERE connection_id = ? AND name = ? AND id != ?",
    )
    .bind(connection_id)
    .bind(name)
    .bind(self_id.unwrap_or(""))
    .fetch_one(&state.pool)
    .await?;

    if duplicate > 0 {
        return Err(AppError::BadRequest(
            "Ya existe un tag con ese nombre en la conexión".to_string(),
        ));
    }

    Ok(())
}

async fn fetch_tag(state: &AppState, id: &str) -> Result<TagResponse, AppError> {
    sqlx::query_as::<_, TagRow>(
        "SELECT id, connection_id, asset_id, name, description, tag_kind, data_type, unit, \
         address, read_write, min_value, max_value, allowed_values, requires_sbo, requires_ack, \
         created_at FROM tags WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .map(TagResponse::from)
    .ok_or_else(|| AppError::NotFound("El tag no existe".to_string()))
}

pub async fn list_tags(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(filter): Query<TagFilter>,
) -> Result<Json<Value>, AppError> {
    let rows = match (&filter.connection_id, &filter.asset_id) {
        (Some(connection_id), Some(asset_id)) => sqlx::query_as::<_, TagRow>(
            "SELECT id, connection_id, asset_id, name, description, tag_kind, data_type, unit, \
             address, read_write, min_value, max_value, allowed_values, requires_sbo, requires_ack, \
             created_at FROM tags WHERE connection_id = ? AND asset_id = ? ORDER BY created_at ASC",
        )
        .bind(connection_id)
        .bind(asset_id)
        .fetch_all(&state.pool)
        .await?,
        (Some(connection_id), None) => sqlx::query_as::<_, TagRow>(
            "SELECT id, connection_id, asset_id, name, description, tag_kind, data_type, unit, \
             address, read_write, min_value, max_value, allowed_values, requires_sbo, requires_ack, \
             created_at FROM tags WHERE connection_id = ? ORDER BY created_at ASC",
        )
        .bind(connection_id)
        .fetch_all(&state.pool)
        .await?,
        (None, Some(asset_id)) => sqlx::query_as::<_, TagRow>(
            "SELECT id, connection_id, asset_id, name, description, tag_kind, data_type, unit, \
             address, read_write, min_value, max_value, allowed_values, requires_sbo, requires_ack, \
             created_at FROM tags WHERE asset_id = ? ORDER BY created_at ASC",
        )
        .bind(asset_id)
        .fetch_all(&state.pool)
        .await?,
        (None, None) => sqlx::query_as::<_, TagRow>(
            "SELECT id, connection_id, asset_id, name, description, tag_kind, data_type, unit, \
             address, read_write, min_value, max_value, allowed_values, requires_sbo, requires_ack, \
             created_at FROM tags ORDER BY created_at ASC",
        )
        .fetch_all(&state.pool)
        .await?,
    };

    let tags: Vec<TagResponse> = rows.into_iter().map(TagResponse::from).collect();

    Ok(Json(json!({ "ok": true, "tags": tags })))
}

pub async fn create_tag(
    _auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<TagPayload>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    validate_tag_payload(&payload)?;
    let protocol =
        ensure_tag_connection_asset(&state, &payload.connection_id, &payload.asset_id).await?;
    validate_tag_address(protocol, &payload.address)?;
    ensure_tag_name_available(&state, &payload.connection_id, payload.name.trim(), None).await?;

    let id = Uuid::new_v4().to_string();
    sqlx::query(
        r#"INSERT INTO tags (
            id, connection_id, asset_id, name, description, tag_kind, data_type, unit,
            address, read_write, min_value, max_value, allowed_values, requires_sbo, requires_ack
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(&payload.connection_id)
    .bind(&payload.asset_id)
    .bind(payload.name.trim())
    .bind(&payload.description)
    .bind(payload.tag_kind)
    .bind(payload.data_type)
    .bind(&payload.unit)
    .bind(SqlxJson(&payload.address))
    .bind(payload.read_write)
    .bind(payload.min_value)
    .bind(payload.max_value)
    .bind(payload.allowed_values.as_ref().map(SqlxJson))
    .bind(payload.requires_sbo)
    .bind(payload.requires_ack)
    .execute(&state.pool)
    .await?;

    let tag = fetch_tag(&state, &id).await?;
    Ok((StatusCode::CREATED, Json(json!({ "ok": true, "tag": tag }))))
}

pub async fn update_tag(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<TagPayload>,
) -> Result<Json<Value>, AppError> {
    validate_tag_payload(&payload)?;
    let protocol =
        ensure_tag_connection_asset(&state, &payload.connection_id, &payload.asset_id).await?;
    validate_tag_address(protocol, &payload.address)?;
    ensure_tag_name_available(
        &state,
        &payload.connection_id,
        payload.name.trim(),
        Some(id.as_str()),
    )
    .await?;

    let result = sqlx::query(
        r#"UPDATE tags SET
            connection_id = ?, asset_id = ?, name = ?, description = ?, tag_kind = ?,
            data_type = ?, unit = ?, address = ?, read_write = ?, min_value = ?, max_value = ?,
            allowed_values = ?, requires_sbo = ?, requires_ack = ?
        WHERE id = ?"#,
    )
    .bind(&payload.connection_id)
    .bind(&payload.asset_id)
    .bind(payload.name.trim())
    .bind(&payload.description)
    .bind(payload.tag_kind)
    .bind(payload.data_type)
    .bind(&payload.unit)
    .bind(SqlxJson(&payload.address))
    .bind(payload.read_write)
    .bind(payload.min_value)
    .bind(payload.max_value)
    .bind(payload.allowed_values.as_ref().map(SqlxJson))
    .bind(payload.requires_sbo)
    .bind(payload.requires_ack)
    .bind(&id)
    .execute(&state.pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("El tag no existe".to_string()));
    }

    let tag = fetch_tag(&state, &id).await?;
    Ok(Json(json!({ "ok": true, "tag": tag })))
}

pub async fn delete_tag(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let result = sqlx::query("DELETE FROM tags WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("El tag no existe".to_string()));
    }

    Ok(Json(json!({ "ok": true })))
}

// ================= Schedules =================

#[derive(Debug, Deserialize)]
pub struct ScheduleFilter {
    pub tag_id: Option<String>,
    pub enabled: Option<bool>,
}

fn validate_schedule_payload(payload: &SchedulePayload) -> Result<(), AppError> {
    if payload.name.trim().is_empty() {
        return Err(AppError::BadRequest(
            "El nombre de la programación es obligatorio".to_string(),
        ));
    }

    match payload.trigger_type {
        TriggerType::Once => {
            if payload.start_date.is_none() {
                return Err(AppError::BadRequest(
                    "Una programación 'once' requiere start_date".to_string(),
                ));
            }
        }
        TriggerType::Cron => {
            if payload
                .cron_expr
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
            {
                return Err(AppError::BadRequest(
                    "Una programación 'cron' requiere cron_expr".to_string(),
                ));
            }
        }
    }

    Ok(())
}

fn validate_target_value(
    data_type: DataType,
    allowed_values: Option<&Value>,
    target_value: &Value,
) -> Result<(), AppError> {
    let valid = match data_type {
        DataType::Bool => target_value.is_boolean(),
        DataType::Int => target_value.is_i64() || target_value.is_u64(),
        DataType::Float => target_value.is_number(),
        DataType::Enum => allowed_values
            .and_then(Value::as_array)
            .map(|values| values.contains(target_value))
            .unwrap_or(true),
    };

    if !valid {
        return Err(AppError::BadRequest(
            "target_value no es compatible con el data_type del tag".to_string(),
        ));
    }

    Ok(())
}

async fn fetch_tag_constraints(
    state: &AppState,
    tag_id: &str,
) -> Result<(DataType, Option<Value>), AppError> {
    let row: Option<(DataType, Option<SqlxJson<Value>>)> =
        sqlx::query_as("SELECT data_type, allowed_values FROM tags WHERE id = ?")
            .bind(tag_id)
            .fetch_optional(&state.pool)
            .await?;

    let (data_type, allowed_values) =
        row.ok_or_else(|| AppError::BadRequest("El tag referenciado no existe".to_string()))?;

    Ok((data_type, allowed_values.map(|v| v.0)))
}

async fn fetch_schedule(state: &AppState, id: &str) -> Result<ScheduleResponse, AppError> {
    sqlx::query_as::<_, ScheduleRow>(
        "SELECT id, name, tag_id, target_value, trigger_type, cron_expr, start_date, \
         end_date, enabled, requires_confirmation, created_by, created_at \
         FROM schedules WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .map(ScheduleResponse::from)
    .ok_or_else(|| AppError::NotFound("La programación no existe".to_string()))
}

pub async fn list_schedules(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(filter): Query<ScheduleFilter>,
) -> Result<Json<Value>, AppError> {
    let rows = match (&filter.tag_id, filter.enabled) {
        (Some(tag_id), Some(enabled)) => sqlx::query_as::<_, ScheduleRow>(
            "SELECT id, name, tag_id, target_value, trigger_type, cron_expr, start_date, \
             end_date, enabled, requires_confirmation, created_by, created_at \
             FROM schedules WHERE tag_id = ? AND enabled = ? ORDER BY created_at ASC",
        )
        .bind(tag_id)
        .bind(enabled)
        .fetch_all(&state.pool)
        .await?,
        (Some(tag_id), None) => sqlx::query_as::<_, ScheduleRow>(
            "SELECT id, name, tag_id, target_value, trigger_type, cron_expr, start_date, \
             end_date, enabled, requires_confirmation, created_by, created_at \
             FROM schedules WHERE tag_id = ? ORDER BY created_at ASC",
        )
        .bind(tag_id)
        .fetch_all(&state.pool)
        .await?,
        (None, Some(enabled)) => sqlx::query_as::<_, ScheduleRow>(
            "SELECT id, name, tag_id, target_value, trigger_type, cron_expr, start_date, \
             end_date, enabled, requires_confirmation, created_by, created_at \
             FROM schedules WHERE enabled = ? ORDER BY created_at ASC",
        )
        .bind(enabled)
        .fetch_all(&state.pool)
        .await?,
        (None, None) => sqlx::query_as::<_, ScheduleRow>(
            "SELECT id, name, tag_id, target_value, trigger_type, cron_expr, start_date, \
             end_date, enabled, requires_confirmation, created_by, created_at \
             FROM schedules ORDER BY created_at ASC",
        )
        .fetch_all(&state.pool)
        .await?,
    };

    let schedules: Vec<ScheduleResponse> =
        rows.into_iter().map(ScheduleResponse::from).collect();

    Ok(Json(json!({ "ok": true, "schedules": schedules })))
}

pub async fn create_schedule(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<SchedulePayload>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    validate_schedule_payload(&payload)?;

    let (data_type, allowed_values) = fetch_tag_constraints(&state, &payload.tag_id).await?;
    validate_target_value(data_type, allowed_values.as_ref(), &payload.target_value)?;

    let id = Uuid::new_v4().to_string();
    sqlx::query(
        r#"INSERT INTO schedules (
            id, name, tag_id, target_value, trigger_type, cron_expr,
            start_date, end_date, enabled, requires_confirmation, created_by
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(payload.name.trim())
    .bind(&payload.tag_id)
    .bind(SqlxJson(&payload.target_value))
    .bind(payload.trigger_type)
    .bind(&payload.cron_expr)
    .bind(&payload.start_date)
    .bind(&payload.end_date)
    .bind(payload.enabled)
    .bind(payload.requires_confirmation)
    .bind(&auth.uid)
    .execute(&state.pool)
    .await?;

    let schedule = fetch_schedule(&state, &id).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "ok": true, "schedule": schedule })),
    ))
}

pub async fn update_schedule(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<SchedulePayload>,
) -> Result<Json<Value>, AppError> {
    validate_schedule_payload(&payload)?;

    let (data_type, allowed_values) = fetch_tag_constraints(&state, &payload.tag_id).await?;
    validate_target_value(data_type, allowed_values.as_ref(), &payload.target_value)?;

    let result = sqlx::query(
        r#"UPDATE schedules SET
            name = ?, tag_id = ?, target_value = ?, trigger_type = ?, cron_expr = ?,
            start_date = ?, end_date = ?, enabled = ?, requires_confirmation = ?
        WHERE id = ?"#,
    )
    .bind(payload.name.trim())
    .bind(&payload.tag_id)
    .bind(SqlxJson(&payload.target_value))
    .bind(payload.trigger_type)
    .bind(&payload.cron_expr)
    .bind(&payload.start_date)
    .bind(&payload.end_date)
    .bind(payload.enabled)
    .bind(payload.requires_confirmation)
    .bind(&id)
    .execute(&state.pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("La programación no existe".to_string()));
    }

    let schedule = fetch_schedule(&state, &id).await?;
    Ok(Json(json!({ "ok": true, "schedule": schedule })))
}

pub async fn delete_schedule(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let result = sqlx::query("DELETE FROM schedules WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("La programación no existe".to_string()));
    }

    Ok(Json(json!({ "ok": true })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scada::models::{ReadWrite, TagKind};
    use serde_json::json;

    #[test]
    fn rejects_a_blank_site_name() {
        let payload = SitePayload {
            name: "   ".to_string(),
        };
        assert!(validate_site_payload(&payload).is_err());
    }

    #[test]
    fn rejects_an_asset_without_kind() {
        let payload = AssetPayload {
            site_id: "site-1".to_string(),
            parent_asset_id: None,
            name: "Area 1".to_string(),
            kind: "  ".to_string(),
        };
        assert!(validate_asset_payload(&payload).is_err());
    }

    #[test]
    fn validates_config_keys_per_protocol() {
        let incomplete = json!({ "endpoint_url": "opc.tcp://x" });
        assert!(validate_connection_config(Protocol::Opcua, &incomplete).is_err());

        let complete = json!({
            "endpoint_url": "opc.tcp://x",
            "security_policy": "None",
            "credentials_ref": "cred-1",
        });
        assert!(validate_connection_config(Protocol::Opcua, &complete).is_ok());

        let mqtt = json!({
            "broker_url": "mqtt://x",
            "base_topic": "plant/area",
            "qos": 1,
            "tls": false,
        });
        assert!(validate_connection_config(Protocol::Mqtt, &mqtt).is_ok());
    }

    #[test]
    fn validates_address_keys_per_protocol() {
        let incomplete = json!({ "db_number": 1 });
        assert!(validate_tag_address(Protocol::S7, &incomplete).is_err());

        let complete = json!({ "db_number": 1, "offset": 0, "bit": 3, "s7_type": "bool" });
        assert!(validate_tag_address(Protocol::S7, &complete).is_ok());
    }

    fn tag_payload(data_type: DataType, min: Option<f64>, max: Option<f64>) -> TagPayload {
        TagPayload {
            connection_id: "conn-1".to_string(),
            asset_id: "asset-1".to_string(),
            name: "Motor1_Run".to_string(),
            description: String::new(),
            tag_kind: TagKind::Telemando,
            data_type,
            unit: None,
            address: json!({ "node_id": "ns=2;s=Motor1.Run" }),
            read_write: ReadWrite::Write,
            min_value: min,
            max_value: max,
            allowed_values: None,
            requires_sbo: false,
            requires_ack: false,
        }
    }

    #[test]
    fn rejects_min_value_greater_than_max_value() {
        let payload = tag_payload(DataType::Float, Some(10.0), Some(5.0));
        assert!(validate_tag_payload(&payload).is_err());
    }

    #[test]
    fn requires_allowed_values_for_enum_tags() {
        let payload = tag_payload(DataType::Enum, None, None);
        assert!(validate_tag_payload(&payload).is_err());
    }

    fn schedule_payload(trigger_type: TriggerType) -> SchedulePayload {
        SchedulePayload {
            name: "Arranque".to_string(),
            tag_id: "tag-1".to_string(),
            target_value: json!(true),
            trigger_type,
            cron_expr: None,
            start_date: None,
            end_date: None,
            enabled: true,
            requires_confirmation: false,
        }
    }

    #[test]
    fn requires_start_date_for_once_schedules() {
        assert!(validate_schedule_payload(&schedule_payload(TriggerType::Once)).is_err());
    }

    #[test]
    fn requires_cron_expr_for_cron_schedules() {
        assert!(validate_schedule_payload(&schedule_payload(TriggerType::Cron)).is_err());
    }

    #[test]
    fn validates_target_value_against_data_type() {
        assert!(validate_target_value(DataType::Bool, None, &json!(true)).is_ok());
        assert!(validate_target_value(DataType::Bool, None, &json!("true")).is_err());
        assert!(validate_target_value(DataType::Int, None, &json!(42)).is_ok());
        assert!(validate_target_value(DataType::Int, None, &json!(4.2)).is_err());

        let allowed = json!(["auto", "manual"]);
        assert!(validate_target_value(DataType::Enum, Some(&allowed), &json!("auto")).is_ok());
        assert!(validate_target_value(DataType::Enum, Some(&allowed), &json!("off")).is_err());
    }
}
