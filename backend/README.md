# calendar-backend

API REST en Rust (axum + sqlx/SQLite) para el frontend React de `react-mern-calendar-fin-seccion-29`.
Implementa login/registro con JWT, el CRUD de eventos de calendario que consume `src/api/calendarApi.js`
en el frontend, y un modelo SCADA (sites, assets, connections, tags, schedules) para supervisar y
programar comandos sobre PLCs/gateways vía OPC UA, MQTT o S7.

## Stack

- **axum** — router HTTP y extractores.
- **sqlx** (`sqlite`, bundled) — acceso a una base de datos SQLite embebida, sin dependencias del sistema.
- **jsonwebtoken** (backend `rust_crypto`, sin dependencias nativas) — firma/verificación de JWT HS256.
- **bcrypt** — hash de contraseñas.
- **tower-http** — CORS.

## Requisitos

- Rust estable (`cargo`/`rustc`).

## Configuración

```bash
cp .env.example .env
```

Variables de entorno (`.env`):

| Variable | Descripción | Default |
| --- | --- | --- |
| `DATABASE_URL` | Cadena de conexión SQLite. El archivo se crea solo si no existe. | `sqlite://calendar.db` |
| `JWT_SECRET` | Secreto para firmar los JWT. **Obligatorio**, cámbialo en producción. | — |
| `JWT_EXPIRES_SECONDS` | Vigencia del token en segundos. | `86400` (24h) |
| `PORT` | Puerto HTTP. | `4000` |
| `ALLOWED_ORIGIN` | Origen permitido por CORS (el del frontend en dev). | `http://localhost:5173` |

El frontend ya apunta a `http://localhost:4000/api` en `.env.template`/`.env.test`, así que con los
valores por defecto no hace falta tocar nada del lado de React.

## Ejecutar

```bash
cargo run
```

Al iniciar se aplican automáticamente las migraciones (`migrations/0001_init.sql`,
`migrations/0002_scada_model.sql` y `migrations/0003_events_from_schedules.sql`), creando
`calendar.db` con las tablas `users`, `events` y el modelo SCADA (`sites`, `assets`, `connections`,
`tags`, `schedules`, `interlocks`, `command_executions`) si no existen.

## Tests

```bash
cargo test      # tests unitarios (JWT, validación de eventos y del modelo SCADA)
cargo clippy    # lints
```

## Contrato de la API

Todas las rutas cuelgan de `/api`. Las respuestas de error siempre tienen la forma
`{ "ok": false, "msg": "..." }`, que es lo que el frontend lee en `error.response.data.msg`.

### Autenticación

| Método | Ruta | Body | Respuesta OK |
| --- | --- | --- | --- |
| POST | `/api/auth` | `{ email, password }` | `{ ok, uid, name, token }` |
| POST | `/api/auth/new` | `{ name, email, password }` | `{ ok, uid, name, token }` |
| GET | `/api/auth/renew` | — (requiere `x-token`) | `{ ok, uid, name, token }` |

El JWT se envía y se valida en el header **`x-token`** (no `Authorization: Bearer`), igual que
`calendarApi.js` en el frontend.

### Eventos (todas requieren `x-token`)

Un evento ya no es una entidad libre: es siempre la ocurrencia de un **schedule** SCADA en una
fecha concreta (ver más abajo). Se genera automáticamente al guardar un schedule, o a mano
eligiendo el schedule y la fecha desde el calendario.

| Método | Ruta | Body | Respuesta OK |
| --- | --- | --- | --- |
| GET | `/api/events` | — | `{ ok, eventos: [...] }` (todos los eventos, de todos los usuarios) |
| POST | `/api/events` | `{ schedule_id, start }` | `201 { ok, evento }` |
| PUT | `/api/events/{id}` | `{ schedule_id, start }` | `{ ok, evento }` (solo el dueño puede editar) |
| DELETE | `/api/events/{id}` | — | `{ ok: true }` (solo el dueño puede borrar) |

`start` es un string ISO-8601 (lo que produce `Date.toJSON()` en el frontend). `end` no se guarda:
se calcula al vuelo como `start + 30 minutos` solo para que `react-big-calendar` pueda dibujar el
evento como un bloque. `title` tampoco se guarda en el evento: siempre refleja el `name` del
schedule que lo generó (vía JOIN). Cada evento se serializa como:

```json
{
  "id": "uuid",
  "schedule_id": "uuid-del-schedule",
  "title": "...",
  "start": "2026-09-10T15:00:00.000Z",
  "end": "2026-09-10T15:30:00.000Z",
  "user": { "uid": "uuid-del-dueño", "name": "Nombre" }
}
```

`user` tiene el mismo shape `{ uid, name }` que `state.auth.user` en Redux, para que
`eventStyleGetter` en `CalendarPage.jsx` pueda comparar `user.uid` y pintar en otro color los
eventos que no son del usuario actual.

### Modelo SCADA

Jerarquía: un **site** (planta) contiene **assets** (planta → área → equipo → PLC, vía
`parent_asset_id` autorreferente); un asset expone una o más **connections** (un PLC/gateway
accesible por OPC UA, MQTT o S7); cada connection expone **tags** (los puntos SCADA: telemandos,
consignas o medidas); y un tag puede tener **schedules** que escriben un valor objetivo sobre él de
forma puntual (`once`) o recurrente (`cron`). Un schedule no referencia un evento del calendario:
es al revés, cada schedule genera sus propios eventos (ver `/api/events` más arriba).

Todas las rutas de este bloque requieren `x-token` y comparten el formato de error `{ ok: false, msg }`.
Las respuestas de lista devuelven `{ ok, <entidad-en-plural>: [...] }`; crear/actualizar devuelven
`{ ok, <entidad-en-singular>: {...} }` (`201` al crear); borrar devuelve `{ ok: true }`.

#### Sites

| Método | Ruta | Body | Notas |
| --- | --- | --- | --- |
| GET | `/api/sites` | — | Lista todos los sites. |
| POST | `/api/sites` | `{ name }` | |
| PUT | `/api/sites/{id}` | `{ name }` | |
| DELETE | `/api/sites/{id}` | — | Borra en cascada sus assets (y, transitivamente, connections/tags/schedules). |

#### Assets

| Método | Ruta | Body / Query | Notas |
| --- | --- | --- | --- |
| GET | `/api/assets` | Query opcional `site_id`, `parent_asset_id` | Filtra el árbol de assets. |
| POST | `/api/assets` | `{ site_id, parent_asset_id?, name, kind }` | `kind` es libre (`plant`, `area`, `equipment`, `plc`, ...). Si hay `parent_asset_id`, debe pertenecer al mismo `site_id`. |
| PUT | `/api/assets/{id}` | igual que POST | Un asset no puede ser su propio padre. |
| DELETE | `/api/assets/{id}` | — | Borra en cascada sus assets hijos, connections, tags y schedules. |

#### Connections

`config` depende de `protocol` (`opcua` \| `mqtt` \| `s7`) y se valida en el servidor:

- `opcua`: `{ endpoint_url, security_policy, credentials_ref }`
- `mqtt`: `{ broker_url, base_topic, qos, tls }`
- `s7`: `{ ip, rack, slot }`

| Método | Ruta | Body / Query | Notas |
| --- | --- | --- | --- |
| GET | `/api/connections` | Query opcional `asset_id` | |
| POST | `/api/connections` | `{ asset_id, name, protocol, config }` | `status` nace en `unknown`; lo actualiza el gateway SCADA, no este CRUD. |
| PUT | `/api/connections/{id}` | igual que POST | No permite cambiar `status`/`last_seen` desde aquí. |
| DELETE | `/api/connections/{id}` | — | Borra en cascada sus tags y schedules. |

#### Tags

`address` depende del `protocol` de la connection asociada y también se valida en el servidor:

- `opcua`: `{ node_id }`
- `mqtt`: `{ topic, json_pointer }`
- `s7`: `{ db_number, offset, bit, s7_type }`

| Método | Ruta | Body / Query | Notas |
| --- | --- | --- | --- |
| GET | `/api/tags` | Query opcional `connection_id`, `asset_id` | |
| POST | `/api/tags` | `{ connection_id, asset_id, name, description?, tag_kind, data_type, unit?, address, read_write?, min_value?, max_value?, allowed_values?, requires_sbo?, requires_ack? }` | `asset_id` debe coincidir con el `asset_id` de `connection_id`. `tag_kind`: `telemando` \| `consigna` \| `medida`. `data_type`: `bool` \| `int` \| `float` \| `enum` (requiere `allowed_values`). El nombre debe ser único dentro de la misma `connection_id`. |
| PUT | `/api/tags/{id}` | igual que POST | |
| DELETE | `/api/tags/{id}` | — | Borra en cascada sus schedules. |

#### Schedules

Generaliza el calendario: en vez de solo título/notas, una programación apunta a un `tag_id` y a un
`target_value` (debe respetar el `data_type` del tag, y estar en `allowed_values` si es `enum`).

| Método | Ruta | Body / Query | Notas |
| --- | --- | --- | --- |
| GET | `/api/schedules` | Query opcional `tag_id`, `enabled` (`true`/`false`) | |
| POST | `/api/schedules` | `{ name, tag_id, target_value, trigger_type, cron_expr?, start_date?, end_date?, enabled?, requires_confirmation? }` | `created_by` se toma del `x-token`. `trigger_type`: `once` (requiere `start_date`) o `cron` (requiere `cron_expr`). No referencia ningún evento; el frontend genera los eventos correspondientes llamando a `POST /api/events` con este `id` recién creado. |
| PUT | `/api/schedules/{id}` | igual que POST | `created_by` no cambia. |
| DELETE | `/api/schedules/{id}` | — | Borra en cascada (`ON DELETE CASCADE`) los eventos generados a partir de él. |

`interlocks` (enclavamientos) y `command_executions` (auditoría de comandos) ya tienen modelo y
migración (`src/scada/models.rs`, `migrations/0002_scada_model.sql`) pero todavía no tienen CRUD
expuesto en la API.

## Base de datos

SQLite (ver `migrations/0001_init.sql`, `migrations/0002_scada_model.sql` y
`migrations/0003_events_from_schedules.sql`):

- `users (id, name, email UNIQUE, password_hash, created_at)`
- `events (id, schedule_id -> schedules.id, start_date, user_id -> users.id, created_at)`
- `sites (id, name, created_at)`
- `assets (id, site_id -> sites.id, parent_asset_id -> assets.id, name, kind, created_at)`
- `connections (id, asset_id -> assets.id, name, protocol, config, status, last_seen, created_at)`
- `tags (id, connection_id -> connections.id, asset_id -> assets.id, name, description, tag_kind, data_type, unit, address, read_write, min_value, max_value, allowed_values, requires_sbo, requires_ack, created_at)`
- `schedules (id, name, tag_id -> tags.id, target_value, trigger_type, cron_expr, start_date, end_date, enabled, requires_confirmation, created_by -> users.id, created_at)`
- `interlocks (id, tag_id -> tags.id, condition_tag_id -> tags.id, operator, condition_value, action, created_at)`
- `command_executions (id, schedule_id -> schedules.id, tag_id -> tags.id, requested_value, requested_by -> users.id, status, sent_at, ack_at, response, error_message, latency_ms, created_at)`

Los IDs son UUID v4 en texto. El calendario es compartido: `GET /events` devuelve los eventos de
todos los usuarios, pero solo el dueño (`user_id`) puede editarlos o borrarlos. El modelo SCADA es
compartido entre todos los usuarios autenticados (sin verificación de propiedad), salvo
`schedules.created_by`, que registra quién la creó a efectos de auditoría.
