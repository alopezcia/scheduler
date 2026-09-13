use axum::Router;
use axum::http::{HeaderName, HeaderValue, Method, header};
use axum::routing::{get, post, put};
use tower_http::cors::CorsLayer;

use crate::auth;
use crate::events;
use crate::scada;
use crate::state::AppState;

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
