use axum::extract::State;
use axum::Json;
use serde_json::json;

use crate::state::AppState;

pub async fn voice_status(State(state): State<AppState>) -> Json<serde_json::Value> {
    match &state.voice {
        Some(supervisor) => Json(json!({
            "enabled": true,
            "running": supervisor.is_running(),
            "port": supervisor.port,
        })),
        None => Json(json!({ "enabled": false })),
    }
}
