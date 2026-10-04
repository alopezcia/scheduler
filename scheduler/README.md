# scheduler

App Rust independiente que **ejecuta las tareas programadas del calendario**: lee del SQLite del
backend los eventos vencidos y, para cada uno, llama a la **API REST del sistema SCADA**
(Ignition, WinCC, AVEVA…) para escribir el valor objetivo del schedule sobre el tag. Es un cliente
saliente: no expone ningún endpoint. Por ahora solo soporta conexiones con `protocol = 'rest'`.

## Cómo funciona

Cada `POLL_INTERVAL_SECONDS` busca eventos que cumplan **todo** esto:

- el schedule está `enabled` y no requiere confirmación manual (`requires_confirmation = 0`);
- el tag del schedule pertenece a una conexión `rest` (las `opcua`/`mqtt`/`s7` se ignoran);
- el evento ya venció (`start_date <= ahora`) y no tiene ninguna fila en `command_executions`.

Para cada uno inserta primero una fila en `command_executions` (índice único por `event_id`, así
un evento se ejecuta **como mucho una vez**, aunque se reinicie o haya dos instancias) y después:

1. Si venció hace más de `MAX_LATE_SECONDS`, **no se envía** y queda como `timeout` (no se
   ejecutan comandos antiguos tras una parada larga).
2. Escribe: `method` (`PUT`/`POST`/`PATCH`) sobre `base_url + path`, con un cuerpo JSON que
   coloca el `target_value` en `json_pointer` (`/value` → `{"value": 7}`; `""` → el valor solo).
3. Si el tag tiene `requires_ack`, relee con `GET` sobre el mismo `path` y compara el valor en
   `json_pointer` con el pedido: coincide → `ack`; si no → `failed` (con `sent_at` rellenado).

Estados resultantes en `command_executions.status`: `sent` (escrito, sin confirmación pedida),
`ack`, `failed` (error, rechazo HTTP no 2xx, no confirmado) y `timeout` (evento perdido o
timeout HTTP). `response`, `error_message` y `latency_ms` quedan guardados como auditoría.

### Lo que **no** envía (falla en vez de ignorar la seguridad)

- Tags con **enclavamientos** (`interlocks`): todavía no se evalúan, así que el comando se
  rechaza (`failed`, sin enviar).
- Tags con **`requires_sbo`**: select-before-operate no está soportado en el cliente REST.
- Tags de solo lectura (`read_write = 'read'`), o `config`/`address`/`target_value` inválidos.

## Credenciales

`config.credentials_ref` es solo una referencia. El secreto se lee de la variable de entorno
`SCHEDULER_CRED_<REF>` (la ref en mayúsculas, con todo lo no alfanumérico como `_`; p. ej.
`ignition-prod` → `SCHEDULER_CRED_IGNITION_PROD`), según `config.auth_type`:

| `auth_type` | Valor de la variable |
| --- | --- |
| `none` | no hace falta |
| `bearer` | el token |
| `basic` | `usuario:contraseña` |
| `api_key` | la clave; se envía en `config.api_key_header` (por defecto `X-API-Key`) |

## Configuración y ejecución

```bash
cp .env.example .env
cargo run
```

| Variable | Descripción | Default |
| --- | --- | --- |
| `DATABASE_URL` | SQLite del backend. No se crea ni se migra desde aquí. | `sqlite://calendar.db` |
| `POLL_INTERVAL_SECONDS` | Intervalo de sondeo. | `5` |
| `MAX_LATE_SECONDS` | Tolerancia para ejecutar un evento ya vencido. | `300` |
| `RUST_LOG` | Nivel de log. | `info` |

El **backend debe haber arrancado antes** al menos una vez: es quien crea la base de datos y
aplica las migraciones (la `0005` añade `command_executions.event_id`, que esta app necesita).
Si el esquema no es el esperado, el scheduler aborta con un mensaje claro.

## Tests

```bash
cargo test    # unitarios + e2e con el esquema real del backend y un SCADA REST falso (axum)
cargo clippy --all-targets
```
