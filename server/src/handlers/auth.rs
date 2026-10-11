use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::api_error;
use crate::auth::{generate_token, hash_password, verify_password};
use crate::db::now_unix;
use crate::identity::hash::public_user_id;
use crate::state::AppState;

const SESSION_LIFETIME_SECS: i64 = 60 * 60 * 24 * 30; // 30 days
const MIN_PASSWORD_LEN: usize = 8;

#[derive(Deserialize)]
pub struct RegisterRequest {
    /// Optional per the plan's "all registration fields optional except the
    /// hashed user id" requirement — if omitted, a default handle is
    /// generated. Full per-server-configurable required-fields policy is not
    /// yet implemented; this is the first, partial step toward it.
    pub username: Option<String>,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub public_id: String,
    pub username: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<dissedoff_protocol::ApiError>)> {
    if req.password.len() < MIN_PASSWORD_LEN {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "weak_password",
            format!("Password must be at least {MIN_PASSWORD_LEN} characters"),
        ));
    }

    let username = req
        .username
        .filter(|u| !u.trim().is_empty())
        .unwrap_or_else(|| format!("user-{}", &Uuid::new_v4().to_string()[..8]));

    let existing: Option<String> = sqlx::query_scalar("SELECT internal_id FROM users WHERE username = ?")
        .bind(&username)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", "db error"))?;
    if existing.is_some() {
        return Err(api_error(StatusCode::CONFLICT, "username_taken", "That username is already taken"));
    }

    let password_hash =
        hash_password(&req.password).map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", "hash error"))?;
    let internal_id = Uuid::new_v4();
    let now = now_unix();

    sqlx::query("INSERT INTO users (internal_id, username, password_hash, created_at) VALUES (?, ?, ?, ?)")
        .bind(internal_id.to_string())
        .bind(&username)
        .bind(&password_hash)
        .bind(now)
        .execute(&state.db)
        .await
        .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", "db error"))?;

    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await
        .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", "db error"))?;
    let is_first_user = user_count == 1;

    assign_role(&state, internal_id, "everyone").await?;
    if is_first_user {
        assign_role(&state, internal_id, "owner").await?;
        tracing::info!(%username, "first registered user — granted owner role");
    }

    let token = issue_session(&state, internal_id).await?;

    Ok(Json(AuthResponse {
        token,
        public_id: public_user_id(&state.salt, internal_id),
        username,
    }))
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<dissedoff_protocol::ApiError>)> {
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT internal_id, password_hash FROM users WHERE username = ?")
            .bind(&req.username)
            .fetch_optional(&state.db)
            .await
            .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", "db error"))?;

    let Some((internal_id_str, password_hash)) = row else {
        return Err(api_error(StatusCode::UNAUTHORIZED, "invalid_credentials", "Invalid username or password"));
    };

    if !verify_password(&req.password, &password_hash) {
        return Err(api_error(StatusCode::UNAUTHORIZED, "invalid_credentials", "Invalid username or password"));
    }

    let internal_id = Uuid::parse_str(&internal_id_str)
        .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", "corrupt id"))?;
    let token = issue_session(&state, internal_id).await?;

    Ok(Json(AuthResponse {
        token,
        public_id: public_user_id(&state.salt, internal_id),
        username: req.username,
    }))
}

async fn issue_session(
    state: &AppState,
    internal_id: Uuid,
) -> Result<String, (StatusCode, Json<dissedoff_protocol::ApiError>)> {
    let (token, token_hash) = generate_token();
    let now = now_unix();
    sqlx::query("INSERT INTO sessions (token_hash, user_internal_id, created_at, expires_at) VALUES (?, ?, ?, ?)")
        .bind(&token_hash)
        .bind(internal_id.to_string())
        .bind(now)
        .bind(now + SESSION_LIFETIME_SECS)
        .execute(&state.db)
        .await
        .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", "db error"))?;
    Ok(token)
}

async fn assign_role(
    state: &AppState,
    internal_id: Uuid,
    role_name: &str,
) -> Result<(), (StatusCode, Json<dissedoff_protocol::ApiError>)> {
    let role_id: String = sqlx::query_scalar("SELECT id FROM roles WHERE name = ?")
        .bind(role_name)
        .fetch_one(&state.db)
        .await
        .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", "missing bootstrap role"))?;
    sqlx::query("INSERT OR IGNORE INTO member_roles (user_internal_id, role_id) VALUES (?, ?)")
        .bind(internal_id.to_string())
        .bind(role_id)
        .execute(&state.db)
        .await
        .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", "db error"))?;
    Ok(())
}
