//! WebSocket entre el frontend y el backend: el navegador controla el scheduler (pausar,
//! reanudar, cancelar una ejecución) y recibe sus notificaciones, siempre a través del
//! backend (que habla con el scheduler por `scheduler_link`).
//!
//! Protocolo (JSON de texto):
//!   1. El cliente envía primero `{ "type": "auth", "token": "<jwt>" }` (un navegador no puede
//!      poner cabeceras en un WebSocket, y así el token no acaba en la URL ni en los logs).
//!      El usuario debe existir; su rol llega en la respuesta `{ "type": "auth", "ok": true, "role" }`.
//!   2. El servidor emite `{ "type": "link", "connected": bool }` (¿está el scheduler
//!      conectado?), `status`, `execution` y `reply`, tal y como los define el scheduler.
//!   3. El cliente envía `{ "id"?, "cmd": "get_status" | "pause" | "resume" | "cancel",
//!      "execution_id"? }` y recibe `{ "type": "reply", "id", "ok", "error" }`.
//!      `pause`, `resume` y `cancel` solo las acepta un administrador; `get_status` cualquiera.

use std::time::Duration;

use axum::extract::State;
use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::broadcast;

use crate::auth::role::Role;
use crate::auth::jwt::verify_token;
use crate::auth::role::role_of;
use crate::state::AppState;

const AUTH_TIMEOUT: Duration = Duration::from_secs(5);
/// Código de cierre de aplicación para "no autorizado".
const CLOSE_UNAUTHORIZED: u16 = 4401;

#[derive(Deserialize)]
struct AuthMessage {
    #[serde(rename = "type")]
    kind: String,
    token: String,
}

#[derive(Deserialize)]
struct ClientRequest {
    id: Option<String>,
    #[serde(flatten)]
    command: ClientCommand,
}

/// Lista cerrada de órdenes: lo que no esté aquí no llega nunca al scheduler.
#[derive(Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
enum ClientCommand {
    GetStatus,
    Pause,
    Resume,
    Cancel { execution_id: String },
}

pub async fn upgrade(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| client_session(socket, state))
}

/// Devuelve `(uid, nombre)` del usuario si el token es válido y el usuario sigue existiendo.
async fn authenticate(socket: &mut WebSocket, state: &AppState) -> Option<(String, String)> {
    let first = tokio::time::timeout(AUTH_TIMEOUT, socket.recv()).await.ok()??.ok()?;
    let Message::Text(text) = first else { return None };
    let auth: AuthMessage = serde_json::from_str(&text).ok()?;
    if auth.kind != "auth" {
        return None;
    }
    let claims = verify_token(&auth.token, &state.jwt_secret).ok()?;
    role_of(&state.pool, &claims.uid).await.ok()??;
    Some((claims.uid, claims.name))
}

async fn send(socket: &mut WebSocket, value: &Value) -> bool {
    socket.send(Message::Text(value.to_string().into())).await.is_ok()
}

async fn client_session(mut socket: WebSocket, state: AppState) {
    let Some((uid, user)) = authenticate(&mut socket, &state).await else {
        let _ = socket
            .send(Message::Close(Some(CloseFrame {
                code: CLOSE_UNAUTHORIZED,
                reason: "No autorizado".into(),
            })))
            .await;
        return;
    };
    tracing::info!(%user, "Cliente conectado al control del scheduler");

    let hub = &state.scheduler;
    let (initial, mut events) = hub.subscribe();
    let role = role_of(&state.pool, &uid).await.ok().flatten().unwrap_or(Role::Consultor);
    if !send(&mut socket, &json!({ "type": "auth", "ok": true, "role": role.as_str() })).await {
        return;
    }
    for message in initial {
        if !send(&mut socket, &message).await {
            return;
        }
    }

    loop {
        tokio::select! {
            incoming = socket.recv() => {
                let text = match incoming {
                    Some(Ok(Message::Text(text))) => text,
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    // Ping/pong los gestiona axum; el binario no se usa.
                    Some(Ok(_)) => continue,
                };
                let reply = match serde_json::from_str::<ClientRequest>(&text) {
                    Ok(request) => handle_request(&state, &uid, &user, request, &mut socket).await,
                    Err(e) => Some(json!({ "type": "reply", "id": null, "ok": false, "error": format!("Petición inválida: {e}") })),
                };
                if let Some(reply) = reply && !send(&mut socket, &reply).await {
                    break;
                }
            }
            event = events.recv() => match event {
                Ok(message) => if !send(&mut socket, &message).await { break },
                // Nos quedamos atrás: se reenvía el estado completo en vez de los mensajes perdidos.
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    for message in hub.current() {
                        if !send(&mut socket, &message).await {
                            return;
                        }
                    }
                }
                Err(broadcast::error::RecvError::Closed) => break,
            },
        }
    }
    tracing::info!(%user, "Cliente desconectado del control del scheduler");
}

/// Ejecuta la orden y devuelve la respuesta para el cliente (si procede).
async fn handle_request(state: &AppState, uid: &str, user: &str, request: ClientRequest, socket: &mut WebSocket) -> Option<Value> {
    let (log, to_scheduler) = match request.command {
        ClientCommand::GetStatus => {
            for message in state.scheduler.current() {
                if !send(socket, &message).await {
                    return None;
                }
            }
            return Some(json!({ "type": "reply", "id": request.id, "ok": true, "error": null }));
        }
        ClientCommand::Pause => ("pause".to_string(), json!({ "cmd": "pause" })),
        ClientCommand::Resume => ("resume".to_string(), json!({ "cmd": "resume" })),
        ClientCommand::Cancel { execution_id } => (
            format!("cancel {execution_id}"),
            json!({ "cmd": "cancel", "execution_id": execution_id }),
        ),
    };

    // Pausar, reanudar y cancelar exigen administrador. El rol se lee en cada orden, no solo
    // al conectar, para que quitárselo a alguien surta efecto con el WebSocket ya abierto.
    if role_of(&state.pool, uid).await.ok().flatten() < Some(Role::Administrador) {
        tracing::warn!(%user, "Orden al scheduler denegada por falta de permisos: {log}");
        return Some(json!({ "type": "reply", "id": request.id, "ok": false, "error": "Solo un administrador puede controlar el scheduler" }));
    }

    tracing::info!(%user, "Orden al scheduler: {log}");
    let (ok, error) = match state.scheduler.request(to_scheduler).await {
        Ok(reply) => (reply["ok"].as_bool().unwrap_or(false), reply["error"].as_str().map(str::to_string)),
        Err(e) => (false, Some(e)),
    };
    Some(json!({ "type": "reply", "id": request.id, "ok": ok, "error": error }))
}
