//! End-to-end proof of Phase 1's exit criterion: two independently-registered
//! users on one running server can chat — register, log in, create a
//! channel, post a message over REST, and see it arrive in real time over
//! the WebSocket gateway — with zero manual config (the server bootstraps
//! its own DB file, salt, default roles, and the first registrant becomes
//! owner automatically).

use dissedoff_server::config::Config;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio_tungstenite::tungstenite::Message as WsMessage;

async fn spawn_test_server() -> String {
    let data_dir = tempfile::tempdir().expect("tempdir");
    let cfg = Config {
        data_dir: data_dir.keep(),
        bind_addr: "127.0.0.1:0".to_string(),
        default_server_name: "Test Server".to_string(),
    };

    let (app, _state) = dissedoff_server::build_app(&cfg).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local_addr");

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("test server error");
    });

    format!("127.0.0.1:{}", addr.port())
}

#[tokio::test]
async fn two_users_register_and_chat_with_zero_manual_config() {
    let addr = spawn_test_server().await;
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    // Alice registers first -> becomes owner (gets MANAGE_CHANNELS).
    let alice: Value = client
        .post(format!("{base}/api/v1/auth/register"))
        .json(&json!({ "username": "alice", "password": "correct-horse-battery" }))
        .send()
        .await
        .expect("register alice")
        .json()
        .await
        .expect("parse alice register response");
    let alice_token = alice["token"].as_str().expect("alice token").to_string();

    // Bob registers second -> ordinary member, no MANAGE_CHANNELS.
    let bob: Value = client
        .post(format!("{base}/api/v1/auth/register"))
        .json(&json!({ "username": "bob", "password": "correct-horse-battery" }))
        .send()
        .await
        .expect("register bob")
        .json()
        .await
        .expect("parse bob register response");
    let bob_token = bob["token"].as_str().expect("bob token").to_string();

    assert_ne!(
        alice["public_id"], bob["public_id"],
        "distinct users must get distinct hashed public ids"
    );

    // Bob (non-owner) cannot create a channel.
    let bob_create = client
        .post(format!("{base}/api/v1/channels"))
        .bearer_auth(&bob_token)
        .json(&json!({ "name": "general", "kind": "text" }))
        .send()
        .await
        .expect("bob create channel request");
    assert_eq!(bob_create.status(), 403);

    // Alice (owner) can.
    let channel: Value = client
        .post(format!("{base}/api/v1/channels"))
        .bearer_auth(&alice_token)
        .json(&json!({ "name": "general", "kind": "text" }))
        .send()
        .await
        .expect("alice create channel request")
        .json()
        .await
        .expect("parse channel response");
    let channel_id = channel["id"].as_str().expect("channel id").to_string();

    // Bob opens the real-time gateway before the message is sent, so he can
    // observe it arrive as a push rather than having to poll.
    let ws_url = format!("ws://{addr}/api/v1/gateway?token={bob_token}");
    let (mut ws, _resp) = tokio_tungstenite::connect_async(ws_url)
        .await
        .expect("bob connects to gateway");

    let hello = next_envelope(&mut ws).await;
    assert_eq!(hello["t"], "HELLO");
    let ready = next_envelope(&mut ws).await;
    assert_eq!(ready["t"], "READY");

    // Alice posts a message over REST.
    let posted: Value = client
        .post(format!("{base}/api/v1/channels/{channel_id}/messages"))
        .bearer_auth(&alice_token)
        .json(&json!({ "content": "hello from alice" }))
        .send()
        .await
        .expect("alice post message")
        .json()
        .await
        .expect("parse message response");
    assert_eq!(posted["content"], "hello from alice");

    // Bob receives it in real time over the gateway.
    let event = next_envelope(&mut ws).await;
    assert_eq!(event["t"], "MESSAGE_CREATE");
    assert_eq!(event["d"]["content"], "hello from alice");

    // And it's durably readable over REST too (e.g. on reconnect/history scroll).
    let history: Value = client
        .get(format!("{base}/api/v1/channels/{channel_id}/messages"))
        .bearer_auth(&bob_token)
        .send()
        .await
        .expect("bob list messages")
        .json()
        .await
        .expect("parse history response");
    let messages = history.as_array().expect("history is an array");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["content"], "hello from alice");
}

async fn next_envelope(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> Value {
    loop {
        match ws.next().await.expect("ws stream ended early").expect("ws error") {
            WsMessage::Text(text) => return serde_json::from_str(&text).expect("valid json envelope"),
            WsMessage::Ping(payload) => {
                ws.send(WsMessage::Pong(payload)).await.expect("pong");
            }
            other => panic!("unexpected ws message: {other:?}"),
        }
    }
}
