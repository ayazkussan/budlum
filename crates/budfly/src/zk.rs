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
    // EXACT air.rs decision tree: a FAILED genesis check counts and skips;
    // a genesis row that PASSES falls through to the state gates.
    if row.tick == 0 {
        if row.v_before != 0 || row.r_before != 0 {
            return true; // G0 violated
        }
    } else if let Some(p) = prev {
        // G3: refractory chain (off-genesis, when a previous row exists)
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

/// Bytes per witness row: five 4-byte fields plus three selector/spike bytes.
pub const WITNESS_STRIDE: usize = 23;

/// The arithmetized WITNESS of a trace: per row, the eight columns
/// `(v_before, i_ext, i_syn, v_after, r_before, spike, sel_refrac,
/// sel_above)` little-endian, row-major. `sel_above = 1` iff `rb == 0` and
/// `clamp(leak + i_ext + i_syn) >= V_TH` (frozen rule). Field arithmetic on
/// a circuit side only ever sees this table.
#[must_use]
pub fn witness_bytes(rows: &[Vec<TraceRow>]) -> Vec<u8> {
    let mut out = Vec::with_capacity(rows.len() * 2 * WITNESS_STRIDE);
    for neuron_rows in rows {
        for row in neuron_rows {
            out.extend_from_slice(&row.v_before.to_le_bytes());
            out.extend_from_slice(&row.i_ext.to_le_bytes());
            out.extend_from_slice(&row.i_syn.to_le_bytes());
            out.extend_from_slice(&row.v_after.to_le_bytes());
            out.extend_from_slice(&row.r_before.to_le_bytes());
            let leaked = row.v_before - (row.v_before >> LEAK_SHIFT);
            let raw = (leaked + row.i_ext + row.i_syn).clamp(V_MIN, V_MAX);
            out.push(row.spike);
            out.push(u8::from(row.r_before > 0));
            out.push(u8::from(row.r_before == 0 && raw >= V_TH));
        }
    }
    out
}

/// Selector column sums over a trace: `(sel_refrac ones, sel_above ones)`.
#[must_use]
pub fn sel_counts(rows: &[Vec<TraceRow>]) -> (usize, usize) {
    let mut refr = 0;
    let mut above = 0;
    for neuron_rows in rows {
        for row in neuron_rows {
            let leaked = row.v_before - (row.v_before >> LEAK_SHIFT);
            let raw = (leaked + row.i_ext + row.i_syn).clamp(V_MIN, V_MAX);
            if row.r_before > 0 {
                refr += 1;
            } else if raw >= V_TH {
                above += 1;
            }
        }
    }
    (refr, above)
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

    fn tamper_at(rows: &mut [Vec<TraceRow>], k: usize, tam: usize) {
        for (g, nr) in rows.iter_mut().enumerate() {
            nr[k].v_after = 0;
            if g == tam {
                nr[k].spike = 1 - nr[k].spike;
            }
        }
    }

    #[test]
    fn the_skeleton_tracks_the_catalogue_at_genesis_too() {
        let c = generate(1, 16, DEFAULT_SEED);
        let stim = sentinel_stimulus(&c, &[0xffu8; 32]);
        let audited = run(&c, SENTINEL_TICKS, &stim, true);
        let rows = audited.rows.clone().unwrap_or_default();
        // full-trace equality (includes the genesis rows that once broke
        // the mirror): both evaluators count the same on any trace:
        assert_eq!(skeleton_violations(&rows), count_violations(&rows));
        let mut liar0 = rows.clone();
        tamper_at(&mut liar0, 0, c.offset(Region::CxEb) + 3);
        assert_eq!(skeleton_violations(&liar0), count_violations(&liar0));
        assert!(count_violations(&liar0) > 0, "genesis fraud is not invisible");
    }

    #[test]
    fn the_fixture_witness_is_frozen_and_selectors_are_nonvacuous() {
        let c = generate(1, 16, DEFAULT_SEED);
        let stim = sentinel_stimulus(&c, &[0xffu8; 32]);
        let audited = run(&c, SENTINEL_TICKS, &stim, true);
        let rows = audited.rows.clone().unwrap_or_default();
        let honest = windows(rows.clone(), false);
        let forged = windows(rows, true);
        assert_eq!(
            crate::sha256::hex32(&crate::sha256::sha256(&witness_bytes(&honest))),
            "bb23c67e50eb155e744b5a1df4a2009a54c210436d41d6af29d3ab6f91b37fc2"
        );
        assert_eq!(sel_counts(&honest), (0, 0), "the quiet tail has no selectors on");
        assert_ne!(
            crate::sha256::sha256(&witness_bytes(&forged)),
            crate::sha256::sha256(&witness_bytes(&honest)),
            "the liar's witness binds differently"
        );
        // non-vacuity probe over the bump-era window (ticks 7..=8):
        let probe: Vec<Vec<TraceRow>> = rows
            .iter()
            .map(|nr| nr[7..=8].to_vec())
            .collect::<Vec<_>>();
        assert_eq!(sel_counts(&probe), (37, 26), "selectors fire when the bump does");
    }
}
