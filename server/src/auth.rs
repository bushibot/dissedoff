use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::http::HeaderMap;
use base64::Engine;
use dissedoff_permissions::{self as perms, Role as PermRole};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::db::now_unix;
use crate::identity::hash::public_user_id;
use crate::state::AppState;

pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| e.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

/// Generate a new opaque bearer token. Returns the token to hand to the
/// client and the sha256 hex digest to store in the DB — the raw token is
/// never persisted, so a DB leak alone doesn't yield usable sessions.
pub fn generate_token() -> (String, String) {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
    let hash = hex::encode(Sha256::digest(token.as_bytes()));
    (token, hash)
}

pub struct AuthedUser {
    pub internal_id: Uuid,
    pub username: String,
    pub public_id: String,
    pub permissions: perms::Permissions,
}

#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    InvalidToken,
    Internal,
}

pub async fn authenticate(state: &AppState, headers: &HeaderMap) -> Result<AuthedUser, AuthError> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(AuthError::MissingToken)?;

    let token_hash = hex::encode(Sha256::digest(token.as_bytes()));

    let row: Option<(String, String, i64)> = sqlx::query_as(
        "SELECT u.internal_id, u.username, s.expires_at
         FROM sessions s JOIN users u ON u.internal_id = s.user_internal_id
         WHERE s.token_hash = ?",
    )
    .bind(&token_hash)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| AuthError::Internal)?;

    let (internal_id_str, username, expires_at) = row.ok_or(AuthError::InvalidToken)?;
    if expires_at < now_unix() {
        return Err(AuthError::InvalidToken);
    }
    let internal_id = Uuid::parse_str(&internal_id_str).map_err(|_| AuthError::Internal)?;

    let permissions = load_permissions(state, internal_id)
        .await
        .map_err(|_| AuthError::Internal)?;

    Ok(AuthedUser {
        internal_id,
        username,
        public_id: public_user_id(&state.salt, internal_id),
        permissions,
    })
}

pub async fn load_permissions(
    state: &AppState,
    internal_id: Uuid,
) -> sqlx::Result<perms::Permissions> {
    let rows: Vec<(i32, i64)> = sqlx::query_as(
        "SELECT r.position, r.permissions FROM roles r
         JOIN member_roles mr ON mr.role_id = r.id
         WHERE mr.user_internal_id = ?",
    )
    .bind(internal_id.to_string())
    .fetch_all(&state.db)
    .await?;

    let roles: Vec<PermRole> = rows
        .into_iter()
        .map(|(position, bits)| PermRole {
            position,
            permissions: perms::Permissions::from_bits_truncate(bits as u64),
        })
        .collect();

    Ok(perms::resolve(&roles, &[], None))
}
