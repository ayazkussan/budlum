//! The settlement sentinel: a spike-coded, hash-anchored readout over a
//! 32-byte fact digest.
//!
//! Modelled on FlyCoder's closed loop — sensory encoder onto visual/compass
//! populations, descending-neuron readout — with the difference that the
//! product here is not a decision but a *provable transcript*:
//!
//! 1. A 32-byte digest (e.g. a domain finality payload hash) deterministically
//!    expands to a stimulus: a heading sector on the compass ring plus a bit
//!    pattern on the lobula.
//! 2. The frozen connectome executes 48 ticks; every tick folds into the
//!    anchor chain.
//! 3. Descending pools (DNa02-like L/R, MDN veto) yield a spike-count verdict.
//!
//! **What this is not:** the verdict's *semantics* are an exploratory
//! encoding — nobody claims fly buckets understand finality. What is real is
//! the anchor: a bit-exact, cross-checked commitment that `digest` was run
//! through pinned configuration `C` and produced this transcript. That is the
//! object a settlement layer can record as a fact (and what fails closed:
//! no anchor, no record). See `docs/BUDFLY.md`.

use crate::connectome::{Connectome, Region};
use crate::rng::SplitMix64;
use crate::sim::{run, Stimuli};

/// Sentinel run length (frozen; golden-pinned).
pub const SENTINEL_TICKS: u32 = 48;

/// Descending readout verdict.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Verdict {
    /// Left command pool out-spiked right.
    Affirm,
    /// Right command pool out-spiked left.
    Reject,
    /// MDN veto fired, or a tie (fails closed).
    Abstain,
}

/// What the sentinel commits to.
#[derive(Clone, Debug)]
pub struct SentinelReport {
    /// Descending verdict.
    pub verdict: Verdict,
    /// DNa02-like left spikes.
    pub dn_l: u64,
    /// DNa02-like right spikes.
    pub dn_r: u64,
    /// MDN-like veto spikes.
    pub mdn: u64,
    /// Stimulated sensory neurons.
    pub stim_neurons: usize,
    /// Hash-chain head over the 48-tick transcript.
    pub anchor: [u8; 32],
}

/// Runs the sentinel over `digest` on `conn`.
///
/// Stimulus expansion (frozen, golden-pinned):
/// * RNG seed = `digest[0..8]` little-endian;
/// * compass sector start = `u16(digest[8..10]) mod ring`;
/// * lobula: bits 0..32 of `digest[16..24]`, left population first, each set
///   bit draws one neuron from the seeded stream.
/// The frozen digest-derived challenge stimulus (also used by
/// `canary::challenge_canary` — one builder, one draw order, never two).
#[must_use]
pub fn sentinel_stimulus(conn: &Connectome, digest: &[u8; 32]) -> Stimuli {
    let mut rng = SplitMix64::new(u64::from_le_bytes([
        digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7],
    ]));
    let eb_off = conn.offset(Region::CxEb);
    let n_eb = conn.size(Region::CxEb);
    let sector_start = usize::from(u16::from_le_bytes([digest[8], digest[9]])) % n_eb;
    let width = (n_eb / 4).max(4);

    let mut stim = Stimuli::new();
    for i in 0..width {
        let gid = (eb_off + (sector_start + i) % n_eb) as u32;
        stim.set(gid, 0, 8);
    }
    // lobula tickle: deterministic feature bits, left side first
    for lob in [Region::LobL, Region::LobR] {
        let base = conn.offset(lob);
        let n = conn.size(lob);
        for b in 0..32usize {
            if (digest[16 + b / 8] >> (b % 8)) & 1 == 1 {
                let gid = (base + rng.below(n as u32) as usize) as u32;
                stim.set_if_absent(gid, 0, 4);
            }
        }
    }
    stim
}

pub fn sentinel_verdict(conn: &Connectome, digest: &[u8; 32]) -> SentinelReport {
    let stim = sentinel_stimulus(conn, digest);

    let result = run(conn, SENTINEL_TICKS, &stim, false);
    let dn_l = result.region_spikes[Region::DnL.idx()];
    let dn_r = result.region_spikes[Region::DnR.idx()];
    let mdn = result.region_spikes[Region::Mdn.idx()];
    let verdict = if mdn > 0 && mdn > 3 * dn_l.max(dn_r) {
        Verdict::Abstain
    } else if dn_l > dn_r {
        Verdict::Affirm
    } else if dn_r > dn_l {
        Verdict::Reject
    } else {
        Verdict::Abstain
    };
    SentinelReport {
        verdict,
        dn_l,
        dn_r,
        mdn,
        stim_neurons: stim.len(),
        anchor: result.anchor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectome::{generate, DEFAULT_SEED};

    #[test]
    fn sentinel_is_deterministic() {
        let c = generate(1, 16, DEFAULT_SEED);
        let d = [42u8; 32];
        let a = sentinel_verdict(&c, &d);
        let b = sentinel_verdict(&c, &d);
        assert_eq!(a.anchor, b.anchor);
        assert_eq!(a.verdict, b.verdict);
        assert_eq!((a.dn_l, a.dn_r, a.mdn), (b.dn_l, b.dn_r, b.mdn));
    }

    /// Pinned by goldens.anchor.toml [tamper] and validated by
    /// scripts/reference_check.py: one bit in a USED digest byte moves the
    /// 48-tick anchor (avalanche), while a byte in the documented UNBOUND
    /// range (20..32) must not. The unbound scope is a published property,
    /// never silently assumed by consumers.
    #[test]
    fn one_bit_avalanche_and_binding_scope() {
        let c = generate(1, 16, DEFAULT_SEED);
        let d = [0xffu8; 32];
        let a = sentinel_verdict(&c, &d);
        let mut used = d;
        used[19] ^= 1; // last bound lobula byte
        let mut offscope = d;
        offscope[31] ^= 1; // documented unbound range byte
        assert_ne!(
            a.anchor,
            sentinel_verdict(&c, &used).anchor,
            "bound byte must avalanche"
        );
        assert_eq!(
            a.anchor,
            sentinel_verdict(&c, &offscope).anchor,
            "byte 31 is documented as unbound"
        );
        assert!(a.stim_neurons > 0, "sentinel stimulus must be non-empty");
    }

    #[test]
    fn abstain_fails_closed_on_ties() {
        // The tie/veto rule is the settlement layer's fail-closed stance in
        // miniature: ambiguity never resolves to an affirmative.
        let c = generate(1, 16, DEFAULT_SEED);
        let d = [0u8; 32];
        let r = sentinel_verdict(&c, &d);
        if r.dn_l == r.dn_r || (r.mdn > 0 && r.mdn > 3 * r.dn_l.max(r.dn_r)) {
            assert_eq!(r.verdict, Verdict::Abstain);
        }
    }
}
