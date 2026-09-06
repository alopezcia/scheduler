# Calendar App

Aplicación de calendario colaborativo hecha con **React + Redux Toolkit**. Permite registrarse/iniciar
sesión, crear, editar y eliminar eventos en un calendario compartido (`react-big-calendar`), y generar
varios eventos de una sola vez a partir de una expresión **crontab**.

El backend que consume este frontend vive en [`backend/`](./backend) — una API REST en **Rust**
(axum + SQLite). Ver [`backend/README.md`](./backend/README.md) para su documentación completa.

## Stack

- **React 18** + **React Router 6**
- **Redux Toolkit** + **react-redux** para el estado global (auth, calendar, ui)
- **Vite** como bundler/dev server
- **react-big-calendar** para la vista de calendario y **react-datepicker** para los selectores de fecha
- **axios** para las llamadas HTTP a la API
- **cron-parser** para interpretar expresiones crontab en el alta masiva de eventos
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
│   │   │   ├── CalendarModal.jsx    # Alta/edición de un evento individual
│   │   │   ├── CalendarCronModal.jsx# Alta masiva de eventos a partir de una expresión crontab
│   │   │   ├── CalendarEvent.jsx    # Render de un evento dentro del calendario
│   │   │   ├── FabAddNew.jsx        # Botón flotante: nuevo evento
│   │   │   ├── FabAddCron.jsx       # Botón flotante: alta masiva vía crontab
│   │   │   ├── FabDelete.jsx        # Botón flotante: eliminar evento activo
│   │   │   └── Navbar.jsx
│   │   └── pages/CalendarPage.jsx  # Página principal, arma el <Calendar /> y los modales/FABs
│   │
│   ├── router/                   # React Router (rutas públicas/privadas según auth)
│   │
│   ├── store/                    # Redux Toolkit
│   │   ├── auth/authSlice.js       # status, user, errorMessage
│   │   ├── calendar/calendarSlice.js # events, activeEvent, isLoadingEvents
│   │   ├── ui/uiSlice.js           # isDateModalOpen, isCronModalOpen
│   │   └── store.js
│   │
│   ├── hooks/                    # Hooks que encapsulan dispatch + llamadas a la API
│   │   ├── useAuthStore.js         # startLogin, startRegister, checkAuthToken, startLogout
│   │   ├── useCalendarStore.js     # startSavingEvent, startSavingManyEvents, startDeletingEvent...
│   │   ├── useUiStore.js           # abrir/cerrar los modales
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
- **`store/calendar` + `hooks/useCalendarStore`**: carga (`GET /events`), crea/edita
  (`POST`/`PUT /events`) y elimina (`DELETE /events/:id`) eventos individuales, además de
  `startSavingManyEvents` para el alta masiva.
- **`CalendarCronModal` + `helpers/generateCronDates`**: a partir de una expresión crontab estándar
  (ej. `0 9 * * 1-5`), una fecha de referencia y un número de repeticiones, genera N fechas de inicio
  (con `cron-parser`) y crea un evento por cada una, reutilizando `startSavingManyEvents`.
- **`store/ui` + `hooks/useUiStore`**: controla la visibilidad de los dos modales de alta de eventos
  (individual y por crontab).

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

## Scripts disponibles

| Comando | Descripción |
| --- | --- |
| `yarn dev` | Levanta el servidor de desarrollo de Vite |
| `yarn build` | Genera el build de producción en `dist/` |
| `yarn preview` | Sirve el build de producción localmente |
| `yarn test` | Corre la suite de Jest en modo watch |
