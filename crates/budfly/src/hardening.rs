//! Hardening — the adversary sweep: how much room a forger actually has,
//! measured exhaustively and pinned to the byte.
//!
//! Not "does the honest tape pass" but "what is the full inventory of
//! mutations a verifier will tolerate". Every sweep is run, not argued:
//!
//! | surface | adversary move | frozen answer |
//! |---------|----------------|---------------|
//! | ACT-1 tape | flip one bit of EVERY byte (24,708 mutants), every truncation, one extension | `8 malformed`, `24,300 viol_caught`, `4 fold_caught` (**the tick bytes**), **396 escapes** |
//! | erasure | flip one bit at EVERY chunk position; replay the epoch | 32/32 positions avalanche all K answers; epoch moves 8/8 |
//! | attest | flip every nonce byte; impersonate a peer | 32/32 avalanche L1, ladder re-heads 32/32, peer binding holds |
//! | league | replay a council; shuffle the rows at the sort | identical anchors; identical table; tie engaged; total order |
//!
//! CLASSIFICATION PRECEDENCE (frozen): `malformed -> viol_caught ->
//! fold_caught -> escaped`. The 396 escapes are pinned offset-by-offset via
//! `escape_offsets_sha256` (sha256 over the LE u32 offset list).
//!
//! THE PINNED HONEST NEGATIVE — the "ghost refractory" class: every escape
//! is the field-16 bit flip `r_before := 0 ^ 1 -> 1` at the WINDOW'S FIRST
//! tick of a quiet neuron (`raw = v_after = spike = 0`). It validates
//! because (a) the window does not see tick `T-2`, so nothing contradicts
//! the fiction "this neuron spiked one tick before the window", (b) C5 is
//! satisfied (`v_after == 0`, `spike == 0`), (c) C6 at the next row expects
//! `1 - 1 = 0`, matching the honest row. It moves no spike, no voltage, no
//! chain fold — the executed state it claims is IDENTICAL to the honest
//! one. Fraud verdicts are unaffected: any tape with a real lie still has
//! `violations > 0`, and painting ghosts on top of it rescues nothing.
//! v1 ships this freedom as a published, hashed set of 396 byte offsets;
//! the v2 fold should commit `r_before` too (fold = `sha256(spikes |
//! v_after | r_before)`), which retires the class at the root.
//!
//! Bit-exact mirror of `scripts/expansion_check.py` [hardening].

use crate::attest;
use crate::connectome::{generate, Connectome};
use crate::divan::council;
use crate::erasure;
use crate::league;
use crate::sha256::sha256;
use crate::tape::{decode_tape, judge};
use crate::tournament::{panel_digest, PANEL_SIZE};

/// Full report of the ACT-1 tape sweep.
#[derive(Clone, Debug)]
pub struct TapeSweepReport {
    /// One single-bit flip per byte of the tape.
    pub mutations: usize,
    /// Mutants that no longer decode (magic/count/length damage).
    pub malformed: usize,
    /// Mutants caught by the C1–C7 constraint kill switch.
    pub viol_caught: usize,
    /// Mutants with 0 violations caught by the chain fold (the tick bytes).
    pub fold_caught: usize,
    /// Byte offsets of the mutants that pass BOTH kill switches
    /// (the pinned ghost-refractory inventory, ascending).
    pub escaped_offsets: Vec<u32>,
    /// `sha256` over the escaped offset list (LE u32 concatenation).
    pub escape_offsets_sha256: [u8; 32],
    /// Strict prefixes that still decode (must be 0).
    pub trunc_decodable: usize,
    /// Whether the tape plus one garbage byte decodes (must be 0).
    pub append_decodable: usize,
}

/// Flips one bit of every tape byte in turn, classifies the mutant, then
/// restores the byte. `published_head` is the chain head the verifier
/// expects for the tape's last tick.
#[must_use]
pub fn tape_adversary_sweep(
    conn: &Connectome,
    tape_bytes: &[u8],
    prev_head: &[u8; 32],
    published_head: &[u8; 32],
) -> TapeSweepReport {
    let mut buf = tape_bytes.to_vec();
    let mut malformed = 0;
    let mut viol_caught = 0;
    let mut fold_caught = 0;
    let mut escaped_offsets = Vec::new();
    for o in 0..buf.len() {
        let orig = buf[o];
        buf[o] = orig ^ 1;
        match judge(conn, &buf, prev_head) {
            None => malformed += 1,
            Some((viol, fold)) => {
                if viol > 0 {
                    viol_caught += 1;
                } else if fold != *published_head {
                    fold_caught += 1;
                } else {
                    escaped_offsets.push(o as u32);
                }
            }
        }
        buf[o] = orig;
    }
    let mut obytes = Vec::with_capacity(escaped_offsets.len() * 4);
    for o in &escaped_offsets {
        obytes.extend_from_slice(&o.to_le_bytes());
    }
    let trunc_decodable = (0..tape_bytes.len())
        .filter(|l| decode_tape(conn.total, &tape_bytes[..*l]).is_some())
        .count();
    let mut ext = tape_bytes.to_vec();
    ext.push(0);
    TapeSweepReport {
        mutations: tape_bytes.len(),
        malformed,
        viol_caught,
        fold_caught,
        escaped_offsets,
        escape_offsets_sha256: sha256(&obytes),
        trunc_decodable,
        append_decodable: usize::from(decode_tape(conn.total, &ext).is_some()),
    }
}

/// Erasure sweep result.
#[derive(Clone, Copy, Debug)]
pub struct ErasureHardReport {
    /// Chunk positions whose bit flip moves all K answers (of 32).
    pub chunk_avalanche: usize,
    /// Answers moved by stepping the epoch one up (of K).
    pub epoch_moves: usize,
}

/// Full-position avalanche: every chunk of the toy blob, not just 13.
#[must_use]
pub fn erasure_adversary_sweep(conn: &Connectome) -> ErasureHardReport {
    let blob = erasure::toy_blob();
    let root = erasure::erasure_root(&blob);
    let base = erasure::erasure_answers(conn, &root, 3);
    let mut chunk_avalanche = 0;
    for (ci, _chunk) in blob.iter().enumerate() {
        let mut m = blob.clone();
        m[ci][0] ^= 1;
        let r2 = erasure::erasure_root(&m);
        let a2 = erasure::erasure_answers(conn, &r2, 3);
        if base.iter().zip(a2.iter()).all(|(x, y)| x != y) {
            chunk_avalanche += 1;
        }
    }
    let e4 = erasure::erasure_answers(conn, &root, 4);
    let epoch_moves = base.iter().zip(e4.iter()).filter(|(x, y)| x != y).count();
    ErasureHardReport {
        chunk_avalanche,
        epoch_moves,
    }
}

/// Attestation sweep result.
#[derive(Clone, Copy, Debug)]
pub struct AttestHardReport {
    /// Nonce byte flips that move L1 (of 32).
    pub nonce_avalanche: usize,
    /// Mutated challenges whose ladder re-heads itself (of 32).
    pub ladder_robust: usize,
    /// A peer flip moves L2.
    pub peer_binding: bool,
}

/// Binding sweep: the answer must move with every input bit, and the
/// L1/L2 ladder must stay self-consistent for every mutated challenge.
#[must_use]
pub fn attest_adversary_sweep(conn: &Connectome) -> AttestHardReport {
    let nonce0 = [0xA5u8; 32];
    let ch0 = attest::challenge(b"fly-peer-01", 7, &nonce0);
    let l1_base = attest::attest_l1(conn, &ch0);
    let l2_base = attest::attest_l2(conn, &ch0);
    let mut nonce_avalanche = 0;
    let mut ladder_robust = 0;
    for (nb, _byte) in nonce0.iter().enumerate() {
        let mut nn = nonce0;
        nn[nb] ^= 1;
        let ch = attest::challenge(b"fly-peer-01", 7, &nn);
        if attest::attest_l1(conn, &ch) != l1_base {
            nonce_avalanche += 1;
        }
        if attest::ladder_consistent(conn, &ch) {
            ladder_robust += 1;
        }
    }
    let chp = attest::challenge(b"fly-peer-02", 7, &nonce0);
    AttestHardReport {
        nonce_avalanche,
        ladder_robust,
        peer_binding: attest::attest_l2(conn, &chp) != l2_base,
    }
}

/// League sweep result.
#[derive(Clone, Debug)]
pub struct LeagueHardReport {
    /// Re-running one council reproduces anchors and verdicts exactly.
    pub council_replay_identical: bool,
    /// Feeding the rows in reverse order yields the same standing.
    pub shuffle_stable: bool,
    /// The season actually contains an abstention tie (rule exercised).
    pub tie_engaged: bool,
    /// The standing is a permutation of the four seeds (no loss/dup).
    pub table_total_order: bool,
    /// Councils played in the season (seeds x panel).
    pub total_councils: usize,
    /// Total abstentions over the whole season.
    pub abstain_sum: u32,
}

/// Invariants of the frozen season, probed as an adversary would.
#[must_use]
pub fn league_adversary_sweep() -> LeagueHardReport {
    let rows = league::play_season();
    let table = league::standing(&rows);
    let conn_ref = generate(1, 16, league::LEAGUE_SEEDS[0]);
    let d0 = panel_digest(0);
    let r1 = council(&conn_ref, &d0);
    let r2 = council(&conn_ref, &d0);
    let mut rev = rows.clone();
    rev.reverse();
    let mut abst: Vec<u32> = rows.iter().map(|r| r.abstains).collect();
    abst.sort_unstable();
    abst.dedup();
    let mut ts = table.clone();
    ts.sort_unstable();
    ts.dedup();
    LeagueHardReport {
        council_replay_identical: r1.anchors == r2.anchors && r1.verdicts == r2.verdicts,
        shuffle_stable: league::standing(&rev) == table,
        tie_engaged: abst.len() < rows.len(),
        table_total_order: ts.len() == league::LEAGUE_SEEDS.len(),
        total_councils: league::LEAGUE_SEEDS.len() * PANEL_SIZE,
        abstain_sum: rows.iter().map(|r| r.abstains).sum(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectome::DEFAULT_SEED;
    use crate::oracle::{sentinel_stimulus, SENTINEL_TICKS};
    use crate::sha256::hex32;
    use crate::sim::{run, run_log};
    use crate::tape::encode_tape;

    fn honest_window() -> (Connectome, Vec<u8>, [u8; 32], [u8; 32]) {
        let c = generate(1, 16, DEFAULT_SEED);
        let stim = sentinel_stimulus(&c, &[0xffu8; 32]);
        let audited = run(&c, SENTINEL_TICKS, &stim, true);
        let log = run_log(&c, SENTINEL_TICKS, &stim);
        let rows = audited.rows.clone().unwrap_or_default();
        let tape = encode_tape(c.total, 23, &rows);
        (c, tape, log[22], log[23])
    }

    #[test]
    fn the_forger_gets_exactly_396_ghost_bytes_and_nothing_else() {
        let (c, tape, prev, head) = honest_window();
        let rep = tape_adversary_sweep(&c, &tape, &prev, &head);
        assert_eq!(rep.mutations, 24_708);
        assert_eq!(rep.malformed, 8, "magic and count bytes die at the door");
        assert_eq!(rep.viol_caught, 24_300);
        assert_eq!(rep.fold_caught, 4, "the tick bytes are fold-only kills");
        assert_eq!(rep.escaped_offsets.len(), 396);
        assert_eq!(
            hex32(&rep.escape_offsets_sha256),
            "52bc5976361948cd0c125cd4f86e78963ed7e2b4efd152d7f1b15c332c1f08a0",
            "the forger's full freedom is this hashed offset list"
        );
        assert_eq!(rep.trunc_decodable, 0);
        assert_eq!(rep.append_decodable, 0);
        // every ghost is r_before field-16 of a tick-(T-1) row: offsets sit
        // strictly inside the first half of the body at 21-row stride 16.
        assert!(rep
            .escaped_offsets
            .iter()
            .all(|o| *o >= 12 + 16 && (*o - 12) % 21 == 16 && (*o - 12) / 21 < c.total as u32));
    }

    #[test]
    fn erasure_avalanche_covers_every_chunk_position() {
        let c = generate(1, 16, DEFAULT_SEED);
        let rep = erasure_adversary_sweep(&c);
        assert_eq!(rep.chunk_avalanche, 32);
        assert_eq!(rep.epoch_moves, erasure::K);
    }

    #[test]
    fn attestation_binding_is_total_and_the_ladder_reheads() {
        let c = generate(1, 16, DEFAULT_SEED);
        let rep = attest_adversary_sweep(&c);
        assert_eq!(rep.nonce_avalanche, 32);
        assert_eq!(rep.ladder_robust, 32);
        assert!(rep.peer_binding);
    }

    #[test]
    fn the_season_table_survives_replay_and_shuffle() {
        let rep = league_adversary_sweep();
        assert!(rep.council_replay_identical);
        assert!(rep.shuffle_stable);
        assert!(rep.tie_engaged, "5==5 is the documented tie-break case");
        assert!(rep.table_total_order);
        assert_eq!(rep.total_councils, 32);
        assert_eq!(rep.abstain_sum, 20);
    }
}
