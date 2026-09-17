//! Canary anchor — the 32-byte integrity heartbeat for neuromorphic
//! firmware.
//!
//! COST MODEL (why this exists): a full 48-tick sentinel run anchors a
//! whole verdict; a 1-tick challenge-response canary anchors "the fabric
//! under my feet still computes like the reference" in ONE tick. On the
//! frozen dynamics that is a single SHA-256 fold — 32 bytes of evidence
//! for ~1 tick of work. Cheap enough to pin per boot, per slot-table swap,
//! per suspected cosmic-ray event.
//!
//! SELF-VALIDATING CHAIN: the same log path that yields the 1-tick canary
//! must, after 48 ticks, land exactly on the externally pinned sentinel
//! golden (```398e126a…``` for the all-ones digest). The test below asserts
//! both ends, so the cheap endpoint is anchored to the expensive contract
//! inside this crate itself — not to any consumer's trust.
//!
//! Bit-exact mirror of `scripts/expansion_check.py` [canary].

use crate::connectome::Connectome;
use crate::oracle::sentinel_stimulus;
use crate::sim::run_log;

/// 1-tick challenge-response anchor: digest-derived stimulus, one folded
/// tick of the canonical 48-tick chain.
#[must_use]
pub fn challenge_canary(conn: &Connectome, digest: &[u8; 32]) -> [u8; 32] {
    let stim = sentinel_stimulus(conn, digest);
    let log = run_log(conn, 1, &stim);
    if log.is_empty() {
        return [0u8; 32];
    }
    log[0]
}

/// Full-chain endpoint of the same log (the expensive contract the canary
/// is pinned to). Kept as a fn so a consumer can sample-check: the two
/// endpoints must always agree with the manifest.
#[must_use]
pub fn chain_end(conn: &Connectome, digest: &[u8; 32]) -> [u8; 32] {
    let stim = sentinel_stimulus(conn, digest);
    let log = run_log(conn, crate::oracle::SENTINEL_TICKS, &stim);
    if log.len() < crate::oracle::SENTINEL_TICKS as usize {
        return [0u8; 32];
    }
    log[crate::oracle::SENTINEL_TICKS as usize - 1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectome::{generate, DEFAULT_SEED};
    use crate::sha256::hex32;

    #[test]
    fn canary_endpoints_are_frozen_and_self_consistent() {
        let c = generate(1, 16, DEFAULT_SEED);
        let digest = [0xffu8; 32];
        assert_eq!(
            hex32(&challenge_canary(&c, &digest)),
            "6c221c70ef0a411e5ceaafc8c31109fec44b21684585e31bd4bea1ae2f11db62"
        );
        // the cheap endpoint is anchored to the expensive contract:
        assert_eq!(
            hex32(&chain_end(&c, &digest)),
            "398e126a88776b4f15f7da9bfb7bd927f47fc32e8a92128a354380ce8d429cc3",
            "log path must land on the pinned sentinel golden"
        );
    }
}
