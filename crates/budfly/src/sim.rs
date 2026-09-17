//! Fixed-point leaky-integrate-and-fire (LIF) execution engine.
//!
//! Deliberate design choices, all in service of *provable* neuromorphic
//! execution:
//!
//! * **Integer-only Q4.12.** Every quantity is an `i32` in units of 1/4096.
//!   The transition function uses shifts, adds and compares — exactly the
//!   operators an AIR for a STARK (BudZero) must arithmetize anyway. No
//!   floats anywhere near the state transition.
//! * **One-tick global synaptic delay.** Spikes emitted at tick `t` are
//!   accumulated and delivered at tick `t+1`. Trace rows become purely
//!   local functions of `(v, r)` and per-tick input sums: a row checks
//!   against its immediate predecessor with no routing state in between.
//! * **Saturating accumulation with frozen bounds.** Fan-in per target is
//!   clamped to ±64.0 as it accumulates, deterministic order = ascending
//!   presynaptic id, then generation order within each row.
//! * **Hash-chained anchors.** After every tick we fold
//!   `SHA-256(anchor‖tick‖spike_bitmap_hash‖membrane_hash)` — the same
//!   commitment a settlement layer can record as a fact of execution.
//!
//! Dynamics constants are frozen (validated by `scripts/reference_check.py`
//! reproducing the golden anchors bit-for-bit).

use crate::connectome::Connectome;
use crate::sha256::sha256;
use std::collections::HashMap;

/// Fixed-point scale: 1.0 in Q4.12.
pub const SCALE: i32 = 1 << 12;
/// Spike threshold (1.0).
pub const V_TH: i32 = SCALE;
/// Membrane clamp ceiling (16.0).
pub const V_MAX: i32 = SCALE * 16;
/// Membrane clamp floor (-8.0).
pub const V_MIN: i32 = -SCALE * 8;
/// Refractory period in ticks.
pub const REFRAC: u32 = 1;
/// Weight quantum per unit of synapse count (1/8).
pub const W_SYN: i32 = 512;
/// Stimulus current injected into stimulated neurons (4.0).
pub const I_STIM: i32 = SCALE * 4;
/// Leak shift: `v -= v >> 4` per tick (~ 15/16 retention).
pub const LEAK_SHIFT: u32 = 4;
/// Fan-in clamp per target per tick (±64.0), engaged during delivery.
pub const FAN_IN_MAX: i32 = SCALE * 64;

/// External stimulus: per-neuron `[start, end)` tick windows at [`I_STIM`].
#[derive(Default)]
pub struct Stimuli {
    windows: HashMap<u32, (u32, u32)>,
}

impl Stimuli {
    /// Empty stimulus set.
    #[must_use]
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
        }
    }

    /// Sets/replaces the window for `gid`.
    pub fn set(&mut self, gid: u32, start: u32, end: u32) {
        self.windows.insert(gid, (start, end));
    }

    /// Sets the window for `gid` only if none exists (first write wins;
    /// mirrors the oracle encoder's semantics).
    pub fn set_if_absent(&mut self, gid: u32, start: u32, end: u32) {
        self.windows.entry(gid).or_insert((start, end));
    }

    /// Number of stimulated neurons.
    #[must_use]
    pub fn len(&self) -> usize {
        self.windows.len()
    }

    /// True when no neuron carries a window.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.windows.is_empty()
    }

    /// Injected current for `gid` at tick `t` (0 or [`I_STIM`]).
    #[must_use]
    pub fn current(&self, gid: u32, t: u32) -> i32 {
        match self.windows.get(&gid) {
            Some(&(s, e)) if s <= t && t < e => I_STIM,
            _ => 0,
        }
    }
}

/// One audited transition row for one neuron at one tick.
#[derive(Clone, Copy, Debug)]
pub struct TraceRow {
    /// Tick index.
    pub tick: u32,
    /// Membrane potential at row start (Q4.12).
    pub v_before: i32,
    /// External current applied this tick.
    pub i_ext: i32,
    /// Synaptic current delivered this tick (computed from t-1 spikes).
    pub i_syn: i32,
    /// Membrane potential after the transition.
    pub v_after: i32,
    /// 1 if the neuron emitted a spike this tick.
    pub spike: u8,
    /// Refractory counter at row start.
    pub r_before: u32,
}

/// Result of a simulation run.
pub struct RunResult {
    /// Final anchor (hash chain head).
    pub anchor: [u8; 32],
    /// Spikes per region across the run.
    pub region_spikes: [u64; 16],
    /// Audited rows per neuron (only when `audit` was requested).
    pub rows: Option<Vec<Vec<TraceRow>>>,
    /// Per-tick anchor chain heads (only when `collect_log` was requested).
    /// `log[t]` is the chain head after folding tick `t` — exactly what a
    /// dispute game bisects.
    pub log: Option<Vec<[u8; 32]>>,
}

/// Runs `ticks` LIF ticks over `conn` under `stim`.
///
/// The transition order — gid-ascending neuron updates, then fan-in delivery
/// in (pre ascending, generation) order — is frozen and golden-pinned.
pub fn run(conn: &Connectome, ticks: u32, stim: &Stimuli, audit: bool) -> RunResult {
    run_core(conn, ticks, stim, audit, None, false)
}

/// Lesion harness: `dead` gids never update nor fire, and no current flows
/// in or out of them. Identical dynamics to [`run`] otherwise; the anchor
/// shape is unchanged (dead neurons contribute zero rows and zero bits).
/// Bit-exact mirror of `run_masked` in `scripts/expansion_check.py`.
#[must_use]
pub fn run_masked(conn: &Connectome, ticks: u32, stim: &Stimuli, dead: &[bool]) -> RunResult {
    run_core(conn, ticks, stim, true, Some(dead), false)
}

/// Honest per-tick anchor log ([`run`] semantics, chain heads collected).
/// The verifier side of the dispute game lives on this vector.
#[must_use]
pub fn run_log(conn: &Connectome, ticks: u32, stim: &Stimuli) -> Vec<[u8; 32]> {
    run_core(conn, ticks, stim, false, None, true).log.unwrap_or_default()
}

#[allow(clippy::too_many_arguments)]
fn run_core(
    conn: &Connectome,
    ticks: u32,
    stim: &Stimuli,
    audit: bool,
    dead: Option<&[bool]>,
    collect_log: bool,
) -> RunResult {
    let total = conn.total;
    let mut v = vec![0i32; total];
    let mut refr = vec![0u32; total];
    let mut pend = vec![0i32; total]; // delivered input (from previous tick)
    let mut anchor = [0u8; 32];
    let mut region_spikes = [0u64; 16];
    let spike_bytes_len = total;
    let mut rows: Option<Vec<Vec<TraceRow>>> = if audit {
        Some(vec![Vec::new(); total])
    } else {
        None
    };
    let mut log: Option<Vec<[u8; 32]>> = if collect_log {
        Some(Vec::with_capacity(ticks as usize))
    } else {
        None
    };

    for t in 0..ticks {
        let mut spike_bytes = vec![0u8; spike_bytes_len];
        let mut spiking: Vec<u32> = Vec::new();

        for gid in 0..total {
            if let Some(dm) = dead {
                if dm[gid] {
                    continue;
                }
            }
            let i_ext = stim.current(gid as u32, t);
            let i_syn = pend[gid];
            let vb = v[gid];
            let rb = refr[gid];
            let mut spike = 0u8;
            if rb > 0 {
                refr[gid] = rb - 1;
                v[gid] = 0;
            } else {
                let leaked = vb - (vb >> LEAK_SHIFT);
                let raw = (leaked + i_ext + i_syn).clamp(V_MIN, V_MAX);
                if raw >= V_TH {
                    spike = 1;
                    v[gid] = 0;
                    refr[gid] = REFRAC;
                } else {
                    v[gid] = raw;
                }
            }
            if spike == 1 {
                spiking.push(gid as u32);
                spike_bytes[gid] = 1;
            }
            if let Some(r) = rows.as_mut() {
                r[gid].push(TraceRow {
                    tick: t,
                    v_before: vb,
                    i_ext,
                    i_syn,
                    v_after: v[gid],
                    spike,
                    r_before: rb,
                });
            }
        }

        // Delivery for tick t+1: ascending pre id, row order = generation order.
        let mut next = vec![0i32; total];
        for &pre in &spiking {
            for e in &conn.adj[pre as usize] {
                let q = e.post as usize;
                if let Some(dm) = dead {
                    if dm[q] {
                        continue;
                    }
                }
                next[q] = (next[q] + e.w).clamp(-FAN_IN_MAX, FAN_IN_MAX);
            }
        }
        for gid in spiking {
            region_spikes[conn.region_of(gid as usize).idx()] += 1;
        }
        pend = next;

        // Anchor fold: anchor || tick_be8 || sha(spike bitmap) || sha(v le bytes)
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
        if let Some(l) = log.as_mut() {
            l.push(anchor);
        }
    }

    RunResult {
        anchor,
        region_spikes,
        rows,
        log,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectome::{generate, Region, DEFAULT_SEED};

    #[test]
    fn silent_network_stays_silent() {
        let c = generate(1, 16, DEFAULT_SEED);
        let r = run(&c, 16, &Stimuli::new(), true);
        assert!(r.region_spikes.iter().all(|&s| s == 0));
    }

    #[test]
    fn stimulated_eb_spikes() {
        let c = generate(1, 16, DEFAULT_SEED);
        let off = c.offset(Region::CxEb) as u32;
        let mut st = Stimuli::new();
        for i in 0..4 {
            st.set(off + i, 0, 4);
        }
        let r = run(&c, 16, &st, false);
        assert!(r.region_spikes[Region::CxEb.idx()] > 0);
    }
}
