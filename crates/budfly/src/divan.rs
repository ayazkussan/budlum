//! The Fly Court — a three-juror quorum over one fact digest.
//!
//! Each juror is a seat-*salted* probe: the seat byte is folded into a fresh
//! digest (`sha256(digest || seat)`), never into a second draw of the base
//! RNG stream, so the draw-order discipline survives: one builder per
//! (builder, seat), each pinned.
//!
//! COUNCIL RULE (fails closed): anything short of unanimity settles as
//! `Abstain`. Three independent probes that disagree about a fact are the
//! settlement layer's definition of "not established".
//!
//! EXPULSION: a juror that submits a lazy chain (the frozen fixture from
//! `dispute.rs`) is found by the bisection game and expelled — the honest
//! rerun arbitrates, and its verdict is what the seat SHOULD have said.
//!
//! Bit-exact mirror of `scripts/expansion_check.py` [divan].

use crate::connectome::Connectome;
use crate::oracle::{sentinel_verdict, Verdict};
use crate::sha256::sha256;

/// Number of jurors in a council (frozen).
pub const SEATS: usize = 3;

/// Seat-salted fact digest: `sha256(digest || seat)`.
#[must_use]
pub fn seat_digest(digest: &[u8; 32], seat: u8) -> [u8; 32] {
    let mut buf = [0u8; 33];
    buf[..32].copy_from_slice(digest);
    buf[32] = seat;
    sha256(&buf)
}

/// What a council commits to.
#[derive(Clone, Debug)]
pub struct CouncilReport {
    /// Per-seat sentinel anchors.
    pub anchors: [[u8; 32]; SEATS],
    /// Per-seat sentinel verdicts.
    pub verdicts: [Verdict; SEATS],
    /// Whether all seats agree.
    pub unanimous: bool,
    /// The settled verdict: the unanimous one, else `Abstain`.
    pub council: Verdict,
}

/// Runs the three-seat council over `digest`.
#[must_use]
pub fn council(conn: &Connectome, digest: &[u8; 32]) -> CouncilReport {
    let mut anchors = [[0u8; 32]; SEATS];
    let mut verdicts = [Verdict::Abstain; SEATS];
    for (s, (a, v)) in anchors.iter_mut().zip(verdicts.iter_mut()).enumerate() {
        let r = sentinel_verdict(conn, &seat_digest(digest, s as u8));
        *a = r.anchor;
        *v = r.verdict;
    }
    let unanimous = verdicts[0] == verdicts[1] && verdicts[1] == verdicts[2];
    CouncilReport {
        anchors,
        verdicts,
        unanimous,
        council: if unanimous {
            verdicts[0]
        } else {
            Verdict::Abstain
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectome::{generate, DEFAULT_SEED};
    use crate::dispute::{dishonest_log_fixture, find_divergence};
    use crate::oracle::{sentinel_stimulus, SENTINEL_TICKS};
    use crate::sha256::hex32;
    use crate::sim::run_log;

    #[test]
    fn council_of_seat_salted_probes_is_frozen() {
        let c = generate(1, 16, DEFAULT_SEED);
        let rep = council(&c, &[0x42u8; 32]);
        assert_eq!(
            hex32(&rep.anchors[0]),
            "ae3aa8d0afc6a7db9a9bb222b8c6917aefad1dc32dea271da827be696112e5ed"
        );
        assert_eq!(
            hex32(&rep.anchors[1]),
            "6b14cece457976715767da61b2cb48ef8ff27cb06b2fb9840887aa2c67a1d1fb"
        );
        assert_eq!(
            hex32(&rep.anchors[2]),
            "5400ce9c2b1f00a8159c178b618fc363f3ccddcb9bab3f22ce2c50f6d40612ec"
        );
        assert_eq!(
            rep.verdicts,
            [Verdict::Abstain, Verdict::Affirm, Verdict::Abstain]
        );
        assert!(!rep.unanimous);
        assert_eq!(rep.council, Verdict::Abstain, "non-unanimous fails closed");
    }

    #[test]
    fn a_lazy_juror_is_found_and_expelled() {
        let c = generate(1, 16, DEFAULT_SEED);
        let stim = sentinel_stimulus(&c, &seat_digest(&[0x42u8; 32], 2));
        let honest = run_log(&c, SENTINEL_TICKS, &stim);
        let liar = dishonest_log_fixture(&c, SENTINEL_TICKS, &stim, 23);
        assert_eq!(find_divergence(&honest, &liar), Some(23));
        assert_eq!(
            hex32(honest.last().unwrap_or(&[0u8; 32])),
            "5400ce9c2b1f00a8159c178b618fc363f3ccddcb9bab3f22ce2c50f6d40612ec",
            "the arbitration rerun reseals seat 2's published anchor"
        );
        assert_eq!(
            hex32(liar.last().unwrap_or(&[0u8; 32])),
            "f690c9384e793d80af7e0f2327764d358b7814aff87bb5bcc4eb98823da456e7"
        );
    }
}
