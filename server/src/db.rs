use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::Path;
use uuid::Uuid;

use crate::identity::hash::ServerSalt;

pub async fn connect(db_path: &Path) -> sqlx::Result<SqlitePool> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| sqlx::Error::Io(e))?;
    }
    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

/// Create the singleton server_info row and the default/owner roles on first
/// boot. Idempotent — safe to call on every startup.
pub async fn bootstrap(pool: &SqlitePool, default_server_name: &str) -> sqlx::Result<()> {
    let existing: Option<i64> = sqlx::query_scalar("SELECT id FROM server_info WHERE id = 1")
        .fetch_optional(pool)
        .await?;

    if existing.is_none() {
        let server_id = Uuid::new_v4().to_string();
        let public_server_id = bs58::encode(Uuid::new_v4().as_bytes()).into_string();
        let now = now_unix();
        sqlx::query(
            "INSERT INTO server_info (id, server_id, public_server_id, name, visibility, federation_mode, created_at)
             VALUES (1, ?, ?, ?, 'private', 'disabled', ?)",
        )
        .bind(&server_id)
        .bind(&public_server_id)
        .bind(default_server_name)
        .bind(now)
        .execute(pool)
        .await?;
        tracing::info!(public_server_id, "bootstrapped new Dissedoff server");
    }

    ensure_role(pool, "owner", 100, dissedoff_permissions::Permissions::ADMINISTRATOR.bits(), false).await?;
    ensure_role(
        pool,
        "everyone",
        0,
        (dissedoff_permissions::Permissions::VIEW_CHANNEL
            | dissedoff_permissions::Permissions::SEND_MESSAGES
            | dissedoff_permissions::Permissions::ATTACH_FILES
            | dissedoff_permissions::Permissions::CREATE_INVITE
            | dissedoff_permissions::Permissions::CONNECT_VOICE
            | dissedoff_permissions::Permissions::SPEAK)
            .bits(),
        true,
    )
    .await?;

    Ok(())
}

async fn ensure_role(
    pool: &SqlitePool,
    name: &str,
    position: i32,
    permissions_bits: u64,
    is_default: bool,
) -> sqlx::Result<()> {
    let existing: Option<String> = sqlx::query_scalar("SELECT id FROM roles WHERE name = ?")
        .bind(name)
        .fetch_optional(pool)
        .await?;
    if existing.is_none() {
        sqlx::query(
            "INSERT INTO roles (id, name, position, permissions, is_default) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(name)
        .bind(position)
        .bind(permissions_bits as i64)
        .bind(is_default as i64)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub fn load_server_salt(path: &Path) -> std::io::Result<ServerSalt> {
    ServerSalt::load_or_generate(path)
}

pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before Unix epoch")
        .as_secs() as i64
}
