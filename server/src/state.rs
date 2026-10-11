use dissedoff_protocol::Envelope;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::identity::hash::ServerSalt;
use crate::snowflake::SnowflakeGenerator;
use crate::voice::murmur_supervisor::MurmurSupervisor;

pub struct AppStateInner {
    pub db: SqlitePool,
    pub salt: ServerSalt,
    pub broadcast: broadcast::Sender<Envelope>,
    pub snowflake: SnowflakeGenerator,
    pub voice: Option<MurmurSupervisor>,
}

pub type AppState = Arc<AppStateInner>;

pub fn new_app_state(db: SqlitePool, salt: ServerSalt, voice: Option<MurmurSupervisor>) -> AppState {
    let (broadcast, _) = broadcast::channel(1024);
    Arc::new(AppStateInner {
        db,
        salt,
        broadcast,
        snowflake: SnowflakeGenerator::new(),
        voice,
    })
}
