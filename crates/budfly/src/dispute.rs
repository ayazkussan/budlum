//! Anchor-bisection dispute game — the fraud-proof over fly execution.
//!
//! An executor claims a run happened and submits per-tick anchor chain
//! heads. A sentinel (validator without voting rights, RoleId 10 per
//! `docs/BUDFLY_SENTINEL_VALIDATOR.md`) reruns the honest protocol, locates
//! the FIRST divergent tick, and the C1–C7 transition constraints arbitrate
//! that single tick. Determinism is what makes this cheap: `48` ticks need
//! at most `ceil(log2 48) = 6` logged anchors to agree, and exactly one tick
//! ever has to be re-audited.
//!
//! Bit-exact mirror of `scripts/expansion_check.py` [dispute].

use crate::connectome::{Connectome, Region};
use crate::sha256::sha256;
use crate::sim::{Stimuli, FAN_IN_MAX, LEAK_SHIFT, REFRAC, V_MAX, V_MIN, V_TH};

/// First tick where two anchor logs disagree (None if identical).
/// Full-log scan: with the logs in hand the walk is O(T); the bisection
/// bound [`bisect_queries_max`] governs how many anchors the two parties
/// ever have to EXCHANGE to get the same answer remotely.
#[must_use]
pub fn find_divergence(a: &[[u8; 32]], b: &[[u8; 32]]) -> Option<usize> {
    a.iter().zip(b).position(|(x, y)| x != y)
}

/// Maximum anchor exchanges to locate divergence remotely: ceil(log2 T).
#[must_use]
pub const fn bisect_queries_max(ticks: u32) -> u32 {
    32 - ticks.saturating_sub(1).leading_zeros()
}

/// The frozen adversarial fixture — the "lazy liar": dynamics replayed
/// honestly, but at `tamper` the liar flips the spike bit of
/// (CxEb offset 3) and reports all-zero membrane bytes for that tick; the
/// chain folds onwards. Yields a self-consistent but dishonest anchor log
/// for testing the game end-to-end. Not production: an adversary model,
/// kept in-crate so the honest side and the threat model version together.
#[doc(hidden)]
#[must_use]
pub fn dishonest_log_fixture(
    conn: &Connectome,
    ticks: u32,
    stim: &Stimuli,
    tamper: u32,
) -> Vec<[u8; 32]> {
    let total = conn.total;
    let mut v = vec![0i32; total];
    let mut refr = vec![0u32; total];
    let mut pend = vec![0i32; total];
    let mut anchor = [0u8; 32];
    let tam_gid = conn.offset(Region::CxEb) + 3;
    let mut log = Vec::with_capacity(ticks as usize);
    for t in 0..ticks {
        let mut spike_bytes = vec![0u8; total];
        let mut spiking: Vec<u32> = Vec::new();
        for gid in 0..total {
            let i_ext = stim.current(gid as u32, t);
            let i_syn = pend[gid];
            let vb = v[gid];
            let rb = refr[gid];
            if rb > 0 {
                refr[gid] = rb - 1;
                v[gid] = 0;
            } else {
                let raw = (vb - (vb >> LEAK_SHIFT) + i_ext + i_syn).clamp(V_MIN, V_MAX);
                if raw >= V_TH {
                    v[gid] = 0;
                    refr[gid] = REFRAC;
                    spike_bytes[gid] = 1;
                    spiking.push(gid as u32);
                } else {
                    v[gid] = raw;
                }
            }
        }
        let mut next = vec![0i32; total];
        for &pre in &spiking {
            for e in &conn.adj[pre as usize] {
                let q = e.post as usize;
                next[q] = (next[q] + e.w).clamp(-FAN_IN_MAX, FAN_IN_MAX);
            }
        }
        pend = next;
        let v_hash = if t == tamper {
            spike_bytes[tam_gid] = 1 - spike_bytes[tam_gid];
            sha256(&vec![0u8; total * 4])
        } else {
            let mut v_bytes = Vec::with_capacity(total * 4);
            for x in &v {
                v_bytes.extend_from_slice(&x.to_le_bytes());
            }
            sha256(&v_bytes)
        };
        let spike_hash = sha256(&spike_bytes);
        let mut msg = Vec::with_capacity(104);
        msg.extend_from_slice(&anchor);
        msg.extend_from_slice(&u64::from(t).to_be_bytes());
        msg.extend_from_slice(&spike_hash);
        msg.extend_from_slice(&v_hash);
        anchor = sha256(&msg);
        log.push(anchor);
    }
    log
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectome::{generate, DEFAULT_SEED};
    use crate::lesion::odor_stim;
    use crate::sha256::hex32;
    use crate::sim::run_log;

    #[test]
    fn dispute_game_finds_the_lie_at_tick_23() {
        let c = generate(1, 16, DEFAULT_SEED);
        let stim = odor_stim(&c);
        let honest = run_log(&c, 48, &stim);
        assert_eq!(
            hex32(&honest[47]),
            "f3d639395fce7e866b3f564d52a90b22c104faa7325351626a3ad08faa51d8e8"
        );
        let liar = dishonest_log_fixture(&c, 48, &stim, 23);
        assert_eq!(
            hex32(&liar[47]),
            "f38c4daa094052394adc9a53253c9a1de6cf0cb58af9a6a55161463907e178e4"
        );
        assert_eq!(find_divergence(&honest, &liar), Some(23));
        assert_eq!(find_divergence(&honest, &honest), None);
        assert_eq!(bisect_queries_max(48), 6);
        // Arbitration: the honest trace at the disputed region satisfies
        // every C1-C7 transition constraint; a fabricated row cannot.
        let audited = crate::sim::run_masked(&c, 48, &stim, &vec![false; c.total]);
        let rows = audited.rows.unwrap_or_default();
        assert_eq!(crate::air::count_violations(&rows), 0);
    }
}
