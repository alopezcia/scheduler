use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use scheduler::config::Config;
use scheduler::control::Control;
use scheduler::executor::run_due_tasks;
use scheduler::rest::RestClient;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = Config::from_env();

    // No se crea la base de datos ni se migra: es del backend. Si falta, se aborta.
    let options = SqliteConnectOptions::from_str(&config.database_url)
        .expect("DATABASE_URL no es una cadena de conexión válida")
        .create_if_missing(false)
        .busy_timeout(Duration::from_secs(10));

    let pool = SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(options)
        .await
        .expect("No se pudo abrir la base de datos SQLite (¿ha arrancado el backend para crearla?)");

    // Falla pronto y con un mensaje claro si el backend aún no aplicó las migraciones necesarias.
    scheduler::db::pending_tasks(&pool)
        .await
        .expect("El esquema no es el esperado: arranca el backend para que aplique las migraciones");

    let client = RestClient::new();
    let control = Arc::new(Control::new());
    spawn_control_socket(&config.socket_path, control.clone());
    let max_late = chrono::Duration::seconds(config.max_late_seconds);
    let mut ticker = tokio::time::interval(Duration::from_secs(config.poll_interval_seconds));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    tracing::info!(
        "Scheduler iniciado (sondeo cada {} s, tolerancia {} s)",
        config.poll_interval_seconds,
        config.max_late_seconds
    );

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                if let Err(e) = run_due_tasks(&pool, &client, &control, Utc::now(), max_late).await {
                    tracing::error!("Error consultando la base de datos: {e}");
                }
            }
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Señal de parada recibida, saliendo");
                break;
            }
        }
    }
}

#[cfg(unix)]
fn spawn_control_socket(path: &str, control: Arc<Control>) {
    let path = std::path::PathBuf::from(path);
    tokio::spawn(async move {
        if let Err(e) = scheduler::socket::serve(&path, control).await {
            tracing::error!("El socket de control ({}) ha fallado: {e}", path.display());
        }
    });
}

#[cfg(not(unix))]
fn spawn_control_socket(_path: &str, _control: Arc<Control>) {
    tracing::warn!("Los sockets Unix no están disponibles en esta plataforma: el scheduler no se podrá controlar desde el backend");
}
