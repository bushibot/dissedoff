//! Per-server-salted hashed user IDs.
//!
//! Design (plan section "Data model & identity"): a user's true internal ID
//! (a server-local UUID primary key) never leaves the server it was created
//! on. Every externally visible identifier — in REST responses, gateway
//! events, and federated payloads — is a per-server-salted HMAC of that
//! internal ID instead, so the same human cannot be correlated across two
//! unrelated Dissedoff servers by ID alone.
//!
//! The salt is generated once per server at first boot (see [`ServerSalt::generate`])
//! and must be persisted only in the server's local data volume. It is never
//! transmitted anywhere, including during federation handshakes. Rotating it
//! re-keys every public ID on that server — a deliberate privacy feature
//! (e.g. after a suspected leak), not a bug.

use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// A server's local, secret HMAC key. Generate once per server at first boot
/// and persist it in the data volume — never commit it, never transmit it.
#[derive(Clone)]
pub struct ServerSalt([u8; 32]);

impl ServerSalt {
    pub fn generate() -> Self {
        use rand::RngCore;
        let mut bytes = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
        Self(bytes)
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Derive the externally-visible public user id for `internal_user_id` on the
/// server whose secret is `salt`. Truncated to 16 bytes then base58-encoded
/// for a short, human-shareable handle.
pub fn public_user_id(salt: &ServerSalt, internal_user_id: Uuid) -> String {
    let mut mac =
        HmacSha256::new_from_slice(salt.as_bytes()).expect("HMAC accepts any key length");
    mac.update(internal_user_id.as_bytes());
    let full = mac.finalize().into_bytes();
    bs58::encode(&full[0..16]).into_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_salt_and_id_produce_a_stable_public_id() {
        let salt = ServerSalt::from_bytes([7u8; 32]);
        let user_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
        let a = public_user_id(&salt, user_id);
        let b = public_user_id(&salt, user_id);
        assert_eq!(a, b);
    }

    #[test]
    fn different_salts_produce_different_public_ids_for_the_same_user() {
        let salt_a = ServerSalt::from_bytes([1u8; 32]);
        let salt_b = ServerSalt::from_bytes([2u8; 32]);
        let user_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
        assert_ne!(
            public_user_id(&salt_a, user_id),
            public_user_id(&salt_b, user_id)
        );
    }

    #[test]
    fn rotating_the_salt_rekeys_the_public_id() {
        let old_salt = ServerSalt::generate();
        let new_salt = ServerSalt::generate();
        let user_id = Uuid::new_v4();
        assert_ne!(
            public_user_id(&old_salt, user_id),
            public_user_id(&new_salt, user_id)
        );
    }

    #[test]
    fn public_id_never_contains_the_internal_uuid_text() {
        let salt = ServerSalt::generate();
        let user_id = Uuid::new_v4();
        let public_id = public_user_id(&salt, user_id);
        assert!(!public_id.contains(&user_id.to_string()));
    }
}
