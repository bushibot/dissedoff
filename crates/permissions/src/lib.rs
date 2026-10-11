//! Role permission bitfield model.
//!
//! Same *shape* as the permission model Discord documents for bot developers
//! (role + per-channel overwrite, resolved by role position), but the bit
//! assignments and precedence algorithm below are defined independently, not
//! copied from Discord's actual bit layout.

use bitflags::bitflags;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Permissions: u64 {
        const VIEW_CHANNEL      = 1 << 0;
        const SEND_MESSAGES     = 1 << 1;
        const ATTACH_FILES      = 1 << 2;
        const MANAGE_MESSAGES   = 1 << 3;
        const CONNECT_VOICE     = 1 << 4;
        const SPEAK             = 1 << 5;
        const MUTE_MEMBERS      = 1 << 6;
        const DEAFEN_MEMBERS    = 1 << 7;
        const CREATE_INVITE     = 1 << 8;
        const KICK_MEMBERS      = 1 << 9;
        const BAN_MEMBERS       = 1 << 10;
        const MANAGE_CHANNELS   = 1 << 11;
        const MANAGE_ROLES      = 1 << 12;
        const MANAGE_SERVER     = 1 << 13;
        const ADMINISTRATOR     = 1 << 63;
    }
}

// Serialized as the raw u64 bitfield on the wire, rather than relying on
// bitflags' own optional serde integration, to keep the wire shape obvious
// and independent of that crate's internal representation.
impl Serialize for Permissions {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.bits().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Permissions {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bits = u64::deserialize(deserializer)?;
        Ok(Permissions::from_bits_truncate(bits))
    }
}

/// A single role, ordered by `position` (higher = takes precedence when roles conflict).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub position: i32,
    pub permissions: Permissions,
}

/// A per-channel allow/deny overwrite, targeted at a role or a specific member.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Overwrite {
    pub allow: Permissions,
    pub deny: Permissions,
}

/// Resolve a member's effective permissions in a channel.
///
/// Algorithm (independently defined, not ported from any other platform):
/// 1. Start from the bitwise OR of all the member's role permissions.
/// 2. `ADMINISTRATOR` short-circuits to all permissions granted.
/// 3. Apply role overwrites for the channel in ascending role `position` order
///    (lowest position first, so higher-position roles win ties).
/// 4. Apply the member-specific overwrite last, if present — it always wins.
pub fn resolve(
    member_roles: &[Role],
    role_overwrites: &[(i32, Overwrite)], // (role position, overwrite)
    member_overwrite: Option<&Overwrite>,
) -> Permissions {
    let mut base = Permissions::empty();
    for role in member_roles {
        base |= role.permissions;
    }
    if base.contains(Permissions::ADMINISTRATOR) {
        return Permissions::all();
    }

    let mut sorted_overwrites = role_overwrites.to_vec();
    sorted_overwrites.sort_by_key(|(position, _)| *position);
    for (_, overwrite) in sorted_overwrites {
        base &= !overwrite.deny;
        base |= overwrite.allow;
    }

    if let Some(overwrite) = member_overwrite {
        base &= !overwrite.deny;
        base |= overwrite.allow;
    }

    base
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn administrator_short_circuits_to_all_permissions() {
        let roles = vec![Role {
            position: 0,
            permissions: Permissions::ADMINISTRATOR,
        }];
        let resolved = resolve(&roles, &[], None);
        assert_eq!(resolved, Permissions::all());
    }

    #[test]
    fn higher_position_role_overwrite_wins_tie() {
        let roles = vec![Role {
            position: 0,
            permissions: Permissions::VIEW_CHANNEL,
        }];
        let overwrites = vec![
            (
                0,
                Overwrite {
                    allow: Permissions::empty(),
                    deny: Permissions::SEND_MESSAGES,
                },
            ),
            (
                1,
                Overwrite {
                    allow: Permissions::SEND_MESSAGES,
                    deny: Permissions::empty(),
                },
            ),
        ];
        let resolved = resolve(&roles, &overwrites, None);
        assert!(resolved.contains(Permissions::SEND_MESSAGES));
    }

    #[test]
    fn member_specific_overwrite_wins_over_role_overwrite() {
        let roles = vec![Role {
            position: 0,
            permissions: Permissions::SEND_MESSAGES,
        }];
        let role_overwrites = vec![];
        let member_overwrite = Overwrite {
            allow: Permissions::empty(),
            deny: Permissions::SEND_MESSAGES,
        };
        let resolved = resolve(&roles, &role_overwrites, Some(&member_overwrite));
        assert!(!resolved.contains(Permissions::SEND_MESSAGES));
    }
}
