//! MaleCNS-*styled* connectome generator.
//!
//! Honesty up front, FlyCoder-style: this is **not** the published MaleCNS
//! connectome. It is a deterministic, integer-only graph built from canonical
//! *Drosophila* circuit motifs, sized so unit tests stay fast and scaled so
//! the proportions track the real thing:
//!
//! | motif | regions | real-system anchor |
//! |---|---|---|
//! | visual pathway | lamina -> medulla -> lobula (bilateral) | retinotopic cartridges/columns |
//! | compass | central-complex FB -> EB ring + global inhibition | heading ring attractor |
//! | associative memory | mushroom body PN -> KC -> MBON, APL feedback | sparse combinatorial KC code |
//! | action selection | lateral horn -> DNa02-like L/R command pools, MDN veto | descending neurons (FlyCoder's readout) |
//!
//! The generator v1.0 is *frozen*: edge order, RNG stream layout and region
//! floors are part of the golden anchors in `tests/goldens.rs`. Any change
//! here is a new generator version, not an edit.
//!
//! A real-edge import path (FlyWire/MaleCNS `pre,post,syn_count` CSV) is
//! documented in `docs/BUDFLY.md`; nothing network-sized is committed to Git.

use crate::rng::SplitMix64;
use crate::sim::W_SYN;

/// Number of regions in the stylized CNS.
pub const N_REGIONS: usize = 16;

/// Region index. Discriminants are frozen; offsets are assigned in this order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(usize)]
pub enum Region {
    /// Lamina, left eye (sensory surface; feeds only on external input).
    LamL = 0,
    /// Lamina, right eye.
    LamR = 1,
    /// Medulla, left.
    MedL = 2,
    /// Medulla, right.
    MedR = 3,
    /// Lobula/lobula-plate complex, left (LC/LPLC types in FlyCoder's encoding).
    LobL = 4,
    /// Lobula complex, right.
    LobR = 5,
    /// Central complex, ellipsoid-body ring (bump attractor).
    CxEb = 6,
    /// Central complex, fan-shaped body.
    CxFb = 7,
    /// Ring-global inhibitory pool.
    CxInh = 8,
    /// Mushroom-body Kenyon cells.
    MbKc = 9,
    /// Mushroom-body output neurons (MBON).
    MbMbon = 10,
    /// APL: mushroom-body global feedback inhibition.
    MbApl = 11,
    /// Lateral horn.
    Lh = 12,
    /// Descending command pool, left (DNa02-like).
    DnL = 13,
    /// Descending command pool, right.
    DnR = 14,
    /// MDN-like cross-channel veto.
    Mdn = 15,
}

impl Region {
    /// All regions, in frozen index order.
    pub const ALL: [Region; N_REGIONS] = [
        Region::LamL,
        Region::LamR,
        Region::MedL,
        Region::MedR,
        Region::LobL,
        Region::LobR,
        Region::CxEb,
        Region::CxFb,
        Region::CxInh,
        Region::MbKc,
        Region::MbMbon,
        Region::MbApl,
        Region::Lh,
        Region::DnL,
        Region::DnR,
        Region::Mdn,
    ];

    /// Frozen index.
    #[must_use]
    pub const fn idx(self) -> usize {
        self as usize
    }

    /// Human-readable name (also used by the report example).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Region::LamL => "lamina_L",
            Region::LamR => "lamina_R",
            Region::MedL => "medulla_L",
            Region::MedR => "medulla_R",
            Region::LobL => "lobula_L",
            Region::LobR => "lobula_R",
            Region::CxEb => "central_complex_EB_ring",
            Region::CxFb => "central_complex_FB",
            Region::CxInh => "cx_inhibitory",
            Region::MbKc => "mushroom_body_KC",
            Region::MbMbon => "mbon",
            Region::MbApl => "apl",
            Region::Lh => "lateral_horn",
            Region::DnL => "DNa02_like_L",
            Region::DnR => "DNa02_like_R",
            Region::Mdn => "MDN_like",
        }
    }

    /// Size at scale 1 (proportions track the male CNS; stylized, not real).
    const fn base(self) -> usize {
        match self {
            Region::LamL | Region::LamR => 512,
            Region::MedL | Region::MedR => 1024,
            Region::LobL | Region::LobR => 512,
            Region::CxEb => 64,
            Region::CxFb => 128,
            Region::CxInh => 8,
            Region::MbKc => 2000,
            Region::MbMbon => 10,
            Region::MbApl => 1,
            Region::Lh => 256,
            Region::DnL | Region::DnR => 8,
            Region::Mdn => 4,
        }
    }

    /// Minimum size at any scale: small pools must survive downscaling or the
    /// compass ring and the descending readout lose their meaning.
    const fn floor(self) -> usize {
        match self {
            Region::CxEb => 48,
            Region::CxFb => 64,
            Region::CxInh => 6,
            Region::MbMbon => 8,
            Region::MbApl => 1,
            Region::Lh => 64,
            Region::DnL | Region::DnR => 6,
            Region::Mdn => 4,
            _ => 8,
        }
    }
}

/// One directed, weighted edge. Weight is Q4.12 fixed point; negative means
/// inhibitory. Magnitude = synapse_count * [`W_SYN`].
#[derive(Clone, Copy, Debug)]
pub struct Edge {
    /// Presynaptic global neuron id.
    pub pre: u32,
    /// Postsynaptic global neuron id.
    pub post: u32,
    /// Q4.12 weight.
    pub w: i32,
}

/// A generated connectome: region layout plus the edge list (in frozen
/// generation order) plus a per-neuron adjacency built stably from it.
#[derive(Clone, Debug)]
pub struct Connectome {
    /// Neurons per region, indexed by [`Region::idx`].
    pub sizes: [usize; N_REGIONS],
    /// First global id of each region.
    pub offsets: [usize; N_REGIONS],
    /// Total neuron count.
    pub total: usize,
    /// Edges in generation order (golden-relevant).
    pub edges: Vec<Edge>,
    /// Adjacency by presynaptic id; each row preserves generation order.
    pub adj: Vec<Vec<Edge>>,
}

impl Connectome {
    /// First global id of `region`.
    #[must_use]
    pub fn offset(&self, region: Region) -> usize {
        self.offsets[region.idx()]
    }

    /// Neuron count of `region`.
    #[must_use]
    pub fn size(&self, region: Region) -> usize {
        self.sizes[region.idx()]
    }

    /// Region of a global id (16-entry scan; trivially cheap).
    #[must_use]
    pub fn region_of(&self, gid: usize) -> Region {
        for r in Region::ALL {
            if gid >= self.offsets[r.idx()] && gid < self.offsets[r.idx()] + self.sizes[r.idx()] {
                return r;
            }
        }
        // Every gid < total belongs to exactly one region by construction.
        Region::LamL
    }

    /// Per-region zero fan-in counts (quality gate uses this: every region
    /// except the lamina sensory surfaces must be fully reachable).
    #[must_use]
    pub fn zero_fan_in_by_region(&self) -> [usize; N_REGIONS] {
        let mut indeg = vec![0usize; self.total];
        for e in &self.edges {
            indeg[e.post as usize] += 1;
        }
        let mut out = [0usize; N_REGIONS];
        for r in Region::ALL {
            let (off, n) = (self.offsets[r.idx()], self.sizes[r.idx()]);
            out[r.idx()] = (off..off + n).filter(|&g| indeg[g] == 0).count();
        }
        out
    }
}

/// Motif-scoped stream tags: adding or retuning one motif never reshuffles
/// the others. Frozen in generator v1.0.
const TAG_VISUAL: u64 = 0xA11CE;
const TAG_CENTRAL: u64 = 0xC0FFEE;
const TAG_MUSHROOM: u64 = 0xDEC0DE;
const TAG_OUTPUT: u64 = 0xF00D;

/// Default generator seed (frozen).
pub const DEFAULT_SEED: u64 = 0xB0DF17;

/// Generator state: region layout (read-only) plus the growing edge list.
/// Bundled so the hot `add` stays within the arg-count lint budget.
struct Gen<'a> {
    offsets: &'a [usize; N_REGIONS],
    edges: Vec<Edge>,
}

impl Gen<'_> {
    /// Appends one directed edge; generation order is golden-relevant.
    fn add(&mut self, pre: Region, i: usize, post: Region, j: usize, count: u32, inhibitory: bool) {
        let mut w = i32::try_from(count.max(1)).unwrap_or(1).saturating_mul(W_SYN);
        if inhibitory {
            w = -w;
        }
        self.edges.push(Edge {
            pre: (self.offsets[pre.idx()] + i) as u32,
            post: (self.offsets[post.idx()] + j) as u32,
            w,
        });
    }
}

/// Generates the frozen v1.0 connectome at `scale_num / scale_den`.
///
/// `(1, 16)` is the unit-test scale: 588 neurons, 2,283 edges, anchors pinned
/// in `tests/goldens.rs`. `(1, 1)` gives the full stylized CNS (~6.4k neurons).
#[must_use]
pub fn generate(scale_num: u32, scale_den: u32, seed: u64) -> Connectome {
    let mut rv = SplitMix64::new(seed ^ TAG_VISUAL);
    let mut rc = SplitMix64::new(seed ^ TAG_CENTRAL);
    let mut rm = SplitMix64::new(seed ^ TAG_MUSHROOM);
    let mut ro = SplitMix64::new(seed ^ TAG_OUTPUT);

    let mut sizes = [0usize; N_REGIONS];
    let mut offsets = [0usize; N_REGIONS];
    let mut total = 0usize;
    for r in Region::ALL {
        let scaled = r.base() * scale_num as usize / scale_den as usize;
        let n = r.floor().max(scaled);
        sizes[r.idx()] = n;
        offsets[r.idx()] = total;
        total += n;
    }
    let mut gen = Gen {
        offsets: &offsets,
        edges: Vec::new(),
    };

    // --- visual pathway: lamina -> medulla -> lobula, per side ----------------
    for (lam, med, lob) in [
        (Region::LamL, Region::MedL, Region::LobL),
        (Region::LamR, Region::MedR, Region::LobR),
    ] {
        let (nl, nm, nb) = (sizes[lam.idx()], sizes[med.idx()], sizes[lob.idx()]);
        for i in 0..nl {
            // lamina -> medulla: 2:1 column mapping + occasional lateral jitter
            for t in 0..2usize {
                let j = (i * 2 + t) % nm;
                gen.add(lam, i, med, j, 3 + rv.below(8), false);
            }
            if rv.below(4) == 0 {
                let j = (i * 2 + 2 + rv.below(7) as usize) % nm;
                gen.add(lam, i, med, j, 1 + rv.below(3), false);
            }
        }
        for jm in 0..nm {
            // medulla -> lobula: convergent, indexed + sparse cross-talk
            for _ in 0..2 {
                let src = rv.below(nm as u32) as usize;
                let dst = (jm * nb / nm) % nb;
                gen.add(med, src, lob, dst, 2 + rv.below(6), false);
                if rv.below(10) == 0 {
                    let dst2 = rv.below(nb as u32) as usize;
                    gen.add(med, src, lob, dst2, 1 + rv.below(3), false);
                }
            }
        }
    }

    // --- lobula outputs: LH, compass FB, and mushroom-body projection neurons -
    for (side, lob) in [(0usize, Region::LobL), (1usize, Region::LobR)] {
        let n_lob = sizes[lob.idx()];
        let (n_lh, n_fb, n_kc) = (
            sizes[Region::Lh.idx()],
            sizes[Region::CxFb.idx()],
            sizes[Region::MbKc.idx()],
        );
        for i in 0..n_lob {
            // interleaved indexed coverage: every LH and FB neuron is reached
            let lh_idx = (i * n_lh / n_lob + side) % n_lh;
            gen.add(lob, i, Region::Lh, lh_idx, 2 + ro.below(5), false);
            let lh_rnd = ro.below(n_lh as u32) as usize;
            gen.add(lob, i, Region::Lh, lh_rnd, 1 + ro.below(4), false);
            let fb_idx = (i * n_fb / n_lob + side) % n_fb;
            gen.add(lob, i, Region::CxFb, fb_idx, 1 + rc.below(4), false);
            let fb_rnd = rc.below(n_fb as u32) as usize;
            gen.add(lob, i, Region::CxFb, fb_rnd, 1 + rc.below(3), false);
            if rm.below(4) < 1 {
                // ~25% of lobula neurons act as projection neurons (PNs)
                for _ in 0..(1 + rm.below(3)) {
                    let kc = rm.below(n_kc as u32) as usize;
                    gen.add(lob, i, Region::MbKc, kc, 1 + rm.below(4), false);
                }
            }
        }
    }

    // --- mushroom body: every KC gets >= 3 PN inputs (no deaf KCs) ------------
    let (n_ll, n_lr) = (sizes[Region::LobL.idx()], sizes[Region::LobR.idx()]);
    let lob_span = (n_ll + n_lr) as u32;
    for k in 0..sizes[Region::MbKc.idx()] {
        for _ in 0..3 {
            let pick = rm.below(lob_span) as usize;
            let (lr, local) = if pick < n_ll {
                (Region::LobL, pick)
            } else {
                (Region::LobR, pick - n_ll)
            };
            gen.add(lr, local, Region::MbKc, k, 1 + rm.below(3), false);
        }
    }

    // --- compass: FB -> EB ring, ring-local excitation, global inhibition -----
    let (n_fb, n_eb, n_ci, n_mdn) = (
        sizes[Region::CxFb.idx()],
        sizes[Region::CxEb.idx()],
        sizes[Region::CxInh.idx()],
        sizes[Region::Mdn.idx()],
    );
    for k in 0..n_fb {
        for _ in 0..2 {
            let j = rc.below(n_eb as u32) as usize;
            gen.add(Region::CxFb, k, Region::CxEb, j, 3 + rc.below(5), false);
        }
    }
    for i in 0..n_eb {
        // ring: excite neighbours, weighted by proximity (bump recurrence)
        for (d, c) in [(1usize, 10u32), (2, 8), (3, 5), (4, 3)] {
            let jf = (i + d) % n_eb;
            gen.add(Region::CxEb, i, Region::CxEb, jf, c, false);
            let jb = (i + n_eb - d) % n_eb;
            gen.add(Region::CxEb, i, Region::CxEb, jb, c, false);
        }
        let ci = i % n_ci;
        gen.add(Region::CxEb, i, Region::CxInh, ci, 4, false);
    }
    for i in 0..n_ci {
        for j in 0..n_eb {
            gen.add(Region::CxInh, i, Region::CxEb, j, 3, true);
        }
    }
    // EB -> FB recurrence keeps the bump alive across ticks
    for i in 0..n_eb {
        if rc.below(2) == 0 {
            let k = rc.below(n_fb as u32) as usize;
            gen.add(Region::CxEb, i, Region::CxFb, k, 2 + rc.below(4), false);
        }
    }

    // --- FB compass state feeds the MDN veto channel --------------------------
    for k in 0..n_fb {
        if rc.below(3) == 0 {
            let j = ro.below(n_mdn as u32) as usize;
            gen.add(Region::CxFb, k, Region::Mdn, j, 2 + ro.below(3), false);
        }
    }

    // --- mushroom body output: KC -> MBON, KC -> APL, APL -> KC ---------------
    let n_kc = sizes[Region::MbKc.idx()];
    let n_mbon = sizes[Region::MbMbon.idx()];
    for k in 0..n_kc {
        let j = rm.below(n_mbon as u32) as usize;
        gen.add(Region::MbKc, k, Region::MbMbon, j, 1 + rm.below(2), false);
        if rm.below(3) == 0 {
            gen.add(Region::MbKc, k, Region::MbApl, 0, 2, false);
        }
    }
    for k in (0..n_kc).step_by(4) {
        gen.add(Region::MbApl, 0, Region::MbKc, k, 1, true);
    }

    // --- action selection: LH + MBON -> descending pools; L/R mutual veto -----
    let n_lh = sizes[Region::Lh.idx()];
    let (n_dl, n_dr) = (sizes[Region::DnL.idx()], sizes[Region::DnR.idx()]);
    for i in 0..n_lh {
        let (tgt, n_t) = if i % 2 == 0 { (Region::DnL, n_dl) } else { (Region::DnR, n_dr) };
        let j = ro.below(n_t as u32) as usize;
        gen.add(Region::Lh, i, tgt, j, 3 + ro.below(6), false);
    }
    for i in 0..n_mbon {
        let jl = ro.below(n_dl as u32) as usize;
        gen.add(Region::MbMbon, i, Region::DnL, jl, 2 + ro.below(4), false);
        let jr = ro.below(n_dr as u32) as usize;
        gen.add(Region::MbMbon, i, Region::DnR, jr, 2 + ro.below(4), false);
    }
    for i in 0..n_dl {
        let j = ro.below(n_dr as u32) as usize;
        gen.add(Region::DnL, i, Region::DnR, j, 2, true);
    }
    for i in 0..n_dr {
        let j = ro.below(n_dl as u32) as usize;
        gen.add(Region::DnR, i, Region::DnL, j, 2, true);
    }
    for i in 0..n_mdn {
        let jl = ro.below(n_dl as u32) as usize;
        gen.add(Region::Mdn, i, Region::DnL, jl, 2, true);
        let jr = ro.below(n_dr as u32) as usize;
        gen.add(Region::Mdn, i, Region::DnR, jr, 2, true);
    }

    let edges = gen.edges;
    // Stable adjacency: iterate edges in generation order, append per pre row.
    let mut adj: Vec<Vec<Edge>> = vec![Vec::new(); total];
    for e in &edges {
        adj[e.pre as usize].push(*e);
    }

    Connectome {
        sizes,
        offsets,
        total,
        edges,
        adj,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_1_16_shape_is_frozen() {
        let c = generate(1, 16, DEFAULT_SEED);
        assert_eq!(c.total, 588);
        assert_eq!(c.edges.len(), 2283);
    }

    #[test]
    fn every_region_except_lamina_is_fully_reached() {
        let c = generate(1, 16, DEFAULT_SEED);
        let zeros = c.zero_fan_in_by_region();
        for r in Region::ALL {
            match r {
                Region::LamL | Region::LamR => {
                    assert_eq!(zeros[r.idx()], c.size(r), "lamina is the sensory surface");
                }
                _ => {
                    let region = r.name();
                    assert_eq!(zeros[r.idx()], 0, "{region} must be reachable");
                }
            }
        }
    }

    #[test]
    fn region_layout_is_contiguous() {
        let c = generate(1, 4, DEFAULT_SEED);
        let mut cursor = 0;
        for r in Region::ALL {
            assert_eq!(c.offset(r), cursor);
            cursor += c.size(r);
        }
        assert_eq!(cursor, c.total);
    }
}
