//! Behavioural scenarios: the checks that turn "crazy" into "engineering".
//! Each mirrors a scenario in `scripts/reference_check.py`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use budfly::air::count_violations;
use budfly::connectome::{generate, Region, DEFAULT_SEED};
use budfly::fabric::{place, real_scale_report, Chip};
use budfly::oracle::sentinel_verdict;
use budfly::sim::{run, Stimuli};

use std::f64::consts::PI;

/// The emergent property: after the stimulus ends, a self-sustaining activity
/// bump must persist near the stimulated ring sector. This is the central
/// complex's famous ring-attractor behaviour, arising here from generator
/// structure alone — nobody hand-codes a bump.
#[test]
fn ring_attractor_bump_emerges() {
    let c = generate(1, 16, DEFAULT_SEED);
    let off = c.offset(Region::CxEb);
    let n = c.size(Region::CxEb);
    let mut stim = Stimuli::new();
    for i in 0..(n / 4) {
        stim.set((off + i) as u32, 0, 8);
    }
    let r = run(&c, 64, &stim, true);
    let rows = r.rows.as_ref().expect("audit rows");

    // Population vector over second-half spikes (test-side analysis may use
    // floats; the execution itself never does).
    let mut counts = vec![0u64; n];
    for (i, cnt) in counts.iter_mut().enumerate() {
        *cnt = rows[off + i][32..].iter().map(|row| u64::from(row.spike)).sum();
    }
    let total: u64 = counts.iter().sum();
    assert!(total > 0, "bump must fire");
    let mut sx = 0f64;
    let mut sy = 0f64;
    for (i, &c) in counts.iter().enumerate() {
        let th = 2.0 * PI * i as f64 / n as f64;
        sx += c as f64 * th.cos();
        sy += c as f64 * th.sin();
    }
    let ang = sy.atan2(sx).to_degrees().rem_euclid(360.0);
    let target = 360.0 * (n as f64 / 8.0) / n as f64;
    let err = (ang - target).abs().min((target - ang).abs()).min(360.0 - (ang - target).abs());
    assert!(
        err < 45.0,
        "bump at {ang:.1} deg, target {target:.1} deg, circular error {err:.1} deg"
    );
}

/// Same seed, twice: identical anchors. The settlement layer can only ever
/// record facts if the fact-production is deterministic.
#[test]
fn determinism_holds() {
    let c = generate(1, 16, DEFAULT_SEED);
    let mut stim = Stimuli::new();
    stim.set(c.offset(Region::CxEb) as u32, 0, 8);
    let a = run(&c, 32, &stim, false);
    let b = run(&c, 32, &stim, false);
    assert_eq!(a.anchor, b.anchor);
    assert_eq!(a.region_spikes, b.region_spikes);
}

/// An honest audited trace satisfies every transition constraint; flipping a
/// single spike flag is caught. This is what "provable execution" means here:
/// there is a predicate and the predicate bites.
#[test]
fn air_checker_accepts_honest_and_rejects_tampered() {
    let c = generate(1, 16, DEFAULT_SEED);
    let mut stim = Stimuli::new();
    let off = c.offset(Region::CxEb) as u32;
    stim.set(off, 0, 8);
    let r = run(&c, 32, &stim, true);
    let rows = r.rows.expect("audit rows");
    assert_eq!(count_violations(&rows), 0, "honest trace must pass");

    let mut tampered = rows;
    let row = tampered[0][1];
    tampered[0][1] = budfly::sim::TraceRow {
        spike: 1 - row.spike,
        ..row
    };
    assert!(count_violations(&tampered) > 0, "tampering must be caught");
}

/// The 1/16 stylized CNS fits the BudFly-N1 sketch with SRAM headroom.
#[test]
fn placement_fits_n1() {
    let c = generate(1, 16, DEFAULT_SEED);
    let p = place(&c, &Chip::n1());
    assert_eq!(p.sram_overflow_cores, 0, "must fit core SRAM");
    assert!(p.syn_per_core_max <= Chip::n1().syn_cap());
}

/// The real question of the whole crate, answered numerically: the published
/// MaleCNS at full scale is *runnable* on a plausible mesh — three orders of
/// magnitude faster than the fly's own millisecond time constants.
#[test]
fn real_scale_is_runnable_and_fast() {
    let r = real_scale_report(&Chip::n1());
    assert!(r.mesh_x * r.mesh_y >= r.cores);
    assert!(r.cycles_per_tick <= 8, "tick cost stays single-digit cycles");
    let tps = r.ticks_per_second;
    assert!(
        tps >= 1_000_000,
        "fabric must crush biological real-time, got {tps} ticks/s"
    );
    // Energy sanity: microjoule-scale per tick, not watts of GPU.
    assert!(r.energy_per_tick_pj < 10_000_000);
}

/// Different digests may legitimately yield different descending readouts —
/// the four goldens collectively cover Abstain/Affirm/Reject — but identical
/// digests must always yield identical ones.
#[test]
fn oracle_is_a_function_of_the_digest() {
    let c = generate(1, 16, DEFAULT_SEED);
    let mut seen = std::collections::HashSet::new();
    for fill in [0u8, 0xff, 7, 42] {
        let d = [fill; 32];
        let a = sentinel_verdict(&c, &d);
        let b = sentinel_verdict(&c, &d);
        assert_eq!(a.anchor, b.anchor);
        seen.insert((format!("{:?}", a.verdict), a.dn_l, a.dn_r, a.mdn));
    }
    // At least two distinguishable descending outcomes across four digests:
    // the readout is not a constant.
    assert!(seen.len() >= 2);
}
