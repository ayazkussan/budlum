//! Mushroom-body conditioned suppression — the frozen learnable circuit.
//!
//! Bit-exact mirror of `scripts/expansion_check.py` [learn v3].
//!
//! Protocol (frozen): CS = every even KC stimulated ticks 8..24; US = the
//! MDN pool stimulated ticks 12..20. Three-factor trace gate per tick,
//! applied after spike computation and before delivery (weights evolve
//! online, delivery for t+1 uses UPDATED weights):
//!
//!   gate  = any MDN spiked at t or t-1   (1-tick US trace)
//!   hit   = KC->MBON edge, pre spiked at t-1, post spiked at t
//!   rule  = w <- max(FLOOR, w - DELTA)    (edge order = generation order)
//!
//! Physical honesty: a strict same-tick pre==post==gate rule can NEVER fire
//! in this delay-1 architecture (MBON spikes one tick after its KC drive,
//! exactly when KC is refractory — found and documented during the Python
//! reference phase). The 1-tick eligibility trace is the biologically and
//! architecturally correct rule, and it is the frozen one.

use crate::connectome::{Connectome, Region};
use crate::sha256::sha256;
use crate::sim::{Stimuli, FAN_IN_MAX, LEAK_SHIFT, REFRAC, SCALE, V_MAX, V_MIN, V_TH};

/// Conditioned-depression step (Q4.12 units), frozen.
pub const LEARN_DELTA: i32 = SCALE / 32;
/// Lower clamp for a depressed KC->MBON weight, frozen.
pub const LEARN_FLOOR: i32 = 128;
/// Protocol length.
pub const LEARN_TICKS: u32 = 32;

/// Report of one conditioning run.
pub struct LearnReport {
    /// Run anchor (32 ticks).
    pub run_anchor: [u8; 32],
    /// sha256("learn-v1" || final weights LE i32 in generation order).
    pub learn_anchor: [u8; 32],
    /// MBON spikes per tick.
    pub mbon_per_tick: Vec<u64>,
    /// KC->MBON edges whose final weight differs from the initial one.
    pub changed: usize,
    /// Minimum final KC->MBON weight (reaches the floor on strong pairs).
    pub floor_min: i32,
    /// Maximum final KC->MBON weight.
    pub ceil_max: i32,
    /// MBON spikes in ticks 8..12 (pre-US baseline).
    pub pre_mbon: u64,
    /// MBON spikes in ticks 20..24 (post-US probe).
    pub post_mbon: u64,
}

/// Frozen conditioning run over the canonical connectome.
#[must_use]
pub fn conditioned_run(conn: &Connectome) -> LearnReport {
    let total = conn.total;
    let mut stim = Stimuli::new();
    let k_off = conn.offset(Region::MbKc);
    for i in (0..conn.size(Region::MbKc)).step_by(2) {
        stim.set((k_off + i) as u32, 8, 24);
    }
    let m_off = conn.offset(Region::Mdn);
    for i in 0..conn.size(Region::Mdn) {
        stim.set((m_off + i) as u32, 12, 20);
    }

    // per-pre adjacency of edge indices, generation order
    let mut adj_idx: Vec<Vec<usize>> = vec![Vec::new(); total];
    let mut w: Vec<i32> = conn.edges.iter().map(|e| e.w).collect();
    for (i, e) in conn.edges.iter().enumerate() {
        adj_idx[e.pre as usize].push(i);
    }

    let mut v = vec![0i32; total];
    let mut refr = vec![0u32; total];
    let mut pend = vec![0i32; total];
    let mut anchor = [0u8; 32];
    let mut mbon_per_tick = Vec::with_capacity(LEARN_TICKS as usize);
    let mut prev_spiked = vec![false; total];

    for t in 0..LEARN_TICKS {
        let mut spike_bytes = vec![0u8; total];
        let mut spiking: Vec<usize> = Vec::new();
        let mut mbon = 0u64;
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
                    spiking.push(gid);
                    if conn.region_of(gid) == Region::MbMbon {
                        mbon += 1;
                    }
                } else {
                    v[gid] = raw;
                }
            }
        }
        mbon_per_tick.push(mbon);

        let mut cur_spiked = vec![false; total];
        for &g in &spiking {
            cur_spiked[g] = true;
        }
        let gate = spiking.iter().any(|&g| conn.region_of(g) == Region::Mdn)
            || (0..total).any(|g| prev_spiked[g] && conn.region_of(g) == Region::Mdn);
        if gate {
            for (i, e) in conn.edges.iter().enumerate() {
                if conn.region_of(e.pre as usize) == Region::MbKc
                    && conn.region_of(e.post as usize) == Region::MbMbon
                    && prev_spiked[e.pre as usize]
                    && cur_spiked[e.post as usize]
                {
                    w[i] = w[i].saturating_sub(LEARN_DELTA).max(LEARN_FLOOR);
                }
            }
        }

        let mut next = vec![0i32; total];
        for &p in &spiking {
            for &i in &adj_idx[p] {
                let e = &conn.edges[i];
                let q = e.post as usize;
                next[q] = (next[q] + w[i]).clamp(-FAN_IN_MAX, FAN_IN_MAX);
            }
        }
        pend = next;
        prev_spiked = cur_spiked;

        let spike_hash = sha256(&spike_bytes);
        let mut v_bytes = Vec::with_capacity(total * 4);
        for x in &v {
            v_bytes.extend_from_slice(&x.to_le_bytes());
        }
        let v_hash = sha256(&v_bytes);
        let mut msg = Vec::with_capacity(104);
        msg.extend_from_slice(&anchor);
        msg.extend_from_slice(&u64::from(t).to_be_bytes());
        msg.extend_from_slice(&spike_hash);
        msg.extend_from_slice(&v_hash);
        anchor = sha256(&msg);
    }

    let mut w_bytes = b"learn-v1".to_vec();
    for x in &w {
        w_bytes.extend_from_slice(&x.to_le_bytes());
    }
    let learn_anchor = sha256(&w_bytes);

    let mut changed = 0usize;
    let mut floor_min = i32::MAX;
    let mut ceil_max = i32::MIN;
    for (i, e) in conn.edges.iter().enumerate() {
        if conn.region_of(e.pre as usize) == Region::MbKc
            && conn.region_of(e.post as usize) == Region::MbMbon
        {
            if w[i] != e.w {
                changed += 1;
            }
            floor_min = floor_min.min(w[i]);
            ceil_max = ceil_max.max(w[i]);
        }
    }
    LearnReport {
        run_anchor: anchor,
        learn_anchor,
        pre_mbon: mbon_per_tick[8..12].iter().sum(),
        post_mbon: mbon_per_tick[20..24].iter().sum(),
        mbon_per_tick,
        changed,
        floor_min,
        ceil_max,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectome::{generate, DEFAULT_SEED};
    use crate::sha256::hex32;

    #[test]
    fn conditioned_suppression_is_frozen() {
        let c = generate(1, 16, DEFAULT_SEED);
        let r = conditioned_run(&c);
        eprintln!(
            "DBG changed={} floor={} ceil={} pre={} post={} mbon={:?} run={} learn={}",
            r.changed, r.floor_min, r.ceil_max, r.pre_mbon, r.post_mbon,
            r.mbon_per_tick,
            crate::sha256::hex32(&r.run_anchor),
            crate::sha256::hex32(&r.learn_anchor)
        );
        assert_eq!(r.changed, 63, "changed count");
        assert_eq!(r.floor_min, 128, "floor");
        assert_eq!(r.ceil_max, 1024, "ceil");
        assert_eq!(r.pre_mbon, 16, "pre");
        assert_eq!(r.post_mbon, 9, "post");
        // the behavioral claim: US-paired odor depresses MBON response
        assert!(r.post_mbon < r.pre_mbon);
    }
}
