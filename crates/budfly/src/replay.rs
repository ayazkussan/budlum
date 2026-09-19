//! Replay certificate — sealed counterfactual branches over the frozen log.
//!
//! SPINE: the base chain heads are published; a counterfactual branch only
//! replaces the *stimulus* from `fork` onward — the dynamics underneath are
//! the same frozen connectome, so the first `fork` heads of the branch MUST
//! byte-match the published base prefix. That gives a settlement layer a
//! two-tier check: prefix binding is a cheap byte compare (no execution),
//! while the single divergent suffix is re-executable under the dispute
//! rules if anyone claims fraud.
//!
//! Frozen question (pinned in the test): *"what if MDN had fired at tick
//! 10?"* — the honest negative finding: the all-ones digest's `Affirm` is
//! robust to an early MDN double-pulse (dynamics move, the verdict does
//! not).
//!
//! Bit-exact mirror of `scripts/expansion_check.py` [replay].

use crate::connectome::{Connectome, Region};
use crate::oracle::{verdict_rule, Verdict};
use crate::sim::{run, run_log, Stimuli};

/// What a replay certificate commits to.
#[derive(Clone, Debug)]
pub struct ReplayCert {
    /// Tick at which the branch stimulus takes over.
    pub fork: u32,
    /// Published base head at `fork - 1` (branch prefix must end here too).
    pub base_head_at_fork: [u8; 32],
    /// Branch chain final head (the counterfactual's sealed outcome).
    pub branch_final: [u8; 32],
    /// Number of prefix heads compared (`fork`).
    pub prefix_ticks: u32,
    /// Whether the branch prefix byte-matched the base prefix.
    pub prefix_bound: bool,
    /// Descending counters on the branch (DnL, DnR, MDN).
    pub branch_counters: (u64, u64, u64),
    /// Verdict rule applied to the branch counters.
    pub branch_verdict: Verdict,
    /// Whether the plain `run()` anchor re-sealed the branch log end
    /// (the cheap runner and the log runner must agree).
    pub run_resealed: bool,
}

/// Builds the replay certificate: base heads, branch heads, prefix binding.
#[must_use]
pub fn replay_cert(
    conn: &Connectome,
    base: &Stimuli,
    branch: &Stimuli,
    fork: u32,
    ticks: u32,
) -> ReplayCert {
    let base_log = run_log(conn, ticks, base);
    let branch_log = run_log(conn, ticks, branch);
    let f = (fork as usize).min(ticks as usize);
    let prefix_bound = base_log[..f] == branch_log[..f];
    let base_head_at_fork = if f == 0 { [0u8; 32] } else { base_log[f - 1] };
    let branch_final = *branch_log.last().unwrap_or(&[0u8; 32]);
    let r = run(conn, ticks, branch, false);
    let dl = r.region_spikes[Region::DnL.idx()];
    let dr = r.region_spikes[Region::DnR.idx()];
    let mdn = r.region_spikes[Region::Mdn.idx()];
    ReplayCert {
        fork,
        base_head_at_fork,
        branch_final,
        prefix_ticks: fork,
        prefix_bound,
        branch_counters: (dl, dr, mdn),
        branch_verdict: verdict_rule(dl, dr, mdn),
        run_resealed: r.anchor == branch_final,
    }
}

/// The frozen "MDN arrives at `fork`" counterfactual: base stimulus plus a
/// veto double-pulse on the whole MDN pool for two ticks (`fork..fork+2`).
/// Base sentinel windows end by tick 8, so any `fork >= 10` starts clean.
#[must_use]
pub fn mdn_early_stim(conn: &Connectome, base: &Stimuli, fork: u32) -> Stimuli {
    let mut s = base.clone();
    let off = conn.offset(Region::Mdn) as u32;
    for i in 0..conn.size(Region::Mdn) as u32 {
        s.set(off + i, fork, fork + 2);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectome::{generate, DEFAULT_SEED};
    use crate::oracle::{sentinel_stimulus, SENTINEL_TICKS};
    use crate::sha256::hex32;

    #[test]
    fn replay_certificate_is_frozen() {
        let c = generate(1, 16, DEFAULT_SEED);
        let base = sentinel_stimulus(&c, &[0xffu8; 32]);
        let branch = mdn_early_stim(&c, &base, 10);
        let cert = replay_cert(&c, &base, &branch, 10, SENTINEL_TICKS);
        assert_eq!(
            hex32(&cert.base_head_at_fork),
            "090600fb1189fe4e74805461610ba01b18a9f4db6b125c6b0f174a9df613d69a"
        );
        assert_eq!(
            hex32(&cert.branch_final),
            "62cfbd0f6769cf555005034d4ee34d1229d32a9b0fe043c69df17a2f3df96148"
        );
        assert_eq!(cert.prefix_ticks, 10);
        assert!(cert.prefix_bound, "prefix must bind before the fork");
        assert!(cert.run_resealed, "run() must re-seal the branch log end");
        // the honest negative: an early MDN double-pulse moves the dynamics
        // (branch anchor differs) but does NOT flip the all-ones Affirm.
        assert_eq!(cert.branch_counters, (7, 6, 5));
        assert_eq!(cert.branch_verdict, Verdict::Affirm);
    }
}
