use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use dissedoff_permissions::Permissions;
use serde::{Deserialize, Serialize};

use super::{api_error, auth_error_response};
use crate::auth::authenticate;
use crate::state::AppState;

#[derive(Serialize)]
pub struct ServerInfoResponse {
    pub public_server_id: String,
    pub name: String,
    pub description: Option<String>,
    pub visibility: String,
    pub federation_mode: String,
}

pub async fn get_server_info(
    State(state): State<AppState>,
) -> Result<Json<ServerInfoResponse>, (StatusCode, Json<dissedoff_protocol::ApiError>)> {
    let row: (String, String, Option<String>, String, String) = sqlx::query_as(
        "SELECT public_server_id, name, description, visibility, federation_mode FROM server_info WHERE id = 1",
    )
    .fetch_one(&state.db)
    .await
    .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", "db error"))?;

    Ok(Json(ServerInfoResponse {
        public_server_id: row.0,
        name: row.1,
        description: row.2,
        visibility: row.3,
        federation_mode: row.4,
    }))
}

#[derive(Deserialize)]
pub struct UpdateServerInfoRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub visibility: Option<String>,
}

pub async fn update_server_info(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UpdateServerInfoRequest>,
) -> Result<Json<ServerInfoResponse>, (StatusCode, Json<dissedoff_protocol::ApiError>)> {
    let user = authenticate(&state, &headers).await.map_err(auth_error_response)?;
    if !user.permissions.contains(Permissions::MANAGE_SERVER) {
        return Err(api_error(StatusCode::FORBIDDEN, "missing_permission", "requires MANAGE_SERVER"));
    }

    if let Some(name) = &req.name {
        sqlx::query("UPDATE server_info SET name = ? WHERE id = 1")
            .bind(name)
            .execute(&state.db)
            .await
            .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", "db error"))?;
    }
    if let Some(description) = &req.description {
        sqlx::query("UPDATE server_info SET description = ? WHERE id = 1")
            .bind(description)
            .execute(&state.db)
            .await
            .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", "db error"))?;
    }
    if let Some(visibility) = &req.visibility {
        if visibility != "public" && visibility != "private" {
            return Err(api_error(StatusCode::BAD_REQUEST, "invalid_visibility", "visibility must be 'public' or 'private'"));
        }
        sqlx::query("UPDATE server_info SET visibility = ? WHERE id = 1")
            .bind(visibility)
            .execute(&state.db)
            .await
            .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", "db error"))?;
    }

    get_server_info(State(state)).await
}
