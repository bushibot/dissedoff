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
        voice_enabled: false,
        voice_port: 0,
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

/// Covers the v0.2.0 management endpoints (rename/delete channel, edit/
/// delete message, typing) end to end against a real running server,
/// including that each one's permission check actually rejects a
/// non-owner/non-author, and that each one's gateway event actually
/// reaches a second connected client in real time (not just a 200/204).
#[tokio::test]
async fn channel_and_message_management_with_live_gateway_events() {
    let addr = spawn_test_server().await;
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

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

    let ws_url = format!("ws://{addr}/api/v1/gateway?token={bob_token}");
    let (mut ws, _resp) = tokio_tungstenite::connect_async(ws_url)
        .await
        .expect("bob connects to gateway");
    assert_eq!(next_envelope(&mut ws).await["t"], "HELLO");
    assert_eq!(next_envelope(&mut ws).await["t"], "READY");

    // Rename: non-owner rejected, owner succeeds, event reaches bob.
    let bob_rename = client
        .patch(format!("{base}/api/v1/channels/{channel_id}"))
        .bearer_auth(&bob_token)
        .json(&json!({ "name": "nope" }))
        .send()
        .await
        .expect("bob rename request");
    assert_eq!(bob_rename.status(), 403);

    let renamed: Value = client
        .patch(format!("{base}/api/v1/channels/{channel_id}"))
        .bearer_auth(&alice_token)
        .json(&json!({ "name": "renamed-general" }))
        .send()
        .await
        .expect("alice rename request")
        .json()
        .await
        .expect("parse rename response");
    assert_eq!(renamed["name"], "renamed-general");
    let update_event = next_envelope(&mut ws).await;
    assert_eq!(update_event["t"], "CHANNEL_UPDATE");
    assert_eq!(update_event["d"]["name"], "renamed-general");

    // Typing: reaches the other connected client with the sender's username.
    client
        .post(format!("{base}/api/v1/channels/{channel_id}/typing"))
        .bearer_auth(&alice_token)
        .send()
        .await
        .expect("alice typing request");
    let typing_event = next_envelope(&mut ws).await;
    assert_eq!(typing_event["t"], "TYPING_START");
    assert_eq!(typing_event["d"]["username"], "alice");

    // Post, then edit: author can, edited_at gets set, event reaches bob.
    let posted: Value = client
        .post(format!("{base}/api/v1/channels/{channel_id}/messages"))
        .bearer_auth(&alice_token)
        .json(&json!({ "content": "origonal typo" }))
        .send()
        .await
        .expect("alice post message")
        .json()
        .await
        .expect("parse message response");
    let message_id = posted["id"].as_str().expect("message id").to_string();
    assert!(posted["edited_at"].is_null());
    assert_eq!(next_envelope(&mut ws).await["t"], "MESSAGE_CREATE");

    let bob_edit = client
        .patch(format!("{base}/api/v1/channels/{channel_id}/messages/{message_id}"))
        .bearer_auth(&bob_token)
        .json(&json!({ "content": "bob butting in" }))
        .send()
        .await
        .expect("bob edit request");
    assert_eq!(bob_edit.status(), 403, "non-author must not edit another user's message");

    let edited: Value = client
        .patch(format!("{base}/api/v1/channels/{channel_id}/messages/{message_id}"))
        .bearer_auth(&alice_token)
        .json(&json!({ "content": "original, fixed" }))
        .send()
        .await
        .expect("alice edit request")
        .json()
        .await
        .expect("parse edit response");
    assert_eq!(edited["content"], "original, fixed");
    assert!(edited["edited_at"].is_number(), "edited_at must be set after an edit");
    let update_msg_event = next_envelope(&mut ws).await;
    assert_eq!(update_msg_event["t"], "MESSAGE_UPDATE");
    assert_eq!(update_msg_event["d"]["content"], "original, fixed");

    // Delete: non-author rejected, author succeeds, event reaches bob.
    let bob_delete_msg = client
        .delete(format!("{base}/api/v1/channels/{channel_id}/messages/{message_id}"))
        .bearer_auth(&bob_token)
        .send()
        .await
        .expect("bob delete message request");
    assert_eq!(bob_delete_msg.status(), 403);

    let alice_delete_msg = client
        .delete(format!("{base}/api/v1/channels/{channel_id}/messages/{message_id}"))
        .bearer_auth(&alice_token)
        .send()
        .await
        .expect("alice delete message request");
    assert_eq!(alice_delete_msg.status(), 204);
    let delete_msg_event = next_envelope(&mut ws).await;
    assert_eq!(delete_msg_event["t"], "MESSAGE_DELETE");
    assert_eq!(delete_msg_event["d"]["id"], message_id);

    // Delete channel: non-owner rejected, owner succeeds, event reaches bob.
    let bob_delete_channel = client
        .delete(format!("{base}/api/v1/channels/{channel_id}"))
        .bearer_auth(&bob_token)
        .send()
        .await
        .expect("bob delete channel request");
    assert_eq!(bob_delete_channel.status(), 403);

    let alice_delete_channel = client
        .delete(format!("{base}/api/v1/channels/{channel_id}"))
        .bearer_auth(&alice_token)
        .send()
        .await
        .expect("alice delete channel request");
    assert_eq!(alice_delete_channel.status(), 204);
    let delete_channel_event = next_envelope(&mut ws).await;
    assert_eq!(delete_channel_event["t"], "CHANNEL_DELETE");
    assert_eq!(delete_channel_event["d"]["id"], channel_id);

    let remaining: Value = client
        .get(format!("{base}/api/v1/channels"))
        .bearer_auth(&alice_token)
        .send()
        .await
        .expect("list channels")
        .json()
        .await
        .expect("parse channels response");
    assert_eq!(remaining.as_array().expect("array").len(), 0);
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
