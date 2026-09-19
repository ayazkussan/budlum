//! Lesion harness: silence a region, pin the behavioral deficit.
//!
//! Bit-exact mirror of `scripts/expansion_check.py` [lesion]. The canonical
//! probe is an odor stimulus: left-lamina bump for ticks 0..8 plus a fixed
//! PN pattern into every other KC for ticks 8..16, run for 48 ticks.
//! Honest note in the pinned data: the APL probe shows no measurable deficit
//! on this probe (mbon delta is pinned at 0) — the probe is documented as
//! coarse, never overstated.

use crate::connectome::{Connectome, Region};
use crate::sim::{run_masked, Stimuli};

/// Canonical ablation targets of the battery.
#[derive(Clone, Copy, Debug)]
pub enum LesionProbe {
    /// Mushroom-body global feedback inhibition (APL).
    Apl,
    /// Ellipsoid-body compass ring.
    CxEb,
    /// MDN-like cross-channel veto pool.
    Mdn,
}

impl LesionProbe {
    fn region(self) -> Region {
        match self {
            Self::Apl => Region::MbApl,
            Self::CxEb => Region::CxEb,
            Self::Mdn => Region::Mdn,
        }
    }
}

/// Builds the canonical odor probe stimulus.
#[must_use]
pub fn odor_stim(conn: &Connectome) -> Stimuli {
    let mut st = Stimuli::new();
    let l_off = conn.offset(Region::LamL);
    for i in 0..conn.size(Region::LamL) / 4 {
        st.set((l_off + i) as u32, 0, 8);
    }
    let k_off = conn.offset(Region::MbKc);
    for i in 0..conn.size(Region::MbKc) / 2 {
        st.set((k_off + i * 2) as u32, 8, 16);
    }
    st
}

/// Dead-neuron mask for one region.
#[must_use]
pub fn dead_region(conn: &Connectome, region: Region) -> Vec<bool> {
    let mut dm = vec![false; conn.total];
    let (lo, hi) = (conn.offset(region), conn.offset(region) + conn.size(region));
    dm[lo..hi].fill(true);
    dm
}

/// Runs the canonical 48-tick odor probe with an optional ablation.
/// Returns (anchor, spikes per region).
#[must_use]
pub fn lesion_case(conn: &Connectome, probe: Option<LesionProbe>) -> ([u8; 32], [u64; 16]) {
    let stim = odor_stim(conn);
    let dead = match probe {
        None => vec![false; conn.total],
        Some(p) => dead_region(conn, p.region()),
    };
    let r = run_masked(conn, 48, &stim, &dead);
    (r.anchor, r.region_spikes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectome::{generate, DEFAULT_SEED};
    use crate::sha256::hex32;

    #[test]
    fn lesion_battery_is_frozen() {
        let c = generate(1, 16, DEFAULT_SEED);
        let (base_a, base_s) = lesion_case(&c, None);
        assert_eq!(
            hex32(&base_a),
            "f3d639395fce7e866b3f564d52a90b22c104faa7325351626a3ad08faa51d8e8"
        );
        assert_eq!(
            base_s,
            [32, 0, 38, 0, 32, 0, 517, 98, 103, 254, 32, 4, 15, 10, 7, 5]
        );

        let (a, s) = lesion_case(&c, Some(LesionProbe::Apl));
        assert_eq!(
            hex32(&a),
            "75660cb05f2ee5d0384e8517d3a9343596637a296f7bf21c64bfa962bd65ae65"
        );
        assert_eq!(s[Region::MbApl.idx()], 0);
        assert_eq!(s[Region::MbMbon.idx()], base_s[Region::MbMbon.idx()]);

        let (a, s) = lesion_case(&c, Some(LesionProbe::CxEb));
        assert_eq!(
            hex32(&a),
            "80ed2517a96a3813bb5534cad1dce3d05199f49fdb5544dd1dc9e7d553e087cb"
        );
        assert_eq!(s[Region::CxEb.idx()], 0);
        assert_eq!(s[Region::CxFb.idx()], 4);
        assert_eq!(s[Region::CxInh.idx()], 0);
        assert_eq!(s[Region::Mdn.idx()], 0);

        let (a, s) = lesion_case(&c, Some(LesionProbe::Mdn));
        assert_eq!(
            hex32(&a),
            "56a0e023689c415d3a664e3c867f2fa994dab19390e0213b402c624145c28ec7"
        );
        assert_eq!(s[Region::Mdn.idx()], 0);
        assert_eq!(s[Region::CxEb.idx()], base_s[Region::CxEb.idx()]);
    }
}
