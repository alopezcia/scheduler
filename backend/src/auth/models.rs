use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct LoginPayload {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterPayload {
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub ok: bool,
    pub uid: String,
    pub name: String,
    pub token: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct UserRow {
    pub id: String,
    pub name: String,
    pub password_hash: String,
}
