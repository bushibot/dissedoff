use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::http::HeaderValue;
use axum::response::{IntoResponse, Response};
use dissedoff_protocol::{Envelope, EventType};
use serde::Deserialize;

use crate::auth::authenticate;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct GatewayQuery {
    pub token: String,
}

/// WebSocket gateway entry point. Auth is via `?token=` (the browser
/// WebSocket API can't set an Authorization header on the upgrade request),
/// validated with the same session-token path as REST requests.
pub async fn gateway_upgrade(
    State(state): State<AppState>,
    Query(query): Query<GatewayQuery>,
    ws: WebSocketUpgrade,
) -> Response {
    let mut headers = HeaderMap::new();
    if let Ok(value) = HeaderValue::from_str(&format!("Bearer {}", query.token)) {
        headers.insert("authorization", value);
    }

    match authenticate(&state, &headers).await {
        Ok(user) => ws.on_upgrade(move |socket| handle_socket(socket, state, user.username)),
        Err(_) => axum::http::StatusCode::UNAUTHORIZED.into_response(),
    }
}

async fn handle_socket(mut socket: WebSocket, state: AppState, username: String) {
    let mut rx = state.broadcast.subscribe();

    let hello = Envelope {
        t: EventType::Hello,
        seq: 0,
        d: serde_json::json!({ "heartbeat_interval_ms": 30_000 }),
    };
    if send_envelope(&mut socket, &hello).await.is_err() {
        return;
    }

    let ready = Envelope {
        t: EventType::Ready,
        seq: 0,
        d: serde_json::json!({ "username": username }),
    };
    if send_envelope(&mut socket, &ready).await.is_err() {
        return;
    }

    loop {
        tokio::select! {
            broadcasted = rx.recv() => {
                match broadcasted {
                    Ok(envelope) => {
                        if send_envelope(&mut socket, &envelope).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(env) = serde_json::from_str::<Envelope>(&text) {
                            if env.t == EventType::Heartbeat {
                                let ack = Envelope { t: EventType::HeartbeatAck, seq: env.seq, d: serde_json::Value::Null };
                                if send_envelope(&mut socket, &ack).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
        }
    }
}

async fn send_envelope(socket: &mut WebSocket, envelope: &Envelope) -> Result<(), axum::Error> {
    let text = serde_json::to_string(envelope).unwrap_or_default();
    socket.send(Message::Text(text)).await
}
