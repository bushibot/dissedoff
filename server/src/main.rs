mod identity;
mod voice;

use axum::{routing::get, Json, Router};
use serde_json::json;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new().route("/health", get(health));

    let addr = "0.0.0.0:8080";
    tracing::info!("dissedoff-server listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind listen address");
    axum::serve(listener, app)
        .await
        .expect("server error");
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok", "service": "dissedoff-server" }))
}
