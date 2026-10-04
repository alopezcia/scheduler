//! Ejecuta las tareas vencidas: cada evento produce como mucho un comando REST y deja
//! siempre una fila de auditoría en `command_executions`.

use std::time::Instant;

use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use sqlx::SqlitePool;

use crate::db::{self, Outcome, Task};
use crate::rest::{RestAddress, RestClient, RestConfig, RestError, values_match};

const MAX_RESPONSE_LEN: usize = 4096;

/// Busca las tareas vencidas y las ejecuta. Devuelve cuántas tareas procesó.
pub async fn run_due_tasks(
    pool: &SqlitePool,
    client: &RestClient,
    now: DateTime<Utc>,
    max_late: Duration,
) -> Result<usize, sqlx::Error> {
    let mut processed = 0;

    for task in db::pending_tasks(pool).await? {
        let start = match DateTime::parse_from_rfc3339(&task.start_date) {
            Ok(start) => start.with_timezone(&Utc),
            Err(e) => {
                tracing::warn!(event_id = %task.event_id, "start_date inválido ({}): {e}", task.start_date);
                continue;
            }
        };
        if start > now {
            continue;
        }

        if now - start > max_late {
            let msg = format!("Evento perdido: venció hace más de {} s y no se envió", max_late.num_seconds());
            if db::claim(pool, &task, "timeout", Some(&msg)).await?.is_some() {
                tracing::warn!(event_id = %task.event_id, schedule = %task.schedule_name, "{msg}");
                processed += 1;
            }
            continue;
        }

        let Some(execution_id) = db::claim(pool, &task, "pending", None).await? else {
            continue;
        };
        processed += 1;

        let started = Instant::now();
        let result = execute(client, &task).await;
        let latency_ms = Some(started.elapsed().as_millis() as i64);

        let failure_message = result.as_ref().err().map(Failure::message).unwrap_or_default();
        let outcome = match &result {
            Ok(Executed { response, acked }) => Outcome {
                status: if *acked { "ack" } else { "sent" },
                sent: true,
                acked: *acked,
                response: Some(response),
                error_message: None,
                latency_ms,
            },
            Err(failure) => Outcome {
                status: failure.status(),
                sent: failure.was_sent(),
                acked: false,
                response: failure.response(),
                error_message: Some(&failure_message),
                latency_ms,
            },
        };
        db::finish(pool, &execution_id, &outcome).await?;

        match &result {
            Ok(_) => tracing::info!(
                event_id = %task.event_id, schedule = %task.schedule_name, tag = %task.tag_name,
                status = outcome.status, "Comando ejecutado"
            ),
            Err(_) => tracing::error!(
                event_id = %task.event_id, schedule = %task.schedule_name, tag = %task.tag_name,
                "Comando fallido: {failure_message}"
            ),
        }
    }

    Ok(processed)
}

struct Executed {
    response: String,
    acked: bool,
}

enum Failure {
    /// No se envió nada.
    NotSent(String),
    /// Error al enviar (timeout/transporte/HTTP): no hay garantía de que el SCADA no lo aplicara.
    Rest(RestError),
    /// El SCADA aceptó la escritura pero no se pudo confirmar el valor.
    NotAcked { response: String, message: String },
}

impl Failure {
    fn status(&self) -> &'static str {
        match self {
            Failure::Rest(RestError::Timeout(_)) => "timeout",
            _ => "failed",
        }
    }

    fn was_sent(&self) -> bool {
        matches!(self, Failure::NotAcked { .. })
    }

    fn response(&self) -> Option<&str> {
        match self {
            Failure::NotAcked { response, .. } => Some(response),
            Failure::Rest(RestError::Status(_, body)) => Some(truncate(body)),
            _ => None,
        }
    }

    fn message(&self) -> String {
        match self {
            Failure::NotSent(msg) | Failure::NotAcked { message: msg, .. } => msg.clone(),
            Failure::Rest(RestError::Invalid(msg) | RestError::Timeout(msg) | RestError::Transport(msg)) => msg.clone(),
            Failure::Rest(RestError::Status(code, _)) => format!("El SCADA rechazó el comando con HTTP {code} (ver response)"),
        }
    }
}

fn truncate(text: &str) -> &str {
    let mut end = text.len().min(MAX_RESPONSE_LEN);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// Salvaguardas que se comprueban antes de enviar: lo que este cliente todavía no sabe
/// hacer (enclavamientos, SBO) se rechaza en vez de ignorarse, para no saltarse una seguridad.
fn check_preconditions(task: &Task) -> Result<(), Failure> {
    if task.read_write == "read" {
        return Err(Failure::NotSent("El tag es de solo lectura".to_string()));
    }
    if task.has_interlocks {
        return Err(Failure::NotSent(
            "El tag tiene enclavamientos y el scheduler aún no los evalúa: comando no enviado".to_string(),
        ));
    }
    if task.requires_sbo {
        return Err(Failure::NotSent(
            "El tag requiere select-before-operate, no soportado por el cliente REST".to_string(),
        ));
    }
    Ok(())
}

async fn execute(client: &RestClient, task: &Task) -> Result<Executed, Failure> {
    check_preconditions(task)?;

    let cfg: RestConfig = serde_json::from_str(&task.connection_config)
        .map_err(|e| Failure::NotSent(format!("config de la conexión inválida: {e}")))?;
    let addr: RestAddress = serde_json::from_str(&task.address)
        .map_err(|e| Failure::NotSent(format!("address del tag inválida: {e}")))?;
    let value: Value = serde_json::from_str(&task.target_value)
        .map_err(|e| Failure::NotSent(format!("target_value no es JSON válido: {e}")))?;

    let response = client.write(&cfg, &addr, value.clone()).await.map_err(Failure::Rest)?;
    let response = truncate(&response).to_string();

    if !task.requires_ack {
        return Ok(Executed { response, acked: false });
    }

    match client.read(&cfg, &addr).await {
        Ok(actual) if values_match(&value, &actual) => Ok(Executed { response, acked: true }),
        Ok(actual) => Err(Failure::NotAcked {
            response,
            message: format!("Confirmación fallida: se esperaba {value} y el SCADA devuelve {actual}"),
        }),
        Err(e) => Err(Failure::NotAcked {
            response,
            message: format!("No se pudo confirmar el valor: {e}"),
        }),
    }
}
