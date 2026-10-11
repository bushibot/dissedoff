use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use dissedoff_permissions::Permissions;
use dissedoff_protocol::{Envelope, EventType};
use serde::{Deserialize, Serialize};

use super::{api_error, auth_error_response};
use crate::auth::authenticate;
use crate::db::now_unix;
use crate::identity::hash::public_user_id;
use crate::state::AppState;

#[derive(Serialize, sqlx::FromRow)]
pub struct ChannelResponse {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub parent_id: Option<String>,
    pub position: i32,
}

pub async fn list_channels(
    State(state): State<AppState>,
) -> Result<Json<Vec<ChannelResponse>>, (StatusCode, Json<dissedoff_protocol::ApiError>)> {
    let channels: Vec<ChannelResponse> = sqlx::query_as(
        "SELECT id, name, kind, parent_id, position FROM channels ORDER BY position ASC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", "db error"))?;
    Ok(Json(channels))
}

#[derive(Deserialize)]
pub struct CreateChannelRequest {
    pub name: String,
    pub kind: String, // "text" | "voice" | "category"
}

pub async fn create_channel(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateChannelRequest>,
) -> Result<Json<ChannelResponse>, (StatusCode, Json<dissedoff_protocol::ApiError>)> {
    let user = authenticate(&state, &headers).await.map_err(auth_error_response)?;
    if !user.permissions.contains(Permissions::MANAGE_CHANNELS) {
        return Err(api_error(StatusCode::FORBIDDEN, "missing_permission", "requires MANAGE_CHANNELS"));
    }
    if !["text", "voice", "category"].contains(&req.kind.as_str()) {
        return Err(api_error(StatusCode::BAD_REQUEST, "invalid_kind", "kind must be text, voice, or category"));
    }

    let id = state.snowflake.next_id();
    sqlx::query("INSERT INTO channels (id, name, kind, parent_id, position, created_at) VALUES (?, ?, ?, NULL, 0, ?)")
        .bind(&id)
        .bind(&req.name)
        .bind(&req.kind)
        .bind(now_unix())
        .execute(&state.db)
        .await
        .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", "db error"))?;

    let response = ChannelResponse {
        id: id.clone(),
        name: req.name,
        kind: req.kind,
        parent_id: None,
        position: 0,
    };

    let _ = state.broadcast.send(Envelope {
        t: EventType::ChannelCreate,
        seq: 0,
        d: serde_json::to_value(&response).unwrap_or_default(),
    });

    Ok(Json(response))
}

#[derive(Serialize)]
pub struct MessageResponse {
    pub id: String,
    pub channel_id: String,
    pub author_public_id: String,
    pub author_username: String,
    pub content: String,
    pub created_at: i64,
}

#[derive(Deserialize)]
pub struct ListMessagesQuery {
    pub before: Option<String>,
    pub limit: Option<i64>,
}

pub async fn list_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(channel_id): Path<String>,
    Query(query): Query<ListMessagesQuery>,
) -> Result<Json<Vec<MessageResponse>>, (StatusCode, Json<dissedoff_protocol::ApiError>)> {
    let user = authenticate(&state, &headers).await.map_err(auth_error_response)?;
    if !user.permissions.contains(Permissions::VIEW_CHANNEL) {
        return Err(api_error(StatusCode::FORBIDDEN, "missing_permission", "requires VIEW_CHANNEL"));
    }
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let rows: Vec<(String, String, String, String, i64)> = if let Some(before) = &query.before {
        sqlx::query_as(
            "SELECT m.id, m.author_internal_id, u.username, m.content, m.created_at
             FROM messages m JOIN users u ON u.internal_id = m.author_internal_id
             WHERE m.channel_id = ? AND m.id < ? ORDER BY m.id DESC LIMIT ?",
        )
        .bind(&channel_id)
        .bind(before)
        .bind(limit)
        .fetch_all(&state.db)
        .await
    } else {
        sqlx::query_as(
            "SELECT m.id, m.author_internal_id, u.username, m.content, m.created_at
             FROM messages m JOIN users u ON u.internal_id = m.author_internal_id
             WHERE m.channel_id = ? ORDER BY m.id DESC LIMIT ?",
        )
        .bind(&channel_id)
        .bind(limit)
        .fetch_all(&state.db)
        .await
    }
    .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", "db error"))?;

    let messages = rows
        .into_iter()
        .map(|(id, author_internal_id, username, content, created_at)| {
            let author_uuid = uuid::Uuid::parse_str(&author_internal_id).unwrap_or_default();
            MessageResponse {
                id,
                channel_id: channel_id.clone(),
                author_public_id: public_user_id(&state.salt, author_uuid),
                author_username: username,
                content,
                created_at,
            }
        })
        .collect();

    Ok(Json(messages))
}

#[derive(Deserialize)]
pub struct CreateMessageRequest {
    pub content: String,
}

pub async fn create_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(channel_id): Path<String>,
    Json(req): Json<CreateMessageRequest>,
) -> Result<Json<MessageResponse>, (StatusCode, Json<dissedoff_protocol::ApiError>)> {
    let user = authenticate(&state, &headers).await.map_err(auth_error_response)?;
    if !user.permissions.contains(Permissions::SEND_MESSAGES) {
        return Err(api_error(StatusCode::FORBIDDEN, "missing_permission", "requires SEND_MESSAGES"));
    }
    if req.content.trim().is_empty() || req.content.len() > 4000 {
        return Err(api_error(StatusCode::BAD_REQUEST, "invalid_content", "content must be 1-4000 characters"));
    }

    let channel_exists: Option<String> = sqlx::query_scalar("SELECT id FROM channels WHERE id = ?")
        .bind(&channel_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", "db error"))?;
    if channel_exists.is_none() {
        return Err(api_error(StatusCode::NOT_FOUND, "channel_not_found", "no such channel"));
    }

    let id = state.snowflake.next_id();
    let created_at = now_unix();
    sqlx::query("INSERT INTO messages (id, channel_id, author_internal_id, content, created_at) VALUES (?, ?, ?, ?, ?)")
        .bind(&id)
        .bind(&channel_id)
        .bind(user.internal_id.to_string())
        .bind(&req.content)
        .bind(created_at)
        .execute(&state.db)
        .await
        .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", "db error"))?;

    let response = MessageResponse {
        id,
        channel_id,
        author_public_id: user.public_id.clone(),
        author_username: user.username.clone(),
        content: req.content,
        created_at,
    };

    let _ = state.broadcast.send(Envelope {
        t: EventType::MessageCreate,
        seq: 0,
        d: serde_json::to_value(&response).unwrap_or_default(),
    });

    Ok(Json(response))
}
