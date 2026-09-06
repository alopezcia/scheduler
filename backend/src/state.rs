use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub jwt_secret: String,
    pub jwt_expires_seconds: i64,
    pub allowed_origin: String,
}
