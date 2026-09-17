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

/// The eight season-league seeds (frozen).
pub const LEAGUE2_SEEDS: [u64; 8] = [
    0xB0DF17, 0xB0DF18, 0xB0DF19, 0xB0DF1A, 0xB0DF1B, 0xB0DF1C, 0xB0DF1D, 0xB0DF1E,
];
/// Number of archived seasons (frozen).
pub const ARCHIVE_SEASONS: u8 = 3;
/// Archive chain domain seed (frozen).
pub const ARCHIVE_DOMAIN: &[u8] = b"BUDFLY-ARCHIVE1\x00";

/// Season-salted panel digest: `sha256(b"budfly-panel" | season | i)`.
#[must_use]
pub fn season_panel_digest(season: u8, i: u8) -> [u8; 32] {
    let mut buf = [0u8; 14];
    buf[..12].copy_from_slice(b"budfly-panel");
    buf[12] = season;
    buf[13] = i;
    crate::sha256::sha256(&buf)
}

/// One fly's row in one archived season (same rules as the 4-team league).
#[must_use]
pub fn season_row2(conn: &Connectome, seed: u64, season: u8) -> LeagueRow {
    let mut abstains = 0;
    for i in 0..PANEL_SIZE as u8 {
        if !council(conn, &season_panel_digest(season, i)).unanimous {
            abstains += 1;
        }
    }
    let stim = sentinel_stimulus(
        conn,
        &crate::divan::seat_digest(&season_panel_digest(season, 0), 0),
    );
    let log = run_log(conn, crate::oracle::SENTINEL_TICKS, &stim);
    LeagueRow {
        seed,
        abstains,
        tie_anchor: *log.last().unwrap_or(&[0u8; 32]),
    }
}

/// One archived season table.
#[derive(Clone, Debug)]
pub struct Season {
    /// Season number (1-based).
    pub season: u8,
    /// Rows in seed order.
    pub rows: Vec<LeagueRow>,
    /// The standing (seeds in rank order).
    pub standing_table: Vec<u64>,
}

/// Plays all archived seasons (eight teams, three season-salted panels)
/// and chains each table into the archive head:
///
/// ```text
/// standing_digest(s) = sha256(s u8 | per team in standing order:
///                      seed u64 LE | abstains u8 | tie_anchor 32B)
/// head(0) = sha256("BUDFLY-ARCHIVE1\x00"); head(s) = sha256(prev | digest)
/// ```
#[must_use]
pub fn play_archive() -> (Vec<Season>, [u8; 32]) {
    let flies: Vec<Connectome> = LEAGUE2_SEEDS
        .iter()
        .map(|sd| generate(1, 16, *sd))
        .collect();
    let mut head = crate::sha256::sha256(ARCHIVE_DOMAIN);
    let mut seasons = Vec::new();
    for s in 1..=ARCHIVE_SEASONS {
        let rows: Vec<LeagueRow> = flies
            .iter()
            .zip(LEAGUE2_SEEDS.iter())
            .map(|(c, sd)| season_row2(c, *sd, s))
            .collect();
        let table = standing(&rows);
        let mut sd_bytes = vec![s];
        for sd in &table {
            for r in &rows {
                if r.seed == *sd {
                    sd_bytes.extend_from_slice(&sd.to_le_bytes());
                    sd_bytes.push(r.abstains as u8);
                    sd_bytes.extend_from_slice(&r.tie_anchor);
                }
            }
        }
        head = crate::sha256::sha256(&[head, crate::sha256::sha256(&sd_bytes)].concat());
        seasons.push(Season {
            season: s,
            rows,
            standing_table: table,
        });
    }
    (seasons, head)
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

    #[test]
    fn the_archive_chains_three_seasons_of_eight() {
        let (seasons, head) = play_archive();
        assert_eq!(seasons.len(), 3);
        let abs: Vec<Vec<u32>> = seasons
            .iter()
            .map(|s| s.rows.iter().map(|r| r.abstains).collect())
            .collect();
        assert_eq!(abs[0], vec![7, 6, 7, 5, 7, 2, 6, 6]);
        assert_eq!(abs[1], vec![7, 5, 6, 6, 8, 4, 6, 7]);
        assert_eq!(abs[2], vec![8, 5, 7, 4, 6, 2, 7, 4]);
        assert_eq!(
            seasons[0].standing_table,
            vec![0xB0DF1C, 0xB0DF1A, 0xB0DF1D, 0xB0DF18, 0xB0DF1E, 0xB0DF17, 0xB0DF1B, 0xB0DF19]
        );
        assert_eq!(
            seasons[1].standing_table,
            vec![0xB0DF1C, 0xB0DF18, 0xB0DF1D, 0xB0DF19, 0xB0DF1A, 0xB0DF17, 0xB0DF1E, 0xB0DF1B]
        );
        assert_eq!(
            seasons[2].standing_table,
            vec![0xB0DF1C, 0xB0DF1A, 0xB0DF1E, 0xB0DF18, 0xB0DF1B, 0xB0DF1D, 0xB0DF19, 0xB0DF17]
        );
        for s in &seasons {
            assert_eq!(
                s.standing_table.first().copied().unwrap_or(0),
                0xB0DF1C,
                "the dynasty holds three seasons straight"
            );
        }
        assert_eq!(
            crate::sha256::hex32(&head),
            "50073d6867b11a3e286d6eb032477c05411b7b5a47faa93e53391c695a59355b"
        );
    }
}
