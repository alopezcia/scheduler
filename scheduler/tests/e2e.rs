//! Prueba de extremo a extremo: esquema real del backend (sus migraciones), un SCADA REST
//! falso con axum y el ejecutor del scheduler.

use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, put};
use axum::{Json, Router};
use chrono::{Duration, SecondsFormat, Utc};
use scheduler::control::{Control, serve_connection};
use scheduler::executor::run_due_tasks;
use scheduler::rest::RestClient;
use serde_json::{Value, json};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Row, SqlitePool};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

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

async fn slow(State(s): State<Arc<Scada>>) -> StatusCode {
    s.writes.lock().unwrap().push("slow".to_string());
    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    StatusCode::OK
}

async fn start_scada() -> (String, Arc<Scada>) {
    let state = Arc::new(Scada::default());
    let app = Router::new()
        .route("/api/tags/pump1", put(write_pump).get(read_pump))
        .route("/api/tags/broken", put(broken))
        .route("/api/tags/slow", put(slow))
        .route("/api/tags/open", put(|| async { StatusCode::OK }))
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
    let processed = run_due_tasks(&pool, &client, &Control::new(), Utc::now(), max_late).await.unwrap();
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
    assert_eq!(run_due_tasks(&pool, &client, &Control::new(), Utc::now(), max_late).await.unwrap(), 0);
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

    run_due_tasks(&pool, &RestClient::new(), &Control::new(), Utc::now(), Duration::seconds(300)).await.unwrap();

    let (status, err, _, sent) = execution(&pool, "ev").await.unwrap();
    assert_eq!((status.as_str(), sent), ("failed", false));
    assert!(err.unwrap().contains("SCHEDULER_CRED_NO_DEFINIDA"));
    assert!(scada.writes.lock().unwrap().is_empty());
}

async fn next_msg<R: tokio::io::AsyncBufRead + Unpin>(lines: &mut tokio::io::Lines<R>) -> Value {
    serde_json::from_str(&lines.next_line().await.unwrap().expect("conexión cerrada")).unwrap()
}

/// Base con una conexión REST (sin auth) y un tag por cada ruta de `tags`.
async fn rest_db(base_url: &str, tags: &[&str]) -> SqlitePool {
    let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
    sqlx::migrate!("../backend/migrations").run(&pool).await.unwrap();
    exec(&pool, "INSERT INTO users (id, name, email, password_hash) VALUES ('u', 'n', 'e', 'h')", &[]).await;
    exec(&pool, "INSERT INTO sites (id, name) VALUES ('s', 'S')", &[]).await;
    exec(&pool, "INSERT INTO assets (id, site_id, name, kind) VALUES ('a', 's', 'A', 'plc')", &[]).await;
    let cfg = json!({ "scada_vendor": "ignition", "base_url": base_url, "auth_type": "none", "timeout_ms": 60000 }).to_string();
    exec(&pool, "INSERT INTO connections (id, asset_id, name, protocol, config) VALUES ('rest', 'a', 'R', 'rest', ?)", &[&cfg]).await;
    for tag in tags {
        add_tag(&pool, tag, "rest", &format!("/tags/{tag}"), (0, 0)).await;
    }
    pool
}

#[tokio::test]
async fn paused_scheduler_picks_up_nothing_until_resumed() {
    let (base_url, scada) = start_scada().await;
    let pool = rest_db(&base_url, &["open"]).await;
    add_event(&pool, "ev", "open", "1", 0, &at(Duration::seconds(-5))).await;

    let control = Control::new();
    control.set_paused(true);
    let client = RestClient::new();
    let max_late = Duration::seconds(300);

    assert_eq!(run_due_tasks(&pool, &client, &control, Utc::now(), max_late).await.unwrap(), 0);
    assert!(execution(&pool, "ev").await.is_none());
    assert!(scada.writes.lock().unwrap().is_empty());

    control.set_paused(false);
    assert_eq!(run_due_tasks(&pool, &client, &control, Utc::now(), max_late).await.unwrap(), 1);
    assert_eq!(execution(&pool, "ev").await.unwrap().0, "sent");
}

#[tokio::test]
async fn control_connection_reports_status_and_cancels_a_running_execution() {
    let (base_url, scada) = start_scada().await;
    let pool = rest_db(&base_url, &["slow"]).await;
    add_event(&pool, "ev", "slow", "1", 0, &at(Duration::seconds(-5))).await;

    let control = Arc::new(Control::new());
    let (backend_side, scheduler_side) = tokio::io::duplex(8192);
    let served = control.clone();
    tokio::spawn(async move { serve_connection(scheduler_side, &served).await });
    let (reader, mut writer) = tokio::io::split(backend_side);
    let mut lines = BufReader::new(reader).lines();

    // Al conectar llega el estado inicial.
    assert_eq!(next_msg(&mut lines).await, json!({ "type": "status", "paused": false, "running": [] }));

    // La ejecución lenta arranca en segundo plano y se notifica.
    let runner = {
        let (pool, control) = (pool.clone(), control.clone());
        tokio::spawn(async move {
            run_due_tasks(&pool, &RestClient::new(), &control, Utc::now(), Duration::seconds(300)).await.unwrap()
        })
    };
    let started = next_msg(&mut lines).await;
    assert_eq!((started["type"].as_str(), started["status"].as_str()), (Some("execution"), Some("started")));
    let execution_id = started["execution_id"].as_str().unwrap().to_string();
    let status = next_msg(&mut lines).await;
    assert_eq!(status["running"][0]["execution_id"], json!(execution_id));

    // Cancelar una ejecución que no existe falla; la que corre se cancela.
    let cancel = |id: &str| format!("{}
", json!({ "id": "1", "cmd": "cancel", "execution_id": id }));
    writer.write_all(cancel("nope").as_bytes()).await.unwrap();
    let reply = next_msg(&mut lines).await;
    assert_eq!((reply["type"].as_str(), reply["ok"].as_bool()), (Some("reply"), Some(false)));

    writer.write_all(cancel(&execution_id).as_bytes()).await.unwrap();
    assert_eq!(runner.await.unwrap(), 1);

    let (status, err, _, sent) = execution(&pool, "ev").await.unwrap();
    assert_eq!((status.as_str(), sent), ("cancelled", false));
    assert!(err.unwrap().contains("Cancelada"));
    assert_eq!(*scada.writes.lock().unwrap(), vec!["slow".to_string()]);

    // Pausar desde el control se refleja en el estado difundido.
    writer.write_all(b"{\"cmd\":\"pause\"}
").await.unwrap();
    loop {
        let msg = next_msg(&mut lines).await;
        if msg["type"] == "status" && msg["paused"] == true {
            break;
        }
    }
    assert!(control.is_paused());
}
