use std::env;

pub struct Config {
    pub database_url: String,
    pub poll_interval_seconds: u64,
    pub max_late_seconds: i64,
    /// Socket Unix por el que el backend controla el scheduler.
    pub socket_path: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://calendar.db".to_string()),
            poll_interval_seconds: parse_env("POLL_INTERVAL_SECONDS", 5),
            max_late_seconds: parse_env("MAX_LATE_SECONDS", 300),
            socket_path: env::var("SCHEDULER_SOCKET").unwrap_or_else(|_| "/tmp/scheduler.sock".to_string()),
        }
    }
}

fn parse_env<T: std::str::FromStr>(key: &str, default: T) -> T {
    match env::var(key) {
        Ok(raw) => raw
            .parse()
            .unwrap_or_else(|_| panic!("{key} no es un número válido: {raw}")),
        Err(_) => default,
    }
}
