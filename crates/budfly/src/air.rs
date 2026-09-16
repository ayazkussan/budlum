//! AIR-shaped transition-constraint checker.
//!
//! "AIR-shaped", stated precisely: a STARK proves that an execution trace
//! satisfies a set of algebraic transition constraints (an Algebraic
//! Intermediate Representation). This module defines the constraint set the
//! BudZero arithmetization would encode for one BudFly tick, and evaluates
//! those predicates directly over an audited trace. What a future STARK is
//! asked to prove is *exactly* what is checked here — the checker is the
//! executable specification of the circuit.
//!
//! Constraint catalogue (per neuron, per tick row):
//!
//! | id | constraint |
//! |----|------------|
//! | C1 | leak: `leaked = v_before - (v_before >> 4)` |
//! | C2 | accumulate + clamp: `raw = clamp(leaked + i_ext + i_syn, V_MIN, V_MAX)` |
//! | C3 | threshold: `spike = 1 <=> raw >= V_TH` (when `r_before = 0`) |
//! | C4 | reset: `spike = 1 => v_after = 0` |
//! | C5 | hold: `r_before > 0 => v_after = 0 /\ spike = 0` |
//! | C6 | refractory bookkeeping: `r_after` matches decay/spike rule, chain-wide |
//! | C7 | genesis: row 0 starts from `(v, r) = (0, 0)` |

use crate::sim::{TraceRow, LEAK_SHIFT, REFRAC, V_MAX, V_MIN, V_TH};

/// Number of constraint violations across all audited neuron traces.
///
/// Returns 0 for an honest trace. Any tampering with a spike flag, a voltage
/// or a refractory counter breaks at least one constraint at the tampered row
/// or its successor (see `tests/scenarios.rs`).
#[must_use]
pub fn count_violations(rows: &[Vec<TraceRow>]) -> usize {
    let mut bad = 0usize;
    for neuron_rows in rows {
        for (i, row) in neuron_rows.iter().enumerate() {
            if i == 0 {
                // C7: genesis state
                if row.v_before != 0 || row.r_before != 0 {
                    bad += 1;
                    continue;
                }
            } else {
                // C6: r_before must equal the value the previous row committed to
                let prev = &neuron_rows[i - 1];
                let expected = if prev.r_before > 0 {
                    prev.r_before - 1
                } else if prev.spike == 1 {
                    REFRAC
                } else {
                    0
                };
                if row.r_before != expected {
                    bad += 1;
                    continue;
                }
            }
            if row.r_before > 0 {
                // C5: refractory hold
                if row.v_after != 0 || row.spike != 0 {
                    bad += 1;
                }
                continue;
            }
            // C1 + C2
            let leaked = row.v_before - (row.v_before >> LEAK_SHIFT);
            let raw = (leaked + row.i_ext + row.i_syn).clamp(V_MIN, V_MAX);
            // C3
            let expect_spike = u8::from(raw >= V_TH);
            if row.spike != expect_spike {
                bad += 1;
                continue;
            }
            // C4 / C2
            if expect_spike == 1 {
                if row.v_after != 0 {
                    bad += 1;
                }
            } else if row.v_after != raw {
                bad += 1;
            }
        }
    }
    bad
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectome::{generate, Region, DEFAULT_SEED};
    use crate::sim::{run, Stimuli};

    #[test]
    fn honest_trace_has_zero_violations() {
        let c = generate(1, 16, DEFAULT_SEED);
        let mut st = Stimuli::new();
        let off = c.offset(Region::CxEb) as u32;
        for i in 0..4 {
            st.set(off + i, 0, 4);
        }
        let r = run(&c, 32, &st, true);
        if let Some(rows) = &r.rows {
            assert_eq!(count_violations(rows), 0);
        } else {
            panic!("audit rows must exist");
        }
    }

    #[test]
    fn tampered_spike_flag_is_caught() {
        let c = generate(1, 16, DEFAULT_SEED);
        let mut st = Stimuli::new();
        let off = c.offset(Region::CxEb) as u32;
        st.set(off, 0, 8);
        let r = run(&c, 32, &st, true);
        if let Some(rows) = r.rows {
            let mut tampered = rows;
            // flip the spike flag of neuron 0 at tick 1
            let row = tampered[0][1];
            tampered[0][1] = TraceRow {
                spike: 1 - row.spike,
                ..row
            };
            assert!(count_violations(&tampered) > 0);
        } else {
            panic!("audit rows must exist");
        }
    }
}
