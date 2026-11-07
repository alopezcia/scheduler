use axum::Json;
use axum::extract::State;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::auth::jwt::generate_token;
use crate::auth::role::Role;
use crate::auth::models::{AuthResponse, LoginPayload, RegisterPayload, UserRow};
use crate::error::AppError;
use crate::state::AppState;

pub const MIN_PASSWORD_LEN: usize = 6;

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<AuthResponse>, AppError> {
    let email = payload.email.trim().to_lowercase();

    let user = sqlx::query_as::<_, UserRow>(
        "SELECT id, name, password_hash, role FROM users WHERE email = ?",
    )
    .bind(&email)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::BadRequest("Usuario / Password no son correctos".to_string()))?;

    let valid = bcrypt::verify(&payload.password, &user.password_hash)
        .map_err(|err| AppError::Internal(err.to_string()))?;

    if !valid {
        return Err(AppError::BadRequest(
            "Usuario / Password no son correctos".to_string(),
        ));
    }

    let token = generate_token(
        &user.id,
        &user.name,
        &state.jwt_secret,
        state.jwt_expires_seconds,
    )
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(Json(AuthResponse {
        ok: true,
        uid: user.id,
        name: user.name,
        role: Role::parse(&user.role).unwrap_or(Role::Consultor).as_str(),
        token,
    }))
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterPayload>,
) -> Result<Json<AuthResponse>, AppError> {
    let name = payload.name.trim().to_string();
    let email = payload.email.trim().to_lowercase();

    if name.is_empty() || email.is_empty() || payload.password.len() < MIN_PASSWORD_LEN {
        return Err(AppError::BadRequest(format!(
            "Nombre y correo son obligatorios y la contraseña debe tener al menos {MIN_PASSWORD_LEN} caracteres"
        )));
    }

    let existing: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE email = ?")
        .bind(&email)
        .fetch_one(&state.pool)
        .await?;

    if existing > 0 {
        return Err(AppError::BadRequest("El usuario ya existe".to_string()));
    }

    let password_hash = bcrypt::hash(&payload.password, bcrypt::DEFAULT_COST)
        .map_err(|err| AppError::Internal(err.to_string()))?;
    let id = Uuid::new_v4().to_string();

    // El primer usuario de todos es administrador (si no, nadie podría dar de alta al resto);
    // los demás entran como consultor y un administrador les sube el rol.
    let role: String = sqlx::query_scalar(
        "INSERT INTO users (id, name, email, password_hash, role)
         VALUES (?, ?, ?, ?, CASE WHEN (SELECT COUNT(*) FROM users) = 0 THEN 'administrador' ELSE 'consultor' END)
         RETURNING role",
    )
    .bind(&id)
    .bind(&name)
    .bind(&email)
    .bind(&password_hash)
    .fetch_one(&state.pool)
    .await?;

    let token = generate_token(&id, &name, &state.jwt_secret, state.jwt_expires_seconds)
        .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(Json(AuthResponse {
        ok: true,
        uid: id,
        name,
        role: Role::parse(&role).unwrap_or(Role::Consultor).as_str(),
        token,
    }))
}

pub async fn renew(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<AuthResponse>, AppError> {
    let token = generate_token(
        &auth.uid,
        &auth.name,
        &state.jwt_secret,
        state.jwt_expires_seconds,
    )
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(Json(AuthResponse {
        ok: true,
        uid: auth.uid,
        name: auth.name,
        role: auth.role.as_str(),
        token,
    }))
}
