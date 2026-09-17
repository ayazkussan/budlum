//! BudZero bridge, stone 1: the arithmetization SKELETON, executable.
//!
//! What a future STARK proves about one BudFly tick is *exactly* what
//! `air.rs` checks today — this module makes the shape of that statement
//! explicit, and RUNS it:
//!
//! | item | value | note |
//! |------|-------|------|
//! | trace columns per row | 8 | `vb, i_ext, i_syn, va, rb, sp, sel_refrac, sel_above` |
//! | gates | 4 | `G0 genesis, G1 refractory-hold, G2 integrate, G3 chain` |
//! | max constraint degree | 3 | selector-multiplied; threshold/clamp need bit-decomposition notes |
//!
//! Gate equation sets:
//!
//! ```text
//! G0 (tick == 0):  vb == 0 ; rb == 0
//! G1 (rb > 0):     va == 0 ; sp == 0
//! G2 (rb == 0):    sp == sel_above ; va == (sp ? 0 : clamp(vb - vb>>4 + ie + isy))
//! G3 (i > 0):      rb == (prev.rb > 0 ? prev.rb - 1 : (prev.sp == 1 ? REFRAC : 0))
//! ```
//!
//! THE PINNED CLAIM: the skeleton is arithmetization-faithful — its
//! per-gate evaluation follows the same decision tree as
//! `air::count_violations`, so its mismatch count equals the air violation
//! count on ANY trace (asserted below on the honest window: 0, and on the
//! lazy-liar window: 192). A circuit author never has to trust the prose:
//! the catalogue and the skeleton are one decision tree, run twice.
//!
//! Bit-exact mirror of `scripts/expansion_check.py` [zk].

use crate::sim::{TraceRow, LEAK_SHIFT, REFRAC, V_MAX, V_MIN, V_TH};

/// Trace columns per row (frozen).
pub const COLUMNS: usize = 8;
/// Gate families (frozen).
pub const GATES: usize = 4;
/// Max constraint degree after selector multiplication (frozen estimate;
/// threshold and clamp arithmetize through bit-decomposition notes).
pub const DEGREE_MAX: usize = 3;

/// One gate's verdict for one row: violation found or clean.
fn gate_bad(prev: Option<&TraceRow>, row: &TraceRow) -> bool {
    if row.tick == 0 {
        // G0: genesis
        return row.v_before != 0 || row.r_before != 0;
    }
    if let Some(p) = prev {
        // G3: refractory chain
        let expected = if p.r_before > 0 {
            p.r_before - 1
        } else if p.spike == 1 {
            REFRAC
        } else {
            0
        };
        if row.r_before != expected {
            return true;
        }
    }
    if row.r_before > 0 {
        // G1: refractory hold
        return row.v_after != 0 || row.spike != 0;
    }
    // G2: integrate, with the selectors materialized
    let leaked = row.v_before - (row.v_before >> LEAK_SHIFT);
    let raw = (leaked + row.i_ext + row.i_syn).clamp(V_MIN, V_MAX);
    let sel_above = u8::from(raw >= V_TH);
    if row.spike != sel_above {
        return true;
    }
    if sel_above == 1 {
        row.v_after != 0
    } else {
        row.v_after != raw
    }
}

/// Evaluates the skeleton over whole traces (per-neuron row sequences):
/// the number of rows whose gate-selected equation set is violated.
#[must_use]
pub fn skeleton_violations(rows: &[Vec<TraceRow>]) -> usize {
    let mut bad = 0usize;
    for neuron_rows in rows {
        for (i, row) in neuron_rows.iter().enumerate() {
            let prev = if i > 0 { neuron_rows.get(i - 1) } else { None };
            if gate_bad(prev, row) {
                bad += 1;
            }
        }
    }
    bad
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::air::count_violations;
    use crate::connectome::{generate, Region, DEFAULT_SEED};
    use crate::oracle::{sentinel_stimulus, SENTINEL_TICKS};
    use crate::sim::run;

    fn windows(mut rows: Vec<Vec<TraceRow>>, forge: bool) -> Vec<Vec<TraceRow>> {
        if forge {
            let c = generate(1, 16, DEFAULT_SEED);
            let tam = c.offset(Region::CxEb) + 3;
            for (g, nr) in rows.iter_mut().enumerate() {
                nr[23].v_after = 0;
                if g == tam {
                    nr[23].spike = 1 - nr[23].spike;
                }
            }
        }
        rows.iter()
            .map(|nr| nr[22..=23].to_vec())
            .collect::<Vec<_>>()
    }

    #[test]
    fn the_skeleton_reproduces_the_constraint_catalogue_exactly() {
        assert_eq!(COLUMNS, 8);
        assert_eq!(GATES, 4);
        assert_eq!(DEGREE_MAX, 3);
        let c = generate(1, 16, DEFAULT_SEED);
        let stim = sentinel_stimulus(&c, &[0xffu8; 32]);
        let audited = run(&c, SENTINEL_TICKS, &stim, true);
        let rows = audited.rows.clone().unwrap_or_default();
        assert_eq!(rows.len() * 2, 1176, "window row count of the fixture");

        let honest = windows(rows.clone(), false);
        let forged = windows(rows, true);
        assert_eq!(skeleton_violations(&honest), 0);
        assert_eq!(skeleton_violations(&forged), 192);
        // and the equality is with the catalogue itself, on both worlds:
        assert_eq!(skeleton_violations(&honest), count_violations(&honest));
        assert_eq!(skeleton_violations(&forged), count_violations(&forged));
    }
}
