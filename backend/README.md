# calendar-backend

API REST en Rust (axum + sqlx/SQLite) para el frontend React de `react-mern-calendar-fin-seccion-29`.
Implementa login/registro con JWT y el CRUD de eventos de calendario que consume `src/api/calendarApi.js`
en el frontend.

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

Al iniciar se aplican automáticamente las migraciones (`migrations/0001_init.sql`), creando
`calendar.db` con las tablas `users` y `events` si no existen.

## Tests

```bash
cargo test      # tests unitarios (JWT, validación de eventos)
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

| Método | Ruta | Body | Respuesta OK |
| --- | --- | --- | --- |
| GET | `/api/events` | — | `{ ok, eventos: [...] }` (todos los eventos, de todos los usuarios) |
| POST | `/api/events` | `{ title, notes?, start, end }` | `201 { ok, evento }` |
| PUT | `/api/events/{id}` | `{ title, notes?, start, end }` | `{ ok, evento }` (solo el dueño puede editar) |
| DELETE | `/api/events/{id}` | — | `{ ok: true }` (solo el dueño puede borrar) |

`start`/`end` son strings ISO-8601 (lo que produce `Date.toJSON()` en el frontend). Cada evento se
serializa como:

```json
{
  "id": "uuid",
  "title": "...",
  "notes": "...",
  "start": "2026-09-10T15:00:00.000Z",
  "end": "2026-09-10T17:00:00.000Z",
  "user": { "uid": "uuid-del-dueño", "name": "Nombre" }
}
```

`user` tiene el mismo shape `{ uid, name }` que `state.auth.user` en Redux, para que
`eventStyleGetter` en `CalendarPage.jsx` pueda comparar `user.uid` y pintar en otro color los
eventos que no son del usuario actual.

## Base de datos

SQLite con dos tablas (ver `migrations/0001_init.sql`):

- `users (id, name, email UNIQUE, password_hash, created_at)`
- `events (id, title, notes, start_date, end_date, user_id -> users.id, created_at)`

Los IDs son UUID v4 en texto. El calendario es compartido: `GET /events` devuelve los eventos de
todos los usuarios, pero solo el dueño (`user_id`) puede editarlos o borrarlos.
