//! Phase 2 (not yet implemented). Will own the Murmur child process lifecycle:
//!
//! - On first boot: generate a TLS cert and a minimal `murmur.ini` from a template
//!   into the data volume, so no admin ever hand-edits Murmur's config or certs.
//! - Launch Murmur via `tokio::process::Command`, supervise it (restart-on-crash,
//!   health check), and expose its ICE/control interface only on localhost.
//! - Proxy every control operation (create voice channel, move/mute/kick a
//!   session) through the main server's own REST API and permission engine —
//!   Dissedoff's authorization checks happen before any call reaches Murmur;
//!   Murmur's own ACL system is bypassed entirely in favor of that.
//! - Subscribe to Murmur's control-channel events and re-publish them as native
//!   `VoiceStateUpdate` gateway events, so clients never talk to Murmur directly
//!   or need to know it exists.
//!
//! See /config/.claude/plans/mellow-inventing-crayon.md, "Voice subsystem design"
//! and Phase 2 roadmap entry, for the full rationale.

pub struct MurmurSupervisor;

impl MurmurSupervisor {
    /// Not yet implemented — Phase 2.
    pub fn start() -> Result<Self, &'static str> {
        Err("murmur supervisor not yet implemented (Phase 2)")
    }
}
