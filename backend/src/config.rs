use std::env;

#[derive(Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_expires_seconds: i64,
    pub port: u16,
    pub allowed_origin: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://calendar.db".to_string()),
            jwt_secret: env::var("JWT_SECRET")
                .expect("La variable de entorno JWT_SECRET es obligatoria"),
            jwt_expires_seconds: env::var("JWT_EXPIRES_SECONDS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(86_400),
            port: env::var("PORT")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(4000),
            allowed_origin: env::var("ALLOWED_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:5173".to_string()),
        }
    }
}
