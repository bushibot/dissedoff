//! Wire types shared by the Dissedoff server and every client.
//!
//! Event names, field names, and envelope shape are defined independently here
//! and are not derived from, or intended to mirror, Discord's actual gateway
//! wire format. See `/config/.claude/plans/mellow-inventing-crayon.md` section
//! "Protocol" for the design rationale (clean-room: same general REST+WS
//! *pattern* as Discord's public developer docs describe, independently named).

use serde::{Deserialize, Serialize};

pub mod rest;

/// A single message sent over the real-time gateway WebSocket, in either direction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    /// Event type tag, from the Dissedoff-defined vocabulary in [`EventType`].
    pub t: EventType,
    /// Monotonic per-connection sequence number, used for resume/replay after a reconnect.
    pub seq: u64,
    /// Event-specific payload.
    pub d: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventType {
    // Connection lifecycle
    Hello,
    Ready,
    Resume,
    Heartbeat,
    HeartbeatAck,

    // Messaging
    MessageCreate,
    MessageUpdate,
    MessageDelete,
    TypingStart,

    // Presence / membership
    PresenceUpdate,
    MemberJoin,
    MemberLeave,

    // Voice
    VoiceStateUpdate,
    VoiceServerInfo,

    // Server management
    ChannelCreate,
    ChannelUpdate,
    ChannelDelete,
    RoleCreate,
    RoleUpdate,
    RoleDelete,
    ServerUpdate,

    // Federation (only emitted when a server has federation enabled)
    PeerDiscovered,
    FederatedPresenceUpdate,
    FederatedMention,
}

/// A server-defined error response shape for REST endpoints.
/// Deliberately distinct from Discord's own JSON error schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error_code: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_round_trips_through_json() {
        let env = Envelope {
            t: EventType::MessageCreate,
            seq: 1,
            d: serde_json::json!({ "channel_id": "abc", "content": "hi" }),
        };
        let json = serde_json::to_string(&env).unwrap();
        let parsed: Envelope = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.t, EventType::MessageCreate);
        assert_eq!(parsed.seq, 1);
    }
}
