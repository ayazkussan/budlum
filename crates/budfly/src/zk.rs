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

/// The index of the gate that arrests `row`, if any (0=G0, 1=G1, 2=G2,
/// 3=G3). Mirrors `air::violates` EXACTLY: a FAILED genesis check skips
/// the state gates; a genesis row that passes falls through.
fn gate_index(prev: Option<&TraceRow>, row: &TraceRow) -> Option<usize> {
    if row.tick == 0 {
        if row.v_before != 0 || row.r_before != 0 {
            return Some(0); // G0 violated
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
            return Some(3);
        }
    }
    if row.r_before > 0 {
        // G1: refractory hold
        return (row.v_after != 0 || row.spike != 0).then_some(1);
    }
    // G2: integrate, with the selectors materialized
    let leaked = row.v_before - (row.v_before >> LEAK_SHIFT);
    let raw = (leaked + row.i_ext + row.i_syn).clamp(V_MIN, V_MAX);
    let sel_above = u8::from(raw >= V_TH);
    if row.spike != sel_above {
        return Some(2);
    }
    if sel_above == 1 {
        (row.v_after != 0).then_some(2)
    } else {
        (row.v_after != raw).then_some(2)
    }
}

/// One gate's verdict for one row: violation found or clean.
fn gate_bad(prev: Option<&TraceRow>, row: &TraceRow) -> bool {
    gate_index(prev, row).is_some()
}

/// Bytes per witness row: five 4-byte fields plus three selector/spike bytes.
pub const WITNESS_STRIDE: usize = 23;

/// Stone 4 column count (adds `raw`, the pre-threshold integrator value).
pub const COLUMNS4: usize = 9;

/// Number of polynomial identities.
pub const IDENTS: usize = 4;

/// The AIR as four polynomial identities, one integer evaluation per
/// identity per row, compressed to a per-row bit mask (`1 << g` iff
/// identity `g` evaluated nonzero) plus the per-identity nonzero histogram:
///   R0 genesis:   gen·(vb² + rb²)
///   R1 hold:      sel_r·(va² + sp²)
///   R2 integrate: (1−sel_r)·((sp−sel_a)² + (va−(1−sel_a)·raw)²)
///   R3 chain:     (1−gen)·(rb − chain_exp(prev))²
/// Every identity is a sum of squares — zero iff the underlying gate holds.
#[must_use]
pub fn ident_evals(rows: &[Vec<TraceRow>]) -> (Vec<u8>, [usize; 4]) {
    let mut mask = Vec::with_capacity(rows.iter().map(Vec::len).sum::<usize>());
    let mut nonzeros = [0usize; 4];
    for neuron_rows in rows {
        for (i, row) in neuron_rows.iter().enumerate() {
            let vb = i64::from(row.v_before);
            let raw_unclamped = i64::from(
                row.v_before - (row.v_before >> LEAK_SHIFT) + row.i_ext + row.i_syn,
            );
            let raw = raw_unclamped.clamp(i64::from(V_MIN), i64::from(V_MAX));
            let rb = i64::from(row.r_before);
            let sp = i64::from(row.spike);
            let va = i64::from(row.v_after);
            let sel_r = i64::from(u8::from(row.r_before > 0));
            let sel_a = i64::from(u8::from(row.r_before == 0 && raw >= i64::from(V_TH)));
            let gen = i64::from(u8::from(row.tick == 0));
            let r0 = gen * (vb * vb + rb * rb);
            let r1 = sel_r * (va * va + sp * sp);
            let mut r3 = 0i64;
            if row.tick != 0 && i > 0 {
                let p = &neuron_rows[i - 1];
                let exp = if p.r_before > 0 {
                    i64::from(p.r_before - 1)
                } else if p.spike == 1 {
                    i64::from(REFRAC)
                } else {
                    0
                };
                let d = rb - exp;
                r3 = d * d;
            }
            let d_sp = sp - sel_a;
            let d_va = va - (1 - sel_a) * raw;
            let r2 = (1 - sel_r) * (d_sp * d_sp + d_va * d_va);
            let mut bits = 0u8;
            for (g, r) in [r0, r1, r2, r3].iter().enumerate() {
                if *r != 0 {
                    nonzeros[g] += 1;
                    bits |= 1 << g;
                }
            }
            mask.push(bits);
        }
    }
    (mask, nonzeros)
}


/// Gate-residual table: one byte per trace row (`1 << g` iff gate `g`
/// arrested that row, else 0) plus the per-gate hit histogram. The honest
/// matrix is all zero BY PIN; the forged matrix's hit count must equal the
/// catalogue's violation count.
#[must_use]
pub fn gate_residuals(rows: &[Vec<TraceRow>]) -> (Vec<u8>, [usize; 4]) {
    let mut mask = Vec::with_capacity(rows.iter().map(Vec::len).sum::<usize>());
    let mut hits = [0usize; 4];
    for neuron_rows in rows {
        for (i, row) in neuron_rows.iter().enumerate() {
            let prev = if i == 0 { None } else { neuron_rows.get(i - 1) };
            match gate_index(prev, row) {
                Some(g) => {
                    mask.push(1u8 << g);
                    hits[g] += 1;
                }
                None => mask.push(0),
            }
        }
    }
    (mask, hits)
}

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
        assert!(
            count_violations(&liar0) > 0,
            "genesis fraud is not invisible"
        );
    }

    #[test]
    fn the_fixture_witness_is_frozen_and_selectors_are_nonvacuous() {
        let c = generate(1, 16, DEFAULT_SEED);
        let stim = sentinel_stimulus(&c, &[0xffu8; 32]);
        let audited = run(&c, SENTINEL_TICKS, &stim, true);
        let rows = audited.rows.clone().unwrap_or_default();
        // non-vacuity probe over the bump-era window (ticks 7..=8), built
        // before `rows` is moved into the forged world:
        let probe: Vec<Vec<TraceRow>> =
            rows.iter().map(|nr| nr[7..=8].to_vec()).collect::<Vec<_>>();
        let honest = windows(rows.clone(), false);
        let forged = windows(rows, true);
        assert_eq!(
            crate::sha256::hex32(&crate::sha256::sha256(&witness_bytes(&honest))),
            "bb23c67e50eb155e744b5a1df4a2009a54c210436d41d6af29d3ab6f91b37fc2"
        );
        assert_eq!(
            sel_counts(&honest),
            (0, 0),
            "the quiet tail has no selectors on"
        );
        assert_ne!(
            crate::sha256::sha256(&witness_bytes(&forged)),
            crate::sha256::sha256(&witness_bytes(&honest)),
            "the liar's witness binds differently"
        );
        assert_eq!(
            sel_counts(&probe),
            (37, 26),
            "selectors fire when the bump does"
        );
    }

    #[test]
    fn the_residual_matrix_names_the_arresting_gate() {
        let c = generate(1, 16, DEFAULT_SEED);
        let stim = sentinel_stimulus(&c, &[0xffu8; 32]);
        let audited = run(&c, SENTINEL_TICKS, &stim, true);
        let rows = audited.rows.clone().unwrap_or_default();
        let honest = windows(rows.clone(), false);
        let forged = windows(rows, true);
        let (m0, h0) = gate_residuals(&honest);
        assert!(
            m0.iter().all(|b| *b == 0),
            "the honest residual matrix is all zero"
        );
        assert_eq!(h0, [0, 0, 0, 0]);
        let (m1, h1) = gate_residuals(&forged);
        assert_eq!(
            h1,
            [0, 0, 192, 0],
            "the integration gate G2 arrested every forged row"
        );
        // one hit per violated row, and the sum is the catalogue's count:
        assert_eq!(h1.iter().sum::<usize>(), 192);
        assert_eq!(h1.iter().sum::<usize>(), count_violations(&forged));
        assert_eq!(h1.iter().sum::<usize>(), skeleton_violations(&forged));
        assert_eq!(
            crate::sha256::hex32(&crate::sha256::sha256(&m1)),
            "4f437d300ce039c0bf8e815d6f31d52895f04ce78507ad7e25f3e42df185f436"
        );
    }

    #[test]
    fn the_polynomial_identities_arrest_the_same_rows_as_the_tree() {
        assert_eq!(COLUMNS4, 9);
        assert_eq!(IDENTS, 4);
        let c = generate(1, 16, DEFAULT_SEED);
        let stim = sentinel_stimulus(&c, &[0xffu8; 32]);
        let audited = run(&c, SENTINEL_TICKS, &stim, true);
        let rows = audited.rows.clone().unwrap_or_default();
        let honest = windows(rows.clone(), false);
        let forged = windows(rows, true);
        let (m0, n0) = ident_evals(&honest);
        assert!(m0.iter().all(|b| *b == 0), "honest identities all vanish");
        assert_eq!(n0, [0, 0, 0, 0]);
        let (m1, n1) = ident_evals(&forged);
        assert_eq!(n1, [0, 0, 192, 0], "all 192 arrests are R2 (integrate)");
        assert_eq!(n1[2], count_violations(&forged));
        // the polynomial table arrests the SAME rows as the decision tree,
        // byte for byte — not merely the same count:
        let (tree_mask, _) = gate_residuals(&forged);
        assert_eq!(m1, tree_mask);
        assert_eq!(
            crate::sha256::hex32(&crate::sha256::sha256(&m1)),
            "4f437d300ce039c0bf8e815d6f31d52895f04ce78507ad7e25f3e42df185f436"
        );
    }
}
