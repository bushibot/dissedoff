//! The gateway connection state machine and REST client every Dissedoff
//! client (the Tauri desktop/mobile apps, and the reference CLI) builds its
//! UI on top of, so that logic lives in exactly one place rather than being
//! re-derived per client.

pub mod gateway;
pub mod rest;

pub use gateway::GatewayClient;
pub use rest::{ClientError, RestClient};
