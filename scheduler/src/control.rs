//! Control del scheduler en caliente: pausar/reanudar, listar y cancelar ejecuciones en curso
//! y notificar los cambios. Se habla en JSON de una línea por mensaje (NDJSON), por un socket
//! Unix en producción (ver `socket`); `serve_connection` es genérico para poder probarlo
//! sin sockets.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::{broadcast, oneshot};

use crate::db::Task;

/// Mensajes del scheduler hacia el backend.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    /// Estado completo; se envía al conectar y tras cada cambio.
    Status { paused: bool, running: Vec<RunningInfo> },
    /// Cambio de estado de una ejecución: `started`, `ack`, `sent`, `failed`, `timeout`, `cancelled`.
    Execution {
        execution_id: String,
        event_id: String,
        schedule_name: String,
        tag_name: String,
        status: String,
        message: Option<String>,
    },
    /// Respuesta a una petición (mismo `id`).
    Reply { id: Option<String>, ok: bool, error: Option<String> },
}

#[derive(Debug, Clone, Serialize)]
pub struct RunningInfo {
    pub execution_id: String,
    pub event_id: String,
    pub schedule_name: String,
    pub tag_name: String,
    pub started_at: String,
}

/// Peticiones del backend al scheduler.
#[derive(Debug, Deserialize)]
struct Request {
    id: Option<String>,
    #[serde(flatten)]
    command: Command,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
enum Command {
    GetStatus,
    Pause,
    Resume,
    Cancel { execution_id: String },
}

struct Running {
    info: RunningInfo,
    cancel: oneshot::Sender<()>,
}

pub struct Control {
    paused: AtomicBool,
    running: Mutex<HashMap<String, Running>>,
    events: broadcast::Sender<Message>,
}

impl Default for Control {
    fn default() -> Self {
        Self::new()
    }
}

impl Control {
    pub fn new() -> Self {
        Self {
            paused: AtomicBool::new(false),
            running: Mutex::new(HashMap::new()),
            events: broadcast::channel(256).0,
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    /// Pausa o reanuda la recogida de tareas nuevas. Las ya en curso siguen hasta terminar.
    pub fn set_paused(&self, paused: bool) {
        if self.paused.swap(paused, Ordering::SeqCst) != paused {
            tracing::info!("Scheduler {}", if paused { "pausado" } else { "reanudado" });
            self.publish_status();
        }
    }

    pub fn status(&self) -> Message {
        let mut running: Vec<RunningInfo> = self.running.lock().unwrap().values().map(|r| r.info.clone()).collect();
        running.sort_by(|a, b| a.started_at.cmp(&b.started_at));
        Message::Status { paused: self.is_paused(), running }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Message> {
        self.events.subscribe()
    }

    fn publish(&self, message: Message) {
        // Sin suscriptores `send` falla: no pasa nada, nadie escucha.
        let _ = self.events.send(message);
    }

    fn publish_status(&self) {
        self.publish(self.status());
    }

    /// Registra una ejecución en curso. El receptor se activa si alguien la cancela.
    pub fn register(&self, execution_id: &str, task: &Task) -> oneshot::Receiver<()> {
        let (cancel, cancelled) = oneshot::channel();
        let info = RunningInfo {
            execution_id: execution_id.to_string(),
            event_id: task.event_id.clone(),
            schedule_name: task.schedule_name.clone(),
            tag_name: task.tag_name.clone(),
            started_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        };
        self.running
            .lock()
            .unwrap()
            .insert(execution_id.to_string(), Running { info, cancel });
        self.report(execution_id, task, "started", None);
        self.publish_status();
        cancelled
    }

    pub fn unregister(&self, execution_id: &str) {
        self.running.lock().unwrap().remove(execution_id);
        self.publish_status();
    }

    /// Pide cancelar una ejecución en curso. `false` si no existe (o ya terminó).
    pub fn cancel(&self, execution_id: &str) -> bool {
        match self.running.lock().unwrap().remove(execution_id) {
            Some(running) => running.cancel.send(()).is_ok(),
            None => false,
        }
    }

    /// Notifica un cambio de estado de la ejecución de `task`.
    pub fn report(&self, execution_id: &str, task: &Task, status: &str, message: Option<&str>) {
        self.publish(Message::Execution {
            execution_id: execution_id.to_string(),
            event_id: task.event_id.clone(),
            schedule_name: task.schedule_name.clone(),
            tag_name: task.tag_name.clone(),
            status: status.to_string(),
            message: message.map(str::to_string),
        });
    }

    fn handle(&self, request: Request) -> Message {
        let (ok, error) = match request.command {
            Command::GetStatus => return self.status(),
            Command::Pause => {
                self.set_paused(true);
                (true, None)
            }
            Command::Resume => {
                self.set_paused(false);
                (true, None)
            }
            Command::Cancel { execution_id } => {
                if self.cancel(&execution_id) {
                    (true, None)
                } else {
                    (false, Some("No hay ninguna ejecución en curso con ese id".to_string()))
                }
            }
        };
        Message::Reply { id: request.id, ok, error }
    }
}

async fn write_message<W: AsyncWrite + Unpin>(writer: &mut W, message: &Message) -> std::io::Result<()> {
    let mut line = serde_json::to_vec(message).expect("Message siempre serializa");
    line.push(b'\n');
    writer.write_all(&line).await
}

/// Atiende una conexión: envía el estado al conectar, responde a las peticiones y reenvía
/// los cambios de estado. Termina cuando el otro extremo cierra o falla una escritura.
pub async fn serve_connection<S>(stream: S, control: &Control)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let (reader, mut writer) = tokio::io::split(stream);
    let mut lines = BufReader::new(reader).lines();
    // Nos suscribimos antes de enviar el estado inicial para no perder cambios intermedios.
    let mut events = control.subscribe();

    if write_message(&mut writer, &control.status()).await.is_err() {
        return;
    }

    loop {
        let outgoing = tokio::select! {
            line = lines.next_line() => match line {
                Ok(Some(line)) if line.trim().is_empty() => continue,
                Ok(Some(line)) => match serde_json::from_str::<Request>(&line) {
                    Ok(request) => control.handle(request),
                    Err(e) => Message::Reply { id: None, ok: false, error: Some(format!("Petición inválida: {e}")) },
                },
                // EOF o error de lectura: el otro extremo se fue.
                Ok(None) | Err(_) => return,
            },
            event = events.recv() => match event {
                Ok(message) => message,
                // Nos quedamos atrás: lo importante es el estado actual.
                Err(broadcast::error::RecvError::Lagged(_)) => control.status(),
                Err(broadcast::error::RecvError::Closed) => return,
            },
        };
        if write_message(&mut writer, &outgoing).await.is_err() {
            return;
        }
    }
}
