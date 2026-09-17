//! Tournament — two seeds, same ring, same panel, judged by the court.
//!
//! METRIC (honest, pinned): survival = *decisiveness*. For each digest of a
//! frozen 8-digest panel (`sha256(b"budfly-panel" || i)`, `i in 0..8`) each
//! fly runs its own three-seat council; a council that fails to reach
//! unanimity counts one abstention. The fly with fewer abstentions survives;
//! a tie spares both. An abstention machine is settlement dead weight — the
//! layer pays anchors for verdicts it can publish.
//!
//! The frozen upset: challenger seed `0xB0DF18` beats the reference
//! `0xB0DF17` 5–7. That does not make it a better brain — it makes it the
//! more decisive council on THIS panel, which is the only claim the pins
//! carry.
//!
//! Bit-exact mirror of `scripts/expansion_check.py` [tournament].

use crate::connectome::Connectome;
use crate::divan::council;
use crate::sha256::sha256;

/// Frozen panel size (digests `0..PANEL_SIZE` of the panel stream).
pub const PANEL_SIZE: usize = 8;

/// Outcome of a match.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Survivor {
    /// The first fly abstained less.
    First,
    /// The second fly abstained less.
    Second,
    /// Equal decisiveness: both survive.
    Both,
}

/// What a match commits to.
#[derive(Clone, Debug)]
pub struct MatchReport {
    /// Council non-unanimity count of the first fly over the panel.
    pub abstains_a: u32,
    /// Council non-unanimity count of the second fly over the panel.
    pub abstains_b: u32,
    /// Who survived (fewer abstentions).
    pub survivor: Survivor,
}

/// `i`-th panel digest: `sha256(b"budfly-panel" || i)`.
#[must_use]
pub fn panel_digest(i: u8) -> [u8; 32] {
    let mut buf = [0u8; 13];
    buf[..12].copy_from_slice(b"budfly-panel");
    buf[12] = i;
    sha256(&buf)
}

/// Non-unanimous council count of `fly` over the frozen panel.
#[must_use]
pub fn panel_abstains(fly: &Connectome) -> u32 {
    let mut n = 0;
    for i in 0..PANEL_SIZE as u8 {
        if !council(fly, &panel_digest(i)).unanimous {
            n += 1;
        }
    }
    n
}

/// Runs the match.
#[must_use]
pub fn tournament(a: &Connectome, b: &Connectome) -> MatchReport {
    let abstains_a = panel_abstains(a);
    let abstains_b = panel_abstains(b);
    MatchReport {
        abstains_a,
        abstains_b,
        survivor: match abstains_a.cmp(&abstains_b) {
            std::cmp::Ordering::Less => Survivor::First,
            std::cmp::Ordering::Greater => Survivor::Second,
            std::cmp::Ordering::Equal => Survivor::Both,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectome::{generate, DEFAULT_SEED};

    /// Second seed of the frozen match (challenger wiring, same anatomy).
    const CHALLENGER_SEED: u64 = 0xB0DF18;

    #[test]
    fn the_challenger_outlives_the_reference_on_decisiveness() {
        let a = generate(1, 16, DEFAULT_SEED);
        let b = generate(1, 16, CHALLENGER_SEED);
        assert_eq!((a.total, a.edges.len()), (b.total, b.edges.len()));
        let rep = tournament(&a, &b);
        assert_eq!(rep.abstains_a, 7);
        assert_eq!(rep.abstains_b, 5);
        assert_eq!(rep.survivor, Survivor::Second);
    }
}
