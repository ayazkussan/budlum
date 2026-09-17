//! Compass wander probe — the pinned honest negative about the EB ring.
//!
//! Bit-exact mirror of `scripts/expansion_check.py` [compass v4]. Protocol:
//! a 6-wide bump at sector 0 for ticks 0..8, then 64 ticks of silence; per
//! tick decode the dominant 1/16-ring sector (ties resolve to the lowest
//! sector index). Measured truth: the bump does NOT hold its sector (single-
//! sector lock is absent at the frozen parameters) — the ring needs stronger
//! CX inhibition gain before "heading memory" can be claimed, so the crate
//! pins the wander instead of claiming the attractor.

use crate::connectome::{Connectome, Region};
use crate::sim::{run_masked, Stimuli};

/// Result of the wander probe.
pub struct WanderReport {
    /// Run anchor (72 ticks).
    pub anchor: [u8; 32],
    /// Ticks in 8..72 whose dominant sector is sector 0 (persistence count).
    pub hold_sector0: usize,
    /// Ticks in 8..72 with any EB spike.
    pub any_spike: usize,
    /// Sorted (sector, count) histogram of EB spikes at tick 40.
    pub t40_histogram: Vec<(usize, usize)>,
    /// Decoded dominant sector per tick (-1 = silent), length 72.
    pub decoded: Vec<i32>,
}

/// Runs the frozen wander protocol.
#[must_use]
pub fn wander_probe(conn: &Connectome) -> WanderReport {
    const TICKS: u32 = 72;
    let bump_width = 6usize;
    let eb_off = conn.offset(Region::CxEb);
    let ne = conn.size(Region::CxEb);
    let sw = ne / 16;
    let mut stim = Stimuli::new();
    for i in 0..bump_width {
        stim.set((eb_off + i) as u32, 0, 8);
    }
    let dead = vec![false; conn.total];
    let r = run_masked(conn, TICKS, &stim, &dead);
    let rows = r.rows.unwrap_or_default();

    // per-tick sector counts over EB only
    let mut per_tick: Vec<[usize; 16]> = vec![[0usize; 16]; TICKS as usize];
    for (g, rs) in rows.iter().enumerate() {
        if g >= eb_off && g < eb_off + ne {
            let b = (g - eb_off) / sw;
            for row in rs {
                if row.spike == 1 {
                    per_tick[row.tick as usize][b] += 1;
                }
            }
        }
    }
    let mut decoded = Vec::with_capacity(TICKS as usize);
    for counts in &per_tick {
        let best = counts.iter().copied().max().unwrap_or(0);
        if best == 0 {
            decoded.push(-1);
        } else {
            let idx = counts.iter().position(|&c| c == best).unwrap_or(0);
            decoded.push(idx as i32);
        }
    }
    let after = &decoded[8..];
    let hold0 = after.iter().filter(|&&d| d == 0).count();
    let anysp = after.iter().filter(|&&d| d >= 0).count();
    let mut t40: Vec<(usize, usize)> = Vec::new();
    for (b, &c) in per_tick[40].iter().enumerate() {
        if c > 0 {
            t40.push((b, c));
        }
    }
    WanderReport {
        anchor: r.anchor,
        hold_sector0: hold0,
        any_spike: anysp,
        t40_histogram: t40,
        decoded,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectome::{generate, DEFAULT_SEED};
    use crate::sha256::hex32;

    #[test]
    fn wander_is_pinned_as_measured() {
        let c = generate(1, 16, DEFAULT_SEED);
        let w = wander_probe(&c);
        assert_eq!(
            hex32(&w.anchor),
            "438a58bf5c4869d355d810fb0954ce1e786002fd9e17521fb5cbd5a2496dc299"
        );
        assert_eq!(
            w.hold_sector0, 0,
            "the bump must NOT lock sector 0 (honest negative)"
        );
        assert_eq!(w.any_spike, 64);
        assert_eq!(
            w.t40_histogram,
            vec![
                (0, 2),
                (1, 2),
                (3, 3),
                (4, 1),
                (5, 2),
                (11, 1),
                (12, 2),
                (13, 1),
                (14, 3)
            ]
        );
    }
}
