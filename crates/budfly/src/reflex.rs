//! Reflex-latency map: conduction delay from sensory surface to command
//! layer, measured in ticks over the frozen circuit.
//!
//! Bit-exact mirror of `scripts/expansion_check.py` [reflex]. Protocol:
//! left-lamina quarter bump ticks 0..8 only (no odor PN pattern), run 48
//! ticks, pin the first spike tick per region. The pinned table shows the
//! reflex arc (`LamL:0 MedL:1 LobL:2 MbKc:5 Lh:5 DnL:6`), the CX lag
//! (Fb:9 Eb:10 Inh:13), the MDN veto arriving 16 ticks after DnL — and
//! left/right LATERALIZATION: a left-eye stimulus never fires DnR on this
//! connectome (pinned -1), and MBON stays silent without the odor pattern.

use crate::connectome::{Connectome, Region, N_REGIONS};
use crate::sim::{run_masked, Stimuli};

/// First spike tick per region, -1 = silent on this probe.
#[must_use]
pub fn first_spike_ticks(conn: &Connectome) -> [i32; N_REGIONS] {
    let l_off = conn.offset(Region::LamL);
    let mut stim = Stimuli::new();
    for i in 0..conn.size(Region::LamL) / 4 {
        stim.set((l_off + i) as u32, 0, 8);
    }
    let dead = vec![false; conn.total];
    let r = run_masked(conn, 48, &stim, &dead);
    let rows = r.rows.unwrap_or_default();
    let mut first = [-1i32; N_REGIONS];
    for (gid, rs) in rows.iter().enumerate() {
        let r = conn.region_of(gid).idx();
        if let Some(row) = rs.iter().find(|row| row.spike == 1) {
            if first[r] < 0 || (row.tick as i32) < first[r] {
                first[r] = row.tick as i32;
            }
        }
    }
    first
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectome::{generate, DEFAULT_SEED};

    #[test]
    fn reflex_map_is_frozen() {
        let c = generate(1, 16, DEFAULT_SEED);
        let f = first_spike_ticks(&c);
        let want = [0, -1, 1, -1, 2, -1, 10, 9, 13, 5, -1, -1, 5, 6, -1, 22];
        assert_eq!(f, want, "first-spike tick per region");
        // the behavioral arcs embedded in the table:
        assert_eq!(f[Region::DnL.idx()], 6, "LamL->DnL reflex arc = 6 ticks");
        assert_eq!(
            f[Region::Mdn.idx()],
            22,
            "MDN veto lags 16 ticks behind DnL"
        );
        assert_eq!(
            f[Region::DnR.idx()],
            -1,
            "left stimulus lateralizes (DnR silent)"
        );
        assert_eq!(
            f[Region::MbMbon.idx()],
            -1,
            "no odor pattern -> MBON silent"
        );
    }
}
