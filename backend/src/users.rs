//! Gestión de usuarios (solo administradores): listar, crear y cambiar el rol.

use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::Admin;
use crate::auth::handlers::MIN_PASSWORD_LEN;
use crate::auth::role::Role;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UserSummary {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct RolePayload {
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserPayload {
    pub name: String,
    pub email: String,
    pub password: String,
    pub role: String,
}

pub async fn create_user(
    admin: Admin,
    State(state): State<AppState>,
    Json(payload): Json<CreateUserPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    let name = payload.name.trim().to_string();
    let email = payload.email.trim().to_lowercase();
    let role = Role::parse(&payload.role).ok_or_else(|| {
        AppError::BadRequest("El rol debe ser consultor, planificador o administrador".to_string())
    })?;

    if name.is_empty() || !email.contains('@') || payload.password.len() < MIN_PASSWORD_LEN {
        return Err(AppError::BadRequest(format!(
            "Nombre y correo válido son obligatorios y la contraseña debe tener al menos {MIN_PASSWORD_LEN} caracteres"
        )));
    }

    let password_hash = bcrypt::hash(&payload.password, bcrypt::DEFAULT_COST)
        .map_err(|err| AppError::Internal(err.to_string()))?;
    let id = Uuid::new_v4().to_string();

    sqlx::query("INSERT INTO users (id, name, email, password_hash, role) VALUES (?, ?, ?, ?, ?)")
        .bind(&id)
        .bind(&name)
        .bind(&email)
        .bind(&password_hash)
        .bind(role.as_str())
        .execute(&state.pool)
        .await
        .map_err(|err| match &err {
            // El UNIQUE de `email` también cubre dos altas simultáneas con el mismo correo.
            sqlx::Error::Database(db) if db.is_unique_violation() => AppError::BadRequest("El usuario ya existe".to_string()),
            _ => AppError::from(err),
        })?;

    tracing::info!(by = %admin.name, target = %id, role = role.as_str(), "Usuario creado");
    Ok(Json(serde_json::json!({ "ok": true, "user": UserSummary { id, name, email, role: role.as_str().to_string() } })))
}

pub async fn list_users(
    _admin: Admin,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let users = sqlx::query_as::<_, UserSummary>("SELECT id, name, email, role FROM users ORDER BY name, email")
        .fetch_all(&state.pool)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true, "users": users })))
}

pub async fn update_role(
    admin: Admin,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<RolePayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    let role = Role::parse(&payload.role).ok_or_else(|| {
        AppError::BadRequest("El rol debe ser consultor, planificador o administrador".to_string())
    })?;

    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.pool)
        .await?;
    if exists == 0 {
        return Err(AppError::NotFound("El usuario no existe".to_string()));
    }

    // Un solo UPDATE: la condición evita dejar el sistema sin administradores aunque dos
    // administradores se degraden a la vez.
    let result = sqlx::query(
        "UPDATE users SET role = ?1
         WHERE id = ?2
           AND (role <> 'administrador'
                OR ?1 = 'administrador'
                OR (SELECT COUNT(*) FROM users WHERE role = 'administrador') > 1)",
    )
    .bind(role.as_str())
    .bind(&id)
    .execute(&state.pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::BadRequest("Debe quedar al menos un administrador".to_string()));
    }

    tracing::info!(by = %admin.name, target = %id, role = role.as_str(), "Rol de usuario cambiado");
    Ok(Json(serde_json::json!({ "ok": true, "id": id, "role": role.as_str() })))
}
