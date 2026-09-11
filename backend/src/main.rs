mod auth;
mod config;
mod error;
mod events;
mod routes;
mod state;

use std::str::FromStr;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

// Importamos las herramientas de CORS
use tower_http::cors::{Any, CorsLayer}; 
use axum::http::HeaderValue;

use config::AppConfig;
use state::AppState;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = AppConfig::from_env();

    let connect_options = SqliteConnectOptions::from_str(&config.database_url)
        .expect("DATABASE_URL no es una cadena de conexión válida")
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await
        .expect("No se pudo conectar a la base de datos SQLite");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("No se pudieron ejecutar las migraciones");

    let port = config.port;

    // Guardamos una copia del allowed_origin antes de mover config a state
    let origin_str = config.allowed_origin.clone();

    let state = AppState {
        pool,
        jwt_secret: config.jwt_secret,
        jwt_expires_seconds: config.jwt_expires_seconds,
        allowed_origin: config.allowed_origin,
    };

    // 2. Configuramos la capa (Layer) de CORS usando tu variable de entorno
    let cors = CorsLayer::new()
        .allow_origin(
            origin_str
                .parse::<HeaderValue>()
                .expect("El origen configurado en allowed_origin no es válido")
        )
        .allow_methods(Any) // Permite GET, POST, PUT, DELETE, etc.
        .allow_headers(Any); // Permite cualquier cabecera (Content-Type, Authorization, etc.)


    let app = routes::build_router(state).layer(cors);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .expect("No se pudo enlazar el puerto");

    tracing::info!("Servidor escuchando en http://0.0.0.0:{port}");

    axum::serve(listener, app)
        .await
        .expect("Error al ejecutar el servidor");
}
