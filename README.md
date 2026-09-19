# Calendar App

Aplicación de calendario colaborativo hecha con **React + Redux Toolkit**. Permite registrarse/iniciar
sesión y ver un calendario compartido (`react-big-calendar`) cuyos eventos siempre se generan a partir
de una **programación (schedule) SCADA**: una vez al guardar el schedule (una ocurrencia si es
`once`, varias si es `cron`, con la expresión crontab del propio schedule), o a mano desde el
calendario eligiendo el schedule y la fecha. También incluye una sección de **Mantenimiento SCADA**
para administrar sites, assets, connections, tags y schedules.

El backend que consume este frontend vive en [`backend/`](./backend) — una API REST en **Rust**
(axum + SQLite). Ver [`backend/README.md`](./backend/README.md) para su documentación completa.

## Stack

- **React 18** + **React Router 6**
- **Redux Toolkit** + **react-redux** para el estado global (auth, calendar, ui)
- **Vite** como bundler/dev server
- **react-big-calendar** para la vista de calendario y **react-datepicker** para los selectores de fecha
- **axios** para las llamadas HTTP a la API
- **cron-parser** para calcular las fechas de las ocurrencias que un schedule `cron` genera como events
- **sweetalert2** para notificaciones/errores
- **Jest** + **@testing-library/react** para pruebas

## Estructura de carpetas

```
├── backend/                    # API REST en Rust (axum + SQLite) — ver backend/README.md
│
├── src/
│   ├── main.jsx                 # Punto de entrada de la app (monta <CalendarApp />)
│   ├── CalendarApp.jsx           # Provider de Redux + AppRouter
│   │
│   ├── api/                      # Cliente HTTP
│   │   ├── calendarApi.js         # Instancia de axios (baseURL, interceptor x-token)
│   │   └── index.js
│   │
│   ├── auth/                     # Módulo de autenticación (UI)
│   │   └── pages/LoginPage.jsx    # Formulario de login + registro
│   │
│   ├── calendar/                 # Módulo de calendario (UI)
│   │   ├── components/
│   │   │   ├── CalendarModal.jsx    # Alta/edición de un evento: elegir schedule + fecha
│   │   │   ├── CalendarEvent.jsx    # Render de un evento dentro del calendario
│   │   │   ├── FabAddNew.jsx        # Botón flotante: nuevo evento
│   │   │   ├── FabDelete.jsx        # Botón flotante: eliminar evento activo
│   │   │   └── Navbar.jsx
│   │   └── pages/CalendarPage.jsx  # Página principal, arma el <Calendar /> y los modales/FABs
│   │
│   ├── scada/                    # Módulo de mantenimiento SCADA (UI)
│   │   ├── components/
│   │   │   └── EntityMaintenance.jsx # Tabla + modal de alta/edición genéricos, reusados por las 5 páginas
│   │   └── pages/
│   │       ├── ScadaPage.jsx          # Navbar + tabs para elegir la entidad a mantener
│   │       ├── SitesMaintenancePage.jsx
│   │       ├── AssetsMaintenancePage.jsx
│   │       ├── ConnectionsMaintenancePage.jsx
│   │       ├── TagsMaintenancePage.jsx
│   │       └── SchedulesMaintenancePage.jsx
│   │
│   ├── router/                   # React Router (rutas públicas/privadas según auth)
│   │
│   ├── store/                    # Redux Toolkit
│   │   ├── auth/authSlice.js       # status, user, errorMessage
│   │   ├── calendar/calendarSlice.js # events, activeEvent, isLoadingEvents
│   │   ├── ui/uiSlice.js           # isDateModalOpen
│   │   └── store.js
│   │
│   ├── hooks/                    # Hooks que encapsulan dispatch + llamadas a la API
│   │   ├── useAuthStore.js         # startLogin, startRegister, checkAuthToken, startLogout
│   │   ├── useCalendarStore.js     # startSavingEvent, startDeletingEvent, startLoadingEvents...
│   │   ├── useUiStore.js           # abrir/cerrar el modal de evento
│   │   └── useForm.js
│   │
│   └── helpers/                  # Funciones puras reutilizables
│       ├── generateCronDates.js    # Traduce una expresión crontab + fecha inicio en N fechas
│       ├── convertEventsToDateEvents.js
│       ├── calendarLocalizer.js
│       ├── getMessages.js
│       └── getEnvVariables.js
│
└── tests/                        # Espejo de la estructura de src/, con Jest + Testing Library
```

## Módulos principales

- **`store/auth` + `hooks/useAuthStore`**: maneja login, registro y renovación de token JWT contra
  `POST /auth`, `POST /auth/new` y `GET /auth/renew`. El token se guarda en `localStorage` y se envía
  en cada petición mediante el header `x-token` (interceptor en `api/calendarApi.js`).
- **`store/calendar` + `hooks/useCalendarStore`**: carga (`GET /events`) y elimina
  (`DELETE /events/:id`) eventos; `startSavingEvent` crea/edita (`POST`/`PUT /events`) un evento
  como `{ schedule_id, start }` — un evento siempre es la ocurrencia de un schedule SCADA, nunca un
  título/notas libres.
- **`CalendarModal`**: alta/edición de un evento eligiendo un schedule existente (`GET /schedules`)
  y una fecha; no pide título ni fecha de fin (se calculan en el backend a partir del schedule).
- **`store/ui` + `hooks/useUiStore`**: controla la visibilidad del modal de alta/edición de eventos.
- **`scada/` → `SchedulesMaintenancePage` + `helpers/generateCronDates`**: al crear un schedule se
  generan automáticamente sus events en el calendario — uno solo si el disparador es `once` (en su
  `start_date`), o varias ocurrencias si es `cron` (usando `cron-parser` sobre su `cron_expr` para
  calcular N fechas, con `POST /events` por cada una).
- **`scada/`**: sección de mantenimiento (listar/crear/editar/borrar) para las entidades SCADA del
  backend — `sites`, `assets`, `connections`, `tags` y `schedules`. Se accede con el botón
  **Mantenimiento** del `Navbar` (ruta `/scada`), que alterna a **Calendario** para volver. `ScadaPage`
  muestra una barra de tabs y renderiza la página de la entidad activa; cada página sólo define sus
  columnas de tabla y campos de formulario (incluyendo los `select` de claves foráneas, p. ej. el
  `site_id` de un asset o el `connection_id` de un tag) y delega el listado, el modal y las llamadas
  `GET`/`POST`/`PUT`/`DELETE` a `components/EntityMaintenance.jsx`, que además expone un callback
  `onSaved` para que una página reaccione al item recién creado/editado (usado por
  `SchedulesMaintenancePage` para generar los events). Los campos `config`/`address`/
  `target_value`/`allowed_values` se editan como JSON en un textarea y se valida su sintaxis antes de
  enviarlos; el `asset_id` de un tag se toma automáticamente del `connection_id` elegido, igual que
  exige el backend. Ver [`backend/README.md`](./backend/README.md#modelo-scada) para el contrato de
  cada endpoint.

## Desarrollo

1. Copiar el archivo de variables de entorno:

   ```bash
   cp .env.template .env
   ```

2. Ajustar `VITE_API_URL` si el backend no corre en el puerto por defecto:

   ```
   VITE_API_URL=http://localhost:4000/api
   ```

3. Instalar dependencias y levantar el backend (ver [`backend/README.md`](./backend/README.md)):

   ```bash
   cd backend
   cp .env.example .env
   cargo run
   ```

4. Instalar dependencias y levantar el frontend:

   ```bash
   yarn install
   yarn dev
   ```

## Docker

El `Dockerfile` de la raíz genera **una única imagen** con frontend y backend:

- El frontend se compila con Vite (`VITE_API_URL=/api`) y lo sirve **nginx**, que además hace de proxy
  de `/api/` hacia el backend, así que navegador y API comparten origen (sin CORS).
- El backend **no se compila dentro de Docker**: la imagen (`debian:bookworm-slim`) copia el
  ejecutable Linux `backend/target/release/calendar-backend`, que hay que generar antes.

### 1. Compilar el backend para Linux

Desde Linux o WSL (en Windows, un `cargo build` normal genera un `.exe` que no sirve):

```bash
cd backend
cargo build --release
```

El binario queda en `backend/target/release/calendar-backend`. Si se recompila el backend hay que
volver a construir la imagen, porque el binario se copia tal cual.

### 2. Construir y arrancar

```bash
docker compose up --build
```

La aplicación queda en http://localhost:8080. También se puede usar Docker directamente:

```bash
docker build -t scheduler .
docker run -d --name scheduler -p 8080:80 -v scheduler-data:/data scheduler
```

### Configuración

| Variable | Por defecto | Descripción |
| --- | --- | --- |
| `JWT_SECRET` | generado | Secreto para firmar los JWT. Si no se define, se genera uno y se guarda en `/data/jwt_secret` |
| `JWT_EXPIRES_SECONDS` | `86400` | Duración del token |
| `ALLOWED_ORIGIN` | `http://localhost:8080` | Origen permitido por CORS (no afecta al acceso a través de nginx) |
| `DATABASE_URL` | `sqlite:///data/calendar.db` | Ubicación de la base SQLite |

Los datos (base SQLite y secreto JWT) viven en el volumen `/data`, así que sobreviven a reinicios y
a reconstrucciones de la imagen. Para empezar de cero se elimina el volumen con
`docker compose down -v`.

### Base de datos semilla (datos de prueba)

Opcionalmente la imagen puede llevar una base SQLite ya cargada con datos de prueba, útil para
distribuirla a un equipo sin acceso a internet:

- Si al construir la imagen existe `docker/seed/calendar.db`, el `Dockerfile` la copia a `/seed`.
- Al arrancar, `docker/entrypoint.sh` la copia a `/data/calendar.db` **solo si el volumen aún no tiene
  base de datos**. Nunca pisa datos existentes; para volver a la semilla hay que eliminar el volumen
  (`docker compose down -v`).
- Si el fichero no existe, la imagen se construye igualmente y arranca con una base **vacía**.
  `docker/seed/calendar.db` está en `.gitignore`, por lo que un clon limpio del repositorio siempre
  genera una imagen sin datos.

Para generar la base semilla:

1. Arrancar un contenedor con un volumen vacío y registrar un usuario real (el script del paso 3 crea
   uno sin contraseña válida si no encuentra ninguno, y con él no se podría iniciar sesión):

   ```bash
   # la base debe partir vacía: construir la imagen sin docker/seed/calendar.db
   docker run -d --name seedgen -p 8082:80 scheduler:latest
   curl -X POST localhost:8082/api/auth/new -H 'Content-Type: application/json' \
     -d '{"name":"Demo SCADA","email":"demo.scada@example.com","password":"Demo1234"}'
   docker stop seedgen
   ```

2. Copiar la base fuera del contenedor:

   ```bash
   mkdir -p seed-tmp && docker cp seedgen:/data/. seed-tmp && docker rm -fv seedgen
   ```

3. Cargar los datos de prueba (una red de distribución de agua inventada: sites, assets, connections,
   tags, schedules, interlocks y events) y dejar la base en un único fichero, sin WAL:

   ```bash
   python backend/scripts/seed_water_network.py seed-tmp/calendar.db
   python -c "import sqlite3; sqlite3.connect('seed-tmp/calendar.db').execute('PRAGMA journal_mode=DELETE')"
   cp seed-tmp/calendar.db docker/seed/calendar.db && rm -r seed-tmp
   ```

4. Reconstruir la imagen con `docker compose down -v && docker compose up -d --build`. Se puede
   entrar con el usuario y contraseña del paso 1.

Notas: el script fija las fechas de los events **relativas al día en que se ejecuta** (entre hoy y
+10 días), por lo que la semilla envejece; conviene regenerarla antes de llevarla a otro equipo. La
cuenta de la semilla tiene una contraseña conocida: cámbiala si el contenedor va a ser accesible
desde una red.

## Scripts disponibles

| Comando | Descripción |
| --- | --- |
| `yarn dev` | Levanta el servidor de desarrollo de Vite |
| `yarn build` | Genera el build de producción en `dist/` |
| `yarn preview` | Sirve el build de producción localmente |
| `yarn test` | Corre la suite de Jest en modo watch |
