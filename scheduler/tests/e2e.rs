//! Prueba de extremo a extremo: esquema real del backend (sus migraciones), un SCADA REST
//! falso con axum y el ejecutor del scheduler.

use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, put};
use axum::{Json, Router};
use chrono::{Duration, SecondsFormat, Utc};
use scheduler::executor::run_due_tasks;
use scheduler::rest::RestClient;
use serde_json::{Value, json};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Row, SqlitePool};

#[derive(Default)]
struct Scada {
    value: Mutex<Option<Value>>,
    writes: Mutex<Vec<String>>,
}

async fn write_pump(State(s): State<Arc<Scada>>, headers: HeaderMap, Json(body): Json<Value>) -> StatusCode {
    if headers.get("authorization").and_then(|h| h.to_str().ok()) != Some("Bearer secret-token") {
        return StatusCode::UNAUTHORIZED;
    }
    s.writes.lock().unwrap().push("pump1".to_string());
    *s.value.lock().unwrap() = body.get("value").cloned();
    StatusCode::OK
}

async fn read_pump(State(s): State<Arc<Scada>>) -> Json<Value> {
    Json(json!({ "value": s.value.lock().unwrap().clone() }))
}

async fn broken(State(s): State<Arc<Scada>>) -> (StatusCode, &'static str) {
    s.writes.lock().unwrap().push("broken".to_string());
    (StatusCode::INTERNAL_SERVER_ERROR, "tag offline")
}

async fn start_scada() -> (String, Arc<Scada>) {
    let state = Arc::new(Scada::default());
    let app = Router::new()
        .route("/api/tags/pump1", put(write_pump).get(read_pump))
        .route("/api/tags/broken", put(broken))
        .route("/api/tags/guarded", get(read_pump).put(broken))
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/api", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (url, state)
}

fn at(offset: Duration) -> String {
    (Utc::now() + offset).to_rfc3339_opts(SecondsFormat::Millis, true)
}

async fn exec(pool: &SqlitePool, sql: &'static str, binds: &[&str]) {
    let mut q = sqlx::query(sql);
    for b in binds {
        q = q.bind(*b);
    }
    q.execute(pool).await.unwrap();
}

async fn add_tag(pool: &SqlitePool, id: &str, conn: &str, path: &str, flags: (i32, i32)) {
    let address = json!({ "method": "PUT", "path": path, "json_pointer": "/value" }).to_string();
    sqlx::query(
        "INSERT INTO tags (id, connection_id, asset_id, name, tag_kind, data_type, address, requires_sbo, requires_ack)
         VALUES (?, ?, 'a', ?, 'consigna', 'int', ?, ?, ?)",
    )
    .bind(id)
    .bind(conn)
    .bind(id)
    .bind(address)
    .bind(flags.0)
    .bind(flags.1)
    .execute(pool)
    .await
    .unwrap();
}

async fn add_event(pool: &SqlitePool, id: &str, tag: &str, value: &str, confirm: i32, start: &str) {
    let schedule = format!("s-{id}");
    sqlx::query(
        "INSERT INTO schedules (id, name, tag_id, target_value, trigger_type, start_date, requires_confirmation, created_by)
         VALUES (?, ?, ?, ?, 'once', ?, ?, 'u')",
    )
    .bind(&schedule)
    .bind(id)
    .bind(tag)
    .bind(value)
    .bind(start)
    .bind(confirm)
    .execute(pool)
    .await
    .unwrap();
    exec(pool, "INSERT INTO events (id, schedule_id, start_date, user_id) VALUES (?, ?, ?, 'u')", &[id, &schedule, start]).await;
}

async fn execution(pool: &SqlitePool, event: &str) -> Option<(String, Option<String>, Option<String>, bool)> {
    sqlx::query("SELECT status, error_message, response, sent_at IS NOT NULL AS sent FROM command_executions WHERE event_id = ?")
        .bind(event)
        .fetch_optional(pool)
        .await
        .unwrap()
        .map(|r| (r.get("status"), r.get("error_message"), r.get("response"), r.get("sent")))
}

#[tokio::test]
async fn executes_due_rest_tasks_once_and_audits_each_outcome() {
    unsafe { std::env::set_var("SCHEDULER_CRED_MOCK_SCADA", "secret-token") };
    let (base_url, scada) = start_scada().await;

    let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
    sqlx::migrate!("../backend/migrations").run(&pool).await.unwrap();

    exec(&pool, "INSERT INTO users (id, name, email, password_hash) VALUES ('u', 'n', 'e', 'h')", &[]).await;
    exec(&pool, "INSERT INTO sites (id, name) VALUES ('s', 'S')", &[]).await;
    exec(&pool, "INSERT INTO assets (id, site_id, name, kind) VALUES ('a', 's', 'A', 'plc')", &[]).await;
    let rest_config = json!({
        "scada_vendor": "ignition", "base_url": base_url, "auth_type": "bearer",
        "credentials_ref": "mock-scada", "timeout_ms": 2000
    })
    .to_string();
    exec(&pool, "INSERT INTO connections (id, asset_id, name, protocol, config) VALUES ('rest', 'a', 'R', 'rest', ?)", &[&rest_config]).await;
    exec(&pool, "INSERT INTO connections (id, asset_id, name, protocol, config) VALUES ('mqtt', 'a', 'M', 'mqtt', '{}')", &[]).await;

    add_tag(&pool, "pump1", "rest", "/tags/pump1", (0, 1)).await;
    add_tag(&pool, "broken", "rest", "/tags/broken", (0, 0)).await;
    add_tag(&pool, "guarded", "rest", "/tags/guarded", (0, 0)).await;
    add_tag(&pool, "cond", "rest", "/tags/pump1", (0, 0)).await;
    add_tag(&pool, "sbo", "rest", "/tags/pump1", (1, 0)).await;
    add_tag(&pool, "mq", "mqtt", "/x", (0, 0)).await;
    exec(&pool, "INSERT INTO interlocks (id, tag_id, condition_tag_id, operator, condition_value) VALUES ('i', 'guarded', 'cond', 'eq', '1')", &[]).await;

    let due = at(Duration::seconds(-10));
    add_event(&pool, "ev-ok", "pump1", "7", 0, &due).await;
    add_event(&pool, "ev-500", "broken", "1", 0, &due).await;
    add_event(&pool, "ev-interlock", "guarded", "1", 0, &due).await;
    add_event(&pool, "ev-sbo", "sbo", "1", 0, &due).await;
    add_event(&pool, "ev-confirm", "pump1", "9", 1, &due).await;
    add_event(&pool, "ev-mqtt", "mq", "1", 0, &due).await;
    add_event(&pool, "ev-future", "pump1", "9", 0, &at(Duration::hours(1))).await;
    add_event(&pool, "ev-old", "pump1", "9", 0, &at(Duration::hours(-1))).await;

    let client = RestClient::new();
    let max_late = Duration::seconds(300);
    let processed = run_due_tasks(&pool, &client, Utc::now(), max_late).await.unwrap();
    assert_eq!(processed, 5);

    let (status, _, response, sent) = execution(&pool, "ev-ok").await.unwrap();
    assert_eq!((status.as_str(), sent), ("ack", true));
    assert!(response.is_some());
    assert_eq!(*scada.value.lock().unwrap(), Some(json!(7)));

    let (status, err, response, sent) = execution(&pool, "ev-500").await.unwrap();
    assert_eq!((status.as_str(), sent), ("failed", false));
    assert!(err.is_some());
    assert_eq!(response.as_deref(), Some("tag offline"));

    let (status, err, _, sent) = execution(&pool, "ev-interlock").await.unwrap();
    assert_eq!((status.as_str(), sent), ("failed", false));
    assert!(err.unwrap().contains("enclavamientos"));

    let (status, err, _, _) = execution(&pool, "ev-sbo").await.unwrap();
    assert_eq!(status, "failed");
    assert!(err.unwrap().contains("select-before-operate"));

    let (status, err, _, sent) = execution(&pool, "ev-old").await.unwrap();
    assert_eq!((status.as_str(), sent), ("timeout", false));
    assert!(err.unwrap().contains("perdido"));

    for ignored in ["ev-confirm", "ev-mqtt", "ev-future"] {
        assert!(execution(&pool, ignored).await.is_none(), "{ignored} no debe ejecutarse");
    }
    // El enclavado y el SBO no llegaron a enviarse al SCADA.
    assert_eq!(*scada.writes.lock().unwrap(), vec!["pump1".to_string(), "broken".to_string()]);

    // Idempotencia: una segunda pasada no repite nada.
    assert_eq!(run_due_tasks(&pool, &client, Utc::now(), max_late).await.unwrap(), 0);
    assert_eq!(scada.writes.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn fails_when_the_credentials_secret_is_missing() {
    let (base_url, scada) = start_scada().await;
    let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
    sqlx::migrate!("../backend/migrations").run(&pool).await.unwrap();

    exec(&pool, "INSERT INTO users (id, name, email, password_hash) VALUES ('u', 'n', 'e', 'h')", &[]).await;
    exec(&pool, "INSERT INTO sites (id, name) VALUES ('s', 'S')", &[]).await;
    exec(&pool, "INSERT INTO assets (id, site_id, name, kind) VALUES ('a', 's', 'A', 'plc')", &[]).await;
    let cfg = json!({
        "scada_vendor": "wincc", "base_url": base_url, "auth_type": "bearer",
        "credentials_ref": "no-definida", "timeout_ms": 2000
    })
    .to_string();
    exec(&pool, "INSERT INTO connections (id, asset_id, name, protocol, config) VALUES ('rest', 'a', 'R', 'rest', ?)", &[&cfg]).await;
    add_tag(&pool, "pump1", "rest", "/tags/pump1", (0, 0)).await;
    add_event(&pool, "ev", "pump1", "1", 0, &at(Duration::seconds(-5))).await;

    run_due_tasks(&pool, &RestClient::new(), Utc::now(), Duration::seconds(300)).await.unwrap();

    let (status, err, _, sent) = execution(&pool, "ev").await.unwrap();
    assert_eq!((status.as_str(), sent), ("failed", false));
    assert!(err.unwrap().contains("SCHEDULER_CRED_NO_DEFINIDA"));
    assert!(scada.writes.lock().unwrap().is_empty());
}
