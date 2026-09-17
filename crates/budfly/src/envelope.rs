//! BSE-1 — the BudFly Settlement Envelope: the 136-byte court verdict.
//!
//! This is the ONE artifact a settlement layer stores instead of re-running
//! anything. Everything the council commits to fits in a fixed-size record:
//!
//! ```text
//!   "BSE1"         4 B   magic
//!   fact_digest   32 B   the challenged fact
//!   seat anchors  96 B   3 x 32 B per-tick chain finals
//!   seat verdicts  3 B   0 = Abstain, 1 = Affirm, 2 = Reject
//!   council        1 B   settled verdict (rule: unanimity else Abstain)
//! ```
//!
//! CHEAP vs EXPENSIVE honesty: bytes cannot be re-derived without running
//! the fly — the envelope is a COMMITMENT. What any consumer CAN check for
//! free is [`cheap_consistent`]: magic, length, and the council rule
//! re-applied to the seat codes. A malformed or self-contradicting verdict
//! is rejected without a single neuron firing. Expensive checks (anchor
//! re-execution, bisection) remain one dispute away, as designed.
//!
//! Bit-exact mirror of `scripts/expansion_check.py` [envelope].

use crate::divan::{CouncilReport, SEATS};
use crate::oracle::Verdict;

/// Envelope size in bytes (frozen).
pub const ENVELOPE_LEN: usize = 136;
/// Magic prefix (frozen).
pub const MAGIC: [u8; 4] = *b"BSE1";

/// Wire code for a verdict byte (fails closed: Abstain is zero).
#[must_use]
pub const fn verdict_code(v: Verdict) -> u8 {
    match v {
        Verdict::Abstain => 0,
        Verdict::Affirm => 1,
        Verdict::Reject => 2,
    }
}

/// Decodes a verdict byte; anything unknown maps to `Abstain` (fails closed).
#[must_use]
pub const fn code_verdict(b: u8) -> Verdict {
    match b {
        1 => Verdict::Affirm,
        2 => Verdict::Reject,
        _ => Verdict::Abstain,
    }
}

/// A decoded BSE-1 record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Envelope {
    /// The challenged fact digest.
    pub digest: [u8; 32],
    /// Per-seat chain finals.
    pub seat_anchors: [[u8; 32]; SEATS],
    /// Per-seat verdicts.
    pub seat_verdicts: [Verdict; SEATS],
    /// Settled council verdict.
    pub council: Verdict,
}

/// Encodes a council report to the frozen 136-byte wire form.
#[must_use]
pub fn encode_council(digest: &[u8; 32], rep: &CouncilReport) -> [u8; ENVELOPE_LEN] {
    let mut out = [0u8; ENVELOPE_LEN];
    out[..4].copy_from_slice(&MAGIC);
    out[4..36].copy_from_slice(digest);
    for (i, a) in rep.anchors.iter().enumerate() {
        let s = 36 + i * 32;
        out[s..s + 32].copy_from_slice(a);
    }
    for (i, v) in rep.verdicts.iter().enumerate() {
        out[132 + i] = verdict_code(*v);
    }
    out[135] = verdict_code(rep.council);
    out
}

/// Decodes a byte slice into an envelope, or `None` if malformed.
#[must_use]
pub fn decode(bytes: &[u8]) -> Option<Envelope> {
    if bytes.len() != ENVELOPE_LEN || bytes[..4] != MAGIC {
        return None;
    }
    let mut digest = [0u8; 32];
    digest.copy_from_slice(&bytes[4..36]);
    let mut seat_anchors = [[0u8; 32]; SEATS];
    for (i, a) in seat_anchors.iter_mut().enumerate() {
        let s = 36 + i * 32;
        a.copy_from_slice(&bytes[s..s + 32]);
    }
    let seat_verdicts = [
        code_verdict(bytes[132]),
        code_verdict(bytes[133]),
        code_verdict(bytes[134]),
    ];
    Some(Envelope {
        digest,
        seat_anchors,
        seat_verdicts,
        council: code_verdict(bytes[135]),
    })
}

/// The free check: the council byte must be what the unanimity rule yields
/// over the seat verdicts. No fly execution, no hashing — a forged or
/// corrupted record that contradicts its own seats dies here.
#[must_use]
pub fn cheap_consistent(env: &Envelope) -> bool {
    let unanimous =
        env.seat_verdicts[0] == env.seat_verdicts[1] && env.seat_verdicts[1] == env.seat_verdicts[2];
    env.council
        == (if unanimous {
            env.seat_verdicts[0]
        } else {
            Verdict::Abstain
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectome::{generate, DEFAULT_SEED};
    use crate::divan::council;
    use crate::sha256::hex32;

    const FROZEN_HEX: &str = "425345314242424242424242424242424242424242424242424242424242424242424242ae3aa8d0afc6a7db9a9bb222b8c6917aefad1dc32dea271da827be696112e5ed6b14cece457976715767da61b2cb48ef8ff27cb06b2fb9840887aa2c67a1d1fb5400ce9c2b1f00a8159c178b618fc363f3ccddcb9bab3f22ce2c50f6d40612ec00010000";

    #[test]
    fn bse1_encode_is_a_frozen_byte_stream() {
        let c = generate(1, 16, DEFAULT_SEED);
        let rep = council(&c, &[0x42u8; 32]);
        let env = encode_council(&[0x42u8; 32], &rep);
        assert_eq!(env.len(), ENVELOPE_LEN);
        assert_eq!(hex32(&env[..32]), &FROZEN_HEX[..64]);
        let full: String = env.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(full, FROZEN_HEX);
    }

    #[test]
    fn decode_roundtrips_and_cheap_check_holds() {
        let c = generate(1, 16, DEFAULT_SEED);
        let rep = council(&c, &[0x42u8; 32]);
        let bytes = encode_council(&[0x42u8; 32], &rep);
        let env = decode(&bytes).expect("frozen bytes must decode");
        assert!(cheap_consistent(&env));
        assert_eq!(env.council, Verdict::Abstain);
        assert_eq!(env.digest, [0x42u8; 32]);
        // corruption that breaks self-consistency is rejected for free:
        let mut forged = env.clone();
        forged.council = Verdict::Affirm;
        assert!(!cheap_consistent(&forged));
        assert!(decode(&bytes[..100]).is_none());
    }
}
