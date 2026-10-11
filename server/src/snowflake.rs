//! A simple timestamp+counter unique ID generator for messages/channels.
//!
//! Layout (independently chosen, not Discord's literal epoch or bit split):
//! 42 bits milliseconds since the Unix epoch, 22 bits of per-process counter.
//! Single-node only for now — fine for a single-deployment-per-server model;
//! revisit if/when horizontal sharding within one deployment is ever needed.

use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

struct GeneratorState {
    last_millis: u64,
    counter: u64,
}

pub struct SnowflakeGenerator {
    state: Mutex<GeneratorState>,
}

impl SnowflakeGenerator {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(GeneratorState {
                last_millis: 0,
                counter: 0,
            }),
        }
    }

    pub fn next_id(&self) -> String {
        let now_millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before Unix epoch")
            .as_millis() as u64;

        let mut state = self.state.lock().expect("snowflake generator mutex poisoned");
        if now_millis != state.last_millis {
            state.last_millis = now_millis;
            state.counter = 0;
        } else {
            state.counter = (state.counter + 1) & 0x3F_FFFF;
        }

        let id = (state.last_millis << 22) | state.counter;
        id.to_string()
    }
}

impl Default for SnowflakeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_monotonically_increasing() {
        let gen = SnowflakeGenerator::new();
        let a: u128 = gen.next_id().parse().unwrap();
        let b: u128 = gen.next_id().parse().unwrap();
        assert!(b > a);
    }

    #[test]
    fn many_ids_in_the_same_millisecond_are_unique() {
        let gen = SnowflakeGenerator::new();
        let mut ids = std::collections::HashSet::new();
        for _ in 0..1000 {
            assert!(ids.insert(gen.next_id()));
        }
    }
}
