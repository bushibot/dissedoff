use dissedoff_protocol::{Envelope, EventType};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::rest::ClientError;

/// The gateway connection state machine every client builds its real-time UI
/// on top of: connect, consume the HELLO/READY handshake, keep the
/// connection alive with heartbeats, and hand every other event to the
/// caller through `incoming`.
pub struct GatewayClient {
    pub incoming: mpsc::Receiver<Envelope>,
}

impl GatewayClient {
    pub async fn connect(http_base_url: &str, token: &str) -> Result<Self, ClientError> {
        let ws_base = http_base_url
            .replacen("https://", "wss://", 1)
            .replacen("http://", "ws://", 1);
        let url = format!("{ws_base}/api/v1/gateway?token={token}");

        let (stream, _) = tokio_tungstenite::connect_async(url)
            .await
            .map_err(|e| ClientError::Api {
                code: "gateway_connect_failed".to_string(),
                message: e.to_string(),
            })?;

        let (mut tx, mut rx) = stream.split();
        let (incoming_tx, incoming_rx) = mpsc::channel(256);

        // First frame must be HELLO; pull the heartbeat interval out of it
        // before handing control to the steady-state loop below.
        let heartbeat_interval_ms = match rx.next().await {
            Some(Ok(WsMessage::Text(text))) => {
                let hello: Envelope = serde_json::from_str(&text).unwrap_or(Envelope {
                    t: EventType::Hello,
                    seq: 0,
                    d: serde_json::Value::Null,
                });
                hello.d["heartbeat_interval_ms"].as_u64().unwrap_or(30_000)
            }
            _ => 30_000,
        };

        tokio::spawn(async move {
            let mut heartbeat = tokio::time::interval(std::time::Duration::from_millis(heartbeat_interval_ms));
            heartbeat.tick().await; // first tick fires immediately; skip it

            loop {
                tokio::select! {
                    frame = rx.next() => {
                        match frame {
                            Some(Ok(WsMessage::Text(text))) => {
                                if let Ok(envelope) = serde_json::from_str::<Envelope>(&text) {
                                    if incoming_tx.send(envelope).await.is_err() {
                                        break;
                                    }
                                }
                            }
                            Some(Ok(WsMessage::Close(_))) | None => break,
                            Some(Err(_)) => break,
                            _ => {}
                        }
                    }
                    _ = heartbeat.tick() => {
                        let beat = Envelope { t: EventType::Heartbeat, seq: 0, d: serde_json::Value::Null };
                        let text = serde_json::to_string(&beat).unwrap_or_default();
                        if tx.send(WsMessage::Text(text)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Ok(Self { incoming: incoming_rx })
    }
}
