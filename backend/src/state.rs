use sqlx::SqlitePool;

use crate::scheduler_link::SchedulerHub;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub jwt_secret: String,
    pub jwt_expires_seconds: i64,
    pub allowed_origin: String,
    pub scheduler: SchedulerHub,
}
