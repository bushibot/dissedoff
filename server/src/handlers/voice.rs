use axum::extract::State;
use axum::Json;
use dissedoff_protocol::rest::VoiceStatusResponse;

use crate::state::AppState;

pub async fn voice_status(State(state): State<AppState>) -> Json<VoiceStatusResponse> {
    match &state.voice {
        Some(supervisor) => Json(VoiceStatusResponse {
            enabled: true,
            running: supervisor.is_running(),
            port: supervisor.port,
        }),
        None => Json(VoiceStatusResponse {
            enabled: false,
            running: false,
            port: 0,
        }),
    }
}
