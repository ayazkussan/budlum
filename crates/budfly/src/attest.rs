//! Attestation — the neural identity handshake: proof-of-exact-build.
//!
//! A peer shows it executes THIS frozen connectome through THIS frozen code
//! path by answering a challenge that is bound to `(peer_id, epoch, nonce)`:
//!
//! ```text
//! challenge = sha256("BUDFLY-ATTEST1\x00" || peer_id || epoch be64 || nonce)
//! ```
//!
//! Three rungs of confidence, fixed cost each:
//!
//! | rung | evidence | work |
//! |------|----------|------|
//! | L1 | 1-tick canary anchor | 1 tick (~32 B proof) |
//! | L2 | 48-tick chain end | full dynamics |
//! | L3 | court council verdict | 3 seats x 48 ticks |
//!
//! Rungs are not independent claims: the pinned property is
//! `L1 == L2 chain head at tick 1` (ladder self-consistency).
//!
//! HONEST SCOPE — read before pointing this at anything: this is a
//! *behavioral build-fingerprint* for closed validator sets. Anyone holding
//! the frozen connectome can replay a rung; the map IS the asset being
//! attested. It is not a TEE, not a VDF, not an identity proof in the open
//! world (see `docs/BUDFLY_APPLICATIONS.md`, rejected rows).
//!
//! Bit-exact mirror of `scripts/expansion_check.py` [attest].

use crate::canary::challenge_canary;
use crate::connectome::Connectome;
use crate::divan::{council, CouncilReport};
use crate::oracle::{sentinel_stimulus, SENTINEL_TICKS};
use crate::sha256::sha256;
use crate::sim::run_log;

/// Domain separator of the attestation challenge (frozen).
pub const DOMAIN: &[u8] = b"BUDFLY-ATTEST1\x00";

/// Builds the challenge digest for `(peer_id, epoch, nonce)`.
#[must_use]
pub fn challenge(peer_id: &[u8], epoch: u64, nonce: &[u8; 32]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(DOMAIN.len() + peer_id.len() + 8 + 32);
    buf.extend_from_slice(DOMAIN);
    buf.extend_from_slice(peer_id);
    buf.extend_from_slice(&epoch.to_be_bytes());
    buf.extend_from_slice(nonce);
    sha256(&buf)
}

/// L1: the 1-tick canary anchor over the challenge.
#[must_use]
pub fn attest_l1(conn: &Connectome, digest: &[u8; 32]) -> [u8; 32] {
    challenge_canary(conn, digest)
}

/// L2: the full 48-tick chain end over the same challenge. The caller's
/// cheap consistency: `attest_l1` output must equal the L2 chain's first
/// head (see [`ladder_consistent`]).
#[must_use]
pub fn attest_l2(conn: &Connectome, digest: &[u8; 32]) -> [u8; 32] {
    let stim = sentinel_stimulus(conn, digest);
    let log = run_log(conn, SENTINEL_TICKS, &stim);
    *log.last().unwrap_or(&[0u8; 32])
}

/// L3: the court council verdict over the challenge.
#[must_use]
pub fn attest_l3(conn: &Connectome, digest: &[u8; 32]) -> CouncilReport {
    council(conn, digest)
}

/// The ladder's internal spine: L1 must be the first head of the L2 chain.
#[must_use]
pub fn ladder_consistent(conn: &Connectome, digest: &[u8; 32]) -> bool {
    let stim = sentinel_stimulus(conn, digest);
    let log = run_log(conn, SENTINEL_TICKS, &stim);
    log.first() == Some(&attest_l1(conn, digest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectome::{generate, DEFAULT_SEED};
    use crate::oracle::Verdict;
    use crate::sha256::hex32;

    fn frozen_challenge() -> [u8; 32] {
        challenge(b"fly-peer-01", 7, &[0xa5u8; 32])
    }

    #[test]
    fn the_ladder_is_frozen_and_self_consistent() {
        let c = generate(1, 16, DEFAULT_SEED);
        let ch = frozen_challenge();
        assert_eq!(
            hex32(&ch),
            "4c707cec26a238c9a256fb06269db732783227b9917e91349b3ceb338fec33cb"
        );
        assert_eq!(
            hex32(&attest_l1(&c, &ch)),
            "1ead39dfb6c14294dbb9bc996350ec86d666a956c5e35c17e8fb118f1a085894"
        );
        assert_eq!(
            hex32(&attest_l2(&c, &ch)),
            "d5f675bc1c7d4167ab4bc1ccae7079ea153f1c5967fcbf5fd2fee98a34fa4c45"
        );
        assert!(ladder_consistent(&c, &ch), "L1 must head the L2 chain");
        let rep = attest_l3(&c, &ch);
        assert_eq!(rep.council, Verdict::Abstain, "4,4,0 tie fails closed");
    }

    #[test]
    fn epoch_freshness_moves_the_answer() {
        let c = generate(1, 16, DEFAULT_SEED);
        let stale = attest_l2(&c, &frozen_challenge());
        let fresh = attest_l2(&c, &challenge(b"fly-peer-01", 8, &[0xa5u8; 32]));
        assert_ne!(stale, fresh, "a replayed epoch never validates fresh");
    }
}
