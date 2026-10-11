pub mod auth;
pub mod config;
pub mod db;
pub mod handlers;
pub mod identity;
pub mod snowflake;
pub mod state;
pub mod voice;

use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;

use config::Config;
use state::{new_app_state, AppState};

/// Build the router for a server that has already finished DB/salt setup.
/// Exposed separately from [`build_app`] so integration tests can inspect or
/// reuse `state` after building the router.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/auth/register", post(handlers::auth::register))
        .route("/api/v1/auth/login", post(handlers::auth::login))
        .route(
            "/api/v1/server",
            get(handlers::server_info::get_server_info).patch(handlers::server_info::update_server_info),
        )
        .route(
            "/api/v1/channels",
            get(handlers::channels::list_channels).post(handlers::channels::create_channel),
        )
        .route(
            "/api/v1/channels/:channel_id/messages",
            get(handlers::channels::list_messages).post(handlers::channels::create_message),
        )
        .route("/api/v1/gateway", get(handlers::gateway::gateway_upgrade))
        .with_state(state)
}

/// Run full first-boot setup (data dir, DB connect + migrate, bootstrap
/// roles/server_info, load/generate the server salt) and return a ready
/// router plus the `AppState` it's wired to.
pub async fn build_app(cfg: &Config) -> (Router, AppState) {
    std::fs::create_dir_all(&cfg.data_dir).expect("failed to create data dir");

    let pool = db::connect(&cfg.database_path())
        .await
        .expect("failed to connect to database / run migrations");
    db::bootstrap(&pool, &cfg.default_server_name)
        .await
        .expect("failed to bootstrap server_info/roles");

    let salt = db::load_server_salt(&cfg.salt_path()).expect("failed to load/generate server salt");

    let state = new_app_state(pool, salt);
    let router = build_router(state.clone());
    (router, state)
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok", "service": "dissedoff-server" }))
}
