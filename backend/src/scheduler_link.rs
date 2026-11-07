//! Enlace con la app `scheduler` por un socket Unix (JSON de una línea por mensaje, en los
//! dos sentidos). El backend es el único cliente del socket: reenvía a los navegadores
//! (ver `scheduler_ws`) lo que el scheduler notifica y traduce a peticiones lo que ellos piden.
//!
//! `run_session` es genérico sobre el flujo para poder probarlo sin sockets; solo la conexión
//! (`connect_loop`) depende de Unix.

// En plataformas sin sockets Unix (desarrollo en Windows) solo existe el enlace caído.
#![cfg_attr(not(unix), allow(dead_code))]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::{broadcast, mpsc, oneshot};
use uuid::Uuid;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const RECONNECT_DELAY: Duration = Duration::from_secs(2);

#[derive(Default)]
struct Link {
    /// Canal hacia la sesión activa; `None` si el scheduler no está conectado.
    outbox: Option<mpsc::Sender<String>>,
    /// Último `status` recibido, para dárselo a los navegadores que se conectan después.
    status: Option<Value>,
}

struct Inner {
    link: Mutex<Link>,
    pending: Mutex<HashMap<String, oneshot::Sender<Value>>>,
    events: broadcast::Sender<Value>,
}

/// Punto de acceso compartido (clonable) al scheduler.
#[derive(Clone)]
pub struct SchedulerHub {
    inner: Arc<Inner>,
}

impl Default for SchedulerHub {
    fn default() -> Self {
        Self::new()
    }
}

impl SchedulerHub {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                link: Mutex::new(Link::default()),
                pending: Mutex::new(HashMap::new()),
                events: broadcast::channel(256).0,
            }),
        }
    }

    /// Mensajes con el estado actual del enlace, para quien se acaba de conectar.
    fn snapshot(link: &Link) -> Vec<Value> {
        let mut messages = vec![json!({ "type": "link", "connected": link.outbox.is_some() })];
        messages.extend(link.status.clone());
        messages
    }

    /// Estado actual del enlace y suscripción a los cambios posteriores (sin huecos entre ambos).
    pub fn subscribe(&self) -> (Vec<Value>, broadcast::Receiver<Value>) {
        let link = self.inner.link.lock().unwrap();
        (Self::snapshot(&link), self.inner.events.subscribe())
    }

    pub fn current(&self) -> Vec<Value> {
        Self::snapshot(&self.inner.link.lock().unwrap())
    }

    /// Envía una petición al scheduler (`cmd` y sus parámetros) y espera su respuesta.
    pub async fn request(&self, mut request: Value) -> Result<Value, String> {
        let outbox = self
            .inner
            .link
            .lock()
            .unwrap()
            .outbox
            .clone()
            .ok_or_else(|| "El scheduler no está conectado".to_string())?;

        let id = Uuid::new_v4().to_string();
        request["id"] = json!(id);
        let (reply_tx, reply_rx) = oneshot::channel();
        self.inner.pending.lock().unwrap().insert(id.clone(), reply_tx);

        let mut line = request.to_string();
        line.push('\n');
        if outbox.send(line).await.is_err() {
            self.inner.pending.lock().unwrap().remove(&id);
            return Err("El scheduler no está conectado".to_string());
        }

        match tokio::time::timeout(REQUEST_TIMEOUT, reply_rx).await {
            Ok(Ok(reply)) => Ok(reply),
            Ok(Err(_)) => Err("Se perdió la conexión con el scheduler".to_string()),
            Err(_) => {
                self.inner.pending.lock().unwrap().remove(&id);
                Err("El scheduler no respondió a tiempo".to_string())
            }
        }
    }

    fn connected(&self, outbox: mpsc::Sender<String>) {
        let mut link = self.inner.link.lock().unwrap();
        link.outbox = Some(outbox);
        // Se emite con el candado tomado para que `subscribe` no vea un estado a medias.
        let _ = self.inner.events.send(json!({ "type": "link", "connected": true }));
    }

    fn disconnected(&self) {
        {
            let mut link = self.inner.link.lock().unwrap();
            link.outbox = None;
            link.status = None;
            let _ = self.inner.events.send(json!({ "type": "link", "connected": false }));
        }
        // Soltar los emisores hace fallar las peticiones en vuelo.
        self.inner.pending.lock().unwrap().clear();
    }

    fn on_scheduler_message(&self, line: &str) {
        let Ok(message) = serde_json::from_str::<Value>(line) else {
            tracing::warn!("Mensaje del scheduler no válido: {line}");
            return;
        };
        match message["type"].as_str() {
            Some("reply") => {
                let waiter = message["id"]
                    .as_str()
                    .and_then(|id| self.inner.pending.lock().unwrap().remove(id));
                if let Some(waiter) = waiter {
                    let _ = waiter.send(message);
                }
            }
            Some("status") => {
                let mut link = self.inner.link.lock().unwrap();
                link.status = Some(message.clone());
                let _ = self.inner.events.send(message);
            }
            _ => {
                let _ = self.inner.events.send(message);
            }
        }
    }
}

/// Atiende una conexión con el scheduler hasta que se cierra.
pub async fn run_session<S>(stream: S, hub: &SchedulerHub)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let (reader, mut writer) = tokio::io::split(stream);
    let mut lines = BufReader::new(reader).lines();
    let (outbox, mut inbox) = mpsc::channel::<String>(32);
    hub.connected(outbox);

    loop {
        tokio::select! {
            line = lines.next_line() => match line {
                Ok(Some(line)) => hub.on_scheduler_message(&line),
                Ok(None) | Err(_) => break,
            },
            Some(line) = inbox.recv() => {
                if writer.write_all(line.as_bytes()).await.is_err() {
                    break;
                }
            }
        }
    }

    hub.disconnected();
}

/// Mantiene la conexión con el socket del scheduler, reintentando si no está o se cae.
#[cfg(unix)]
pub async fn connect_loop(path: String, hub: SchedulerHub) {
    let mut reported_down = false;
    loop {
        match tokio::net::UnixStream::connect(&path).await {
            Ok(stream) => {
                tracing::info!("Conectado al scheduler ({path})");
                reported_down = false;
                run_session(stream, &hub).await;
                tracing::warn!("Conexión con el scheduler cerrada");
            }
            Err(e) => {
                // Solo se avisa una vez por caída para no llenar el log mientras reintenta.
                if !reported_down {
                    tracing::warn!("No se puede conectar al scheduler ({path}): {e}; se reintentará");
                    reported_down = true;
                }
            }
        }
        tokio::time::sleep(RECONNECT_DELAY).await;
    }
}

#[cfg(not(unix))]
pub async fn connect_loop(_path: String, _hub: SchedulerHub) {
    tracing::warn!("Los sockets Unix no están disponibles en esta plataforma: el control del scheduler queda desactivado");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Extremo "scheduler": lee una petición y contesta con la respuesta dada.
    async fn fake_scheduler(stream: tokio::io::DuplexStream, reply_ok: bool) {
        let (reader, mut writer) = tokio::io::split(stream);
        let mut lines = BufReader::new(reader).lines();
        writer.write_all(b"{\"type\":\"status\",\"paused\":false,\"running\":[]}\n").await.unwrap();
        while let Some(line) = lines.next_line().await.unwrap() {
            let request: Value = serde_json::from_str(&line).unwrap();
            let reply = json!({ "type": "reply", "id": request["id"], "ok": reply_ok, "error": null });
            writer.write_all(format!("{reply}\n").as_bytes()).await.unwrap();
        }
    }

    #[tokio::test]
    async fn forwards_status_and_correlates_replies() {
        let hub = SchedulerHub::new();
        let (backend_side, scheduler_side) = tokio::io::duplex(4096);
        tokio::spawn(fake_scheduler(scheduler_side, true));
        let session_hub = hub.clone();
        tokio::spawn(async move { run_session(backend_side, &session_hub).await });

        // Sin conexión todavía el enlace figura como caído; al conectar llega el estado.
        let (_, mut events) = hub.subscribe();
        loop {
            let event = events.recv().await.unwrap();
            if event["type"] == "status" {
                assert_eq!(event["paused"], json!(false));
                break;
            }
        }
        assert_eq!(hub.current().len(), 2);

        let reply = hub.request(json!({ "cmd": "pause" })).await.unwrap();
        assert_eq!(reply["ok"], json!(true));
    }

    #[tokio::test]
    async fn requests_fail_while_the_scheduler_is_down() {
        let hub = SchedulerHub::new();
        assert!(hub.request(json!({ "cmd": "pause" })).await.is_err());
        assert_eq!(hub.current(), vec![json!({ "type": "link", "connected": false })]);
    }

    #[tokio::test]
    async fn disconnect_clears_status_and_fails_inflight_requests() {
        let hub = SchedulerHub::new();
        let (backend_side, scheduler_side) = tokio::io::duplex(4096);
        let session_hub = hub.clone();
        let session = tokio::spawn(async move { run_session(backend_side, &session_hub).await });
        // Esperamos a que la sesión esté registrada.
        while hub.current()[0]["connected"] != json!(true) {
            tokio::task::yield_now().await;
        }

        let requester = {
            let hub = hub.clone();
            tokio::spawn(async move { hub.request(json!({ "cmd": "pause" })).await })
        };
        // El "scheduler" cierra sin contestar.
        tokio::time::sleep(Duration::from_millis(50)).await;
        drop(scheduler_side);
        session.await.unwrap();

        assert!(requester.await.unwrap().is_err());
        assert_eq!(hub.current(), vec![json!({ "type": "link", "connected": false })]);
    }
}
