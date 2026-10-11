//! Placeholder. Will hold the gateway connection state machine (connect, HELLO/READY
//! handshake, heartbeat, resume-on-reconnect) and the local message/channel cache that
//! every client (Tauri desktop/mobile) builds its UI on top of.
//!
//! Intentionally left minimal until the server's gateway (Phase 1) exists to connect to —
//! see /config/.claude/plans/mellow-inventing-crayon.md, Phase 1.

pub fn placeholder() -> &'static str {
    "dissedoff-client-core: gateway client not yet implemented (Phase 1)"
}
