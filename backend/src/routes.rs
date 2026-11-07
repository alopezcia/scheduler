use axum::Router;
use axum::http::{HeaderName, HeaderValue, Method, header};
use axum::routing::{get, post, put};
use tower_http::cors::CorsLayer;

use crate::auth;
use crate::events;
use crate::scada;
use crate::scheduler_ws;
use crate::state::AppState;
use crate::users;

fn cors_layer(allowed_origin: &str) -> CorsLayer {
    let origin: HeaderValue = allowed_origin
        .parse()
        .expect("ALLOWED_ORIGIN no es una URL válida");

    CorsLayer::new()
        .allow_origin(origin)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE, HeaderName::from_static("x-token")])
}

pub fn build_router(state: AppState) -> Router {
    let api_routes = Router::new()
        .route("/scheduler/ws", get(scheduler_ws::upgrade))
        .route("/users", get(users::list_users).post(users::create_user))
        .route("/users/{id}/role", put(users::update_role))
        .route("/auth", post(auth::handlers::login))
        .route("/auth/new", post(auth::handlers::register))
        .route("/auth/renew", get(auth::handlers::renew))
        .route(
            "/events",
            get(events::handlers::list_events).post(events::handlers::create_event),
        )
        .route(
            "/events/{id}",
            put(events::handlers::update_event).delete(events::handlers::delete_event),
        )
        .route(
            "/sites",
            get(scada::handlers::list_sites).post(scada::handlers::create_site),
        )
        .route(
            "/sites/{id}",
            put(scada::handlers::update_site).delete(scada::handlers::delete_site),
        )
        .route(
            "/assets",
            get(scada::handlers::list_assets).post(scada::handlers::create_asset),
        )
        .route(
            "/assets/{id}",
            put(scada::handlers::update_asset).delete(scada::handlers::delete_asset),
        )
        .route(
            "/connections",
            get(scada::handlers::list_connections).post(scada::handlers::create_connection),
        )
        .route(
            "/connections/{id}",
            put(scada::handlers::update_connection).delete(scada::handlers::delete_connection),
        )
        .route(
            "/tags",
            get(scada::handlers::list_tags).post(scada::handlers::create_tag),
        )
        .route(
            "/tags/{id}",
            put(scada::handlers::update_tag).delete(scada::handlers::delete_tag),
        )
        .route(
            "/schedules",
            get(scada::handlers::list_schedules).post(scada::handlers::create_schedule),
        )
        .route(
            "/schedules/{id}",
            put(scada::handlers::update_schedule).delete(scada::handlers::delete_schedule),
        );

    let cors = cors_layer(&state.allowed_origin);

    Router::new()
        .nest("/api", api_routes)
        .layer(cors)
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::{Value, json};
    use sqlx::sqlite::SqlitePoolOptions;
    use tower::ServiceExt;

    use super::*;
    use crate::scheduler_link::SchedulerHub;

    async fn app() -> (Router, sqlx::SqlitePool) {
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let state = AppState {
            pool: pool.clone(),
            jwt_secret: "secreto".to_string(),
            jwt_expires_seconds: 3600,
            allowed_origin: "http://localhost".to_string(),
            scheduler: SchedulerHub::new(),
        };
        (build_router(state), pool)
    }

    async fn call(app: &Router, method: &str, uri: &str, token: Option<&str>, body: Value) -> (StatusCode, Value) {
        let mut request = Request::builder().method(method).uri(uri).header("content-type", "application/json");
        if let Some(token) = token {
            request = request.header("x-token", token);
        }
        let response = app.clone().oneshot(request.body(Body::from(body.to_string())).unwrap()).await.unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
    }

    async fn register(app: &Router, email: &str) -> Value {
        let body = json!({ "name": email, "email": email, "password": "secreto1" });
        call(app, "POST", "/api/auth/new", None, body).await.1
    }

    fn event() -> Value {
        json!({ "schedule_id": "no-existe", "start": "2030-01-01T10:00:00Z" })
    }

    #[tokio::test]
    async fn the_first_user_is_admin_and_the_rest_start_as_consultants() {
        let (app, _) = app().await;
        assert_eq!(register(&app, "a@x.com").await["role"], "administrador");
        assert_eq!(register(&app, "b@x.com").await["role"], "consultor");
    }

    #[tokio::test]
    async fn roles_gate_writes_but_not_reads() {
        let (app, pool) = app().await;
        register(&app, "admin@x.com").await;
        let consultant = register(&app, "c@x.com").await;
        let token = consultant["token"].as_str().unwrap();

        // Consultor: lee, pero no escribe.
        assert_eq!(call(&app, "GET", "/api/events", Some(token), Value::Null).await.0, StatusCode::OK);
        assert_eq!(call(&app, "GET", "/api/tags", Some(token), Value::Null).await.0, StatusCode::OK);
        assert_eq!(call(&app, "POST", "/api/events", Some(token), event()).await.0, StatusCode::FORBIDDEN);
        assert_eq!(call(&app, "DELETE", "/api/events/x", Some(token), Value::Null).await.0, StatusCode::FORBIDDEN);
        assert_eq!(call(&app, "POST", "/api/sites", Some(token), json!({ "name": "s" })).await.0, StatusCode::FORBIDDEN);

        // Al subirle el rol, el mismo token vale: el rol se lee de la base de datos.
        sqlx::query("UPDATE users SET role = 'planificador' WHERE email = 'c@x.com'").execute(&pool).await.unwrap();
        // El cuerpo apunta a un schedule inexistente: lo que importa es que ya no sea 403.
        assert_ne!(call(&app, "POST", "/api/events", Some(token), event()).await.0, StatusCode::FORBIDDEN);

        // Sin token, 401.
        assert_eq!(call(&app, "GET", "/api/events", None, Value::Null).await.0, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn a_token_of_a_deleted_user_is_rejected() {
        let (app, pool) = app().await;
        let user = register(&app, "a@x.com").await;
        sqlx::query("DELETE FROM users").execute(&pool).await.unwrap();
        let (status, _) = call(&app, "GET", "/api/events", user["token"].as_str(), Value::Null).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn only_admins_manage_roles_and_the_last_admin_stays() {
        let (app, _) = app().await;
        let admin = register(&app, "admin@x.com").await;
        let other = register(&app, "otro@x.com").await;
        let (admin_token, admin_id) = (admin["token"].as_str().unwrap(), admin["uid"].as_str().unwrap());
        let (other_token, other_id) = (other["token"].as_str().unwrap(), other["uid"].as_str().unwrap());

        // Un consultor no lista ni cambia roles.
        assert_eq!(call(&app, "GET", "/api/users", Some(other_token), Value::Null).await.0, StatusCode::FORBIDDEN);
        let uri = format!("/api/users/{other_id}/role");
        assert_eq!(call(&app, "PUT", &uri, Some(other_token), json!({ "role": "administrador" })).await.0, StatusCode::FORBIDDEN);

        // El administrador lista y asciende; el rol nuevo vale con el mismo token.
        let (status, body) = call(&app, "GET", "/api/users", Some(admin_token), Value::Null).await;
        assert_eq!((status, body["users"].as_array().unwrap().len()), (StatusCode::OK, 2));
        assert_eq!(call(&app, "PUT", &uri, Some(admin_token), json!({ "role": "planificador" })).await.0, StatusCode::OK);
        assert_ne!(call(&app, "POST", "/api/events", Some(other_token), event()).await.0, StatusCode::FORBIDDEN);

        // Valida el rol y el usuario.
        assert_eq!(call(&app, "PUT", &uri, Some(admin_token), json!({ "role": "root" })).await.0, StatusCode::BAD_REQUEST);
        assert_eq!(call(&app, "PUT", "/api/users/nadie/role", Some(admin_token), json!({ "role": "consultor" })).await.0, StatusCode::NOT_FOUND);

        // El último administrador no puede degradarse; con otro administrador sí.
        let own = format!("/api/users/{admin_id}/role");
        assert_eq!(call(&app, "PUT", &own, Some(admin_token), json!({ "role": "consultor" })).await.0, StatusCode::BAD_REQUEST);
        assert_eq!(call(&app, "PUT", &uri, Some(admin_token), json!({ "role": "administrador" })).await.0, StatusCode::OK);
        assert_eq!(call(&app, "PUT", &own, Some(admin_token), json!({ "role": "consultor" })).await.0, StatusCode::OK);
    }

    #[tokio::test]
    async fn admins_create_users_with_a_role_and_they_can_log_in() {
        let (app, _) = app().await;
        let admin = register(&app, "admin@x.com").await;
        let consultant = register(&app, "c@x.com").await;
        let token = admin["token"].as_str().unwrap();
        let body = json!({ "name": " Pía ", "email": "Pia@X.com", "password": "secreto1", "role": "planificador" });

        // Un no administrador no puede crear usuarios.
        let forbidden = call(&app, "POST", "/api/users", consultant["token"].as_str(), body.clone()).await;
        assert_eq!(forbidden.0, StatusCode::FORBIDDEN);

        let (status, created) = call(&app, "POST", "/api/users", Some(token), body.clone()).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!((created["user"]["name"].as_str(), created["user"]["email"].as_str(), created["user"]["role"].as_str()),
                   (Some("Pía"), Some("pia@x.com"), Some("planificador")));

        // Entra con la contraseña indicada y con el rol asignado.
        let (status, login) = call(&app, "POST", "/api/auth", None, json!({ "email": "pia@x.com", "password": "secreto1" })).await;
        assert_eq!((status, login["role"].as_str()), (StatusCode::OK, Some("planificador")));

        // Correo repetido, rol inválido y contraseña corta se rechazan.
        assert_eq!(call(&app, "POST", "/api/users", Some(token), body).await.0, StatusCode::BAD_REQUEST);
        let bad_role = json!({ "name": "X", "email": "x@x.com", "password": "secreto1", "role": "root" });
        assert_eq!(call(&app, "POST", "/api/users", Some(token), bad_role).await.0, StatusCode::BAD_REQUEST);
        let short = json!({ "name": "X", "email": "x@x.com", "password": "123", "role": "consultor" });
        assert_eq!(call(&app, "POST", "/api/users", Some(token), short).await.0, StatusCode::BAD_REQUEST);
    }
}
