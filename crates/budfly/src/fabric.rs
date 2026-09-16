//! The hardware question, answered from the crazy angle.
//!
//! `fabric` is a software sketch of a neuromorphic many-core chip ("BudFly-N
//! fabric") sized to carry a connectome: cores hold neurons and their local
//! synapse table, spikes travel as address-event packets (AER) over a 2D mesh
//! with deterministic XY routing, and every tick is charged an energy budget.
//! It is a *model* — order-of-magnitude constants echo published neuromorphic
//! numbers (Loihi-class); nothing here claims a tape-out.
//!
//! Its job in this repository is to answer, with code instead of adjectives:
//! **what does silicon need to run the fly's map?** — and, via the anchors,
//! what would it need to run it *provably*.

use crate::connectome::Connectome;

/// Fabric energy constants (pico-joules), order-of-magnitude, parametric.
pub const E_HOP_PJ: u64 = 26;
/// Energy per synaptic operation.
pub const E_SOP_PJ: u64 = 24;
/// Energy per neuron state update.
pub const E_NEURON_UPDATE_PJ: u64 = 900;

/// Published MaleCNS scale used for the analytic sizing report
/// (the same figures FlyCoder ships: neuron and filtered-connection counts).
pub const MALE_CNS_NEURONS: usize = 166_700;
/// Filtered connections in the published MaleCNS export.
pub const MALE_CNS_CONNECTIONS: usize = 25_582_938;
/// Assumed mean spike activity per tick (2%, sparse — typical for SNN regimes).
pub const ACTIVE_PERCENT: usize = 2;

/// A chip configuration.
#[derive(Clone, Copy, Debug)]
pub struct Chip {
    /// Maximum neurons per core.
    pub neuron_cap: usize,
    /// Synaptic SRAM per core, bytes (8 B per synapse entry).
    pub syn_sram_bytes: usize,
    /// Synaptic operations per core per cycle.
    pub sop_per_cycle: usize,
}

impl Chip {
    /// The BudFly-N1 sketch: modest cores, dense mesh.
    #[must_use]
    pub const fn n1() -> Self {
        Self {
            neuron_cap: 256,
            syn_sram_bytes: 64 * 1024,
            sop_per_cycle: 64,
        }
    }

    /// Synapse entries that fit one core's SRAM.
    #[must_use]
    pub const fn syn_cap(&self) -> usize {
        self.syn_sram_bytes / 8
    }
}

/// Dimensions of the smallest square-ish mesh holding `n` cores.
#[must_use]
pub fn mesh_dims(n: usize) -> (usize, usize) {
    let mut x = 1usize;
    while x * x < n {
        x += 1;
    }
    (x, n.div_ceil(x))
}

/// Placement report for a concrete connectome on a [`Chip`].
#[derive(Clone, Copy, Debug)]
pub struct Placement {
    /// Cores used (sequential fill, `gid / neuron_cap`).
    pub cores: usize,
    /// Mesh width.
    pub mesh_x: usize,
    /// Mesh height.
    pub mesh_y: usize,
    /// Largest synapse count on any single core.
    pub syn_per_core_max: usize,
    /// Mean synapses per core.
    pub syn_per_core_avg: usize,
    /// Cores whose local synapses exceed SRAM (0 = fits).
    pub sram_overflow_cores: usize,
    /// Mean XY-route hops per edge, ×100 (fixed point for no-float purity).
    pub avg_hops_e2_x100: usize,
    /// Mean destination cores touched per spiking core, ×10.
    pub avg_dest_cores_per_spike_x10: usize,
}

/// Places `conn` on `chip` with sequential fill and measures routing load.
#[must_use]
pub fn place(conn: &Connectome, chip: &Chip) -> Placement {
    let cores = conn.total.div_ceil(chip.neuron_cap);
    let (mx, my) = mesh_dims(cores);
    let mut local = vec![0usize; cores];
    let mut hops_sum = 0usize;
    for e in &conn.edges {
        let (a, b) = (e.pre as usize / chip.neuron_cap, e.post as usize / chip.neuron_cap);
        local[b] += 1;
        let (ax, ay) = (a % mx, a / mx);
        let (bx, by) = (b % mx, b / mx);
        hops_sum += ax.abs_diff(bx) + ay.abs_diff(by);
    }
    let syn = conn.edges.len();
    let syn_per_core_max = local.iter().copied().max().unwrap_or(0);
    let overflow = local.iter().filter(|&&s| s > chip.syn_cap()).count();
    Placement {
        cores,
        mesh_x: mx,
        mesh_y: my,
        syn_per_core_max,
        syn_per_core_avg: syn / cores.max(1),
        sram_overflow_cores: overflow,
        avg_hops_e2_x100: if syn == 0 { 0 } else { hops_sum * 100 / syn },
        avg_dest_cores_per_spike_x10: dest_core_degree_avg_x10(conn, chip, cores),
    }
}

/// Mean |destination cores| per source core, ×10. One AER packet serves a
/// whole destination core (multicast collapses fan-out at the target), so
/// this is the per-spike packet count. Exact set semantics — the generator's
/// sections may interleave source cores, so nothing weaker than a real set
/// is correct here.
fn dest_core_degree_avg_x10(conn: &Connectome, chip: &Chip, cores: usize) -> usize {
    if cores == 0 {
        return 0;
    }
    let mut pairs = std::collections::HashSet::new();
    for e in &conn.edges {
        pairs.insert((e.pre as usize / chip.neuron_cap, e.post as usize / chip.neuron_cap));
    }
    pairs.len() * 10 / cores
}

/// Analytic sizing at the published MaleCNS scale (no million-edge graph is
/// built; the numbers are closed-form from region-averaged statistics).
#[derive(Clone, Copy, Debug)]
pub struct RealScaleReport {
    /// Cores required by neuron capacity alone.
    pub cores_by_neurons: usize,
    /// Cores required by synaptic SRAM alone (binding constraint).
    pub cores_by_synapses: usize,
    /// Cores provisioned (max of the two).
    pub cores: usize,
    /// Mesh width.
    pub mesh_x: usize,
    /// Mesh height.
    pub mesh_y: usize,
    /// Active (spiking) neurons per tick at [`ACTIVE_PERCENT`].
    pub active_per_tick: usize,
    /// Mean fan-out (connections per neuron).
    pub avg_fan_out: usize,
    /// Synaptic operations per tick.
    pub sops_per_tick: usize,
    /// Fabric cycles one tick costs at `sop_per_cycle` per core.
    pub cycles_per_tick: usize,
    /// Ticks per second at a conservative 1 GHz fabric clock.
    pub ticks_per_second: u64,
    /// Energy per tick in picojoules.
    pub energy_per_tick_pj: u64,
}

/// Computes the real-scale sizing: does hardware meet the fly's map?
#[must_use]
pub fn real_scale_report(chip: &Chip) -> RealScaleReport {
    let cores_by_neurons = MALE_CNS_NEURONS.div_ceil(chip.neuron_cap);
    let cores_by_synapses = (MALE_CNS_CONNECTIONS * 8).div_ceil(chip.syn_sram_bytes);
    let cores = cores_by_neurons.max(cores_by_synapses);
    let (mesh_x, mesh_y) = mesh_dims(cores);
    let active = MALE_CNS_NEURONS * ACTIVE_PERCENT / 100;
    let fan_out = MALE_CNS_CONNECTIONS / MALE_CNS_NEURONS;
    let sops = active * fan_out;
    let per_core = sops.div_ceil(cores);
    let cycles = per_core.div_ceil(chip.sop_per_cycle);
    let ticks_per_second = if cycles == 0 { u64::MAX } else { 1_000_000_000u64 / cycles as u64 };
    let energy = active as u64 * E_SOP_PJ
        + active as u64 * 4 * E_HOP_PJ
        + MALE_CNS_NEURONS as u64 * E_NEURON_UPDATE_PJ / 1000;
    RealScaleReport {
        cores_by_neurons,
        cores_by_synapses,
        cores,
        mesh_x,
        mesh_y,
        active_per_tick: active,
        avg_fan_out: fan_out,
        sops_per_tick: sops,
        cycles_per_tick: cycles,
        ticks_per_second,
        energy_per_tick_pj: energy,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectome::{generate, DEFAULT_SEED};

    #[test]
    fn scale_1_16_fits_three_cores() {
        let c = generate(1, 16, DEFAULT_SEED);
        let p = place(&c, &Chip::n1());
        assert_eq!(p.cores, 3);
        assert_eq!((p.mesh_x, p.mesh_y), (2, 2));
        assert_eq!(p.sram_overflow_cores, 0);
        assert_eq!(p.syn_per_core_max, 1617);
        // XY-hop average is an order check (reference: 0.32 hops/edge), not a
        // brittle one: placement keeps the graph tightly local.
        assert!(
            (20..=45).contains(&p.avg_hops_e2_x100),
            "locality lost: {} x100 hops/edge",
            p.avg_hops_e2_x100
        );
        assert_eq!(p.avg_dest_cores_per_spike_x10, 20);
    }

    #[test]
    fn real_scale_report_is_pinned() {
        let r = real_scale_report(&Chip::n1());
        assert_eq!(r.cores_by_neurons, 652);
        assert_eq!(r.cores_by_synapses, 3123);
        assert_eq!(r.cores, 3123);
        assert_eq!((r.mesh_x, r.mesh_y), (56, 56));
        assert_eq!(r.active_per_tick, 3334);
        assert_eq!(r.avg_fan_out, 153);
        assert_eq!(r.sops_per_tick, 510_102);
        assert_eq!(r.cycles_per_tick, 3);
        assert_eq!(r.ticks_per_second, 333_333_333);
        assert_eq!(r.energy_per_tick_pj, 576_782);
        // The sanity claim: three+ orders of magnitude above fly-real-time.
        assert!(r.ticks_per_second > 1_000_000);
    }
}
