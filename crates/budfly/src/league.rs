//! League — season mode for the tournament: four seeds, one panel, a
//! standings table an arbitrator could sign.
//!
//! RULES (frozen): each fly faces the frozen 8-digest panel as in
//! `tournament.rs`; a council that fails unanimity costs one abstention.
//! Standing rule: fewer abstentions ranks higher; exact ties break by the
//! seat-0 anchor of panel digest 0, lexicographic (arbitrary, deterministic,
//! pinned — that is all a tie-break must be).
//!
//! The frozen season result is a full upset ladder: reigning tournament
//! champion `0xB0DF18` drops to third, rookie `0xB0DF1A` takes the crown,
//! and the REFERENCE seed holds the wooden spoon. The metric keeps cutting
//! where it claims to: decisiveness, nothing else.
//!
//! Bit-exact mirror of `scripts/expansion_check.py` [league].

use crate::connectome::{generate, Connectome};
use crate::divan::council;
use crate::oracle::sentinel_stimulus;
use crate::sim::run_log;
use crate::tournament::{panel_digest, PANEL_SIZE};

/// The four season seeds (frozen).
pub const LEAGUE_SEEDS: [u64; 4] = [0xB0DF17, 0xB0DF18, 0xB0DF19, 0xB0DF1A];

/// One season row: what a fly committed over the panel.
#[derive(Clone, Debug)]
pub struct LeagueRow {
    /// Connectome seed.
    pub seed: u64,
    /// Council non-unanimity count over the panel.
    pub abstains: u32,
    /// Tie-break anchor (seat 0, panel digest 0, chain end).
    pub tie_anchor: [u8; 32],
}

/// One fly's season over the frozen panel.
#[must_use]
pub fn season_row(conn: &Connectome, seed: u64) -> LeagueRow {
    let mut abstains = 0;
    for i in 0..PANEL_SIZE as u8 {
        if !council(conn, &panel_digest(i)).unanimous {
            abstains += 1;
        }
    }
    let stim = sentinel_stimulus(conn, &crate::divan::seat_digest(&panel_digest(0), 0));
    let log = run_log(conn, crate::oracle::SENTINEL_TICKS, &stim);
    LeagueRow {
        seed,
        abstains,
        tie_anchor: *log.last().unwrap_or(&[0u8; 32]),
    }
}

/// Generates the four flies and plays the season.
#[must_use]
pub fn play_season() -> Vec<LeagueRow> {
    LEAGUE_SEEDS
        .iter()
        .map(|sd| season_row(&generate(1, 16, *sd), *sd))
        .collect()
}

/// The standings: abstentions ascending, then tie-break anchor ascending.
#[must_use]
pub fn standing(rows: &[LeagueRow]) -> Vec<u64> {
    let mut xs: Vec<LeagueRow> = rows.to_vec();
    xs.sort_by_key(|r| (r.abstains, r.tie_anchor));
    xs.iter().map(|r| r.seed).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_season_standings_are_frozen() {
        let rows = play_season();
        let abstains: Vec<u32> = rows.iter().map(|r| r.abstains).collect();
        assert_eq!(abstains, vec![7, 5, 5, 3]);
        let table = standing(&rows);
        assert_eq!(
            table,
            vec![0xB0DF1Au64, 0xB0DF19, 0xB0DF18, 0xB0DF17],
            "rookie champion, reference wooden spoon (tie 5-5 to seat-0 anchor)"
        );
        assert_eq!(table.first().copied().unwrap_or(0), 0xB0DF1A);
    }

    #[test]
    fn tie_break_is_deterministic_and_documented() {
        let rows = play_season();
        // 5==5 tie between 0xB0DF18 and 0xB0DF19 resolves by seat-0 anchor:
        let get = |sd: u64| rows.iter().find(|r| r.seed == sd).map(|r| r.tie_anchor);
        let a18 = get(0xB0DF18).unwrap_or([0u8; 32]);
        let a19 = get(0xB0DF19).unwrap_or([0u8; 32]);
        assert!(a19 < a18, "tie-break pins 0xB0DF19 ahead of 0xB0DF18");
    }
}
