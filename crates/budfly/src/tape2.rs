//! ACT-2 — the ghost-reaper: the arbitration tape on the v2 chain.
//!
//! The ACT-1 adversary sweep (see `hardening.rs`, pinned) proved the
//! forger's ONLY freedom is the "ghost refractory": an `r_before` flip at
//! the window's first tick, invisible because the v1 fold commits neither
//! the refractory counter nor the window's left tick. ACT-2 retires the
//! class architecturally — v1 stays frozen, nothing is retrodden:
//!
//! 1. NEW CHAIN (`fold2`): the refractory counter enters the committed
//!    state.
//!
//! ```text
//! head2(t) = sha256(head2(t-1) | t be8 | sha256(spike bytes)
//!                  | sha256(v_after LE) | sha256(r_before LE))
//! ```
//!
//! 2. BOTH window ticks are judged against their published head2 anchors:
//!    `fold2(T-1) == head2(T(T-1))` AND `fold2(T) == head2(T)`. A ghost at
//!    the left edge dies on the first anchor.
//!
//! The pinned re-run of the full 24,708-mutant sweep under `judge2`:
//! `8 malformed | 24,300 viol | 400 fold2 | 0 escaped` — the escape set is
//! EMPTY (its sha256 is the empty-list hash `e3b0c442…`). Wire layout is
//! ACT-1's with magic `ACT2` (same 24,708 bytes for the frozen fixture).
//!
//! Bit-exact mirror of `scripts/expansion_check.py` [act2].

use crate::connectome::Connectome;
use crate::sha256::sha256;
use crate::sim::TraceRow;
use crate::tape::{constraint_violations, ActTape, HEADER_BYTES};

/// Magic prefix of the v2 tape (frozen).
pub const MAGIC2: [u8; 4] = *b"ACT2";

/// Encodes the window rows exactly as ACT-1, under the v2 magic.
#[must_use]
pub fn encode_tape2(total: usize, tick: u32, rows: &[Vec<TraceRow>]) -> Vec<u8> {
    let mut out = crate::tape::encode_tape(total, tick, rows);
    out[..4].copy_from_slice(&MAGIC2);
    out
}

/// Decodes a v2 tape (ACT-1 rules, `ACT2` magic), or `None` if malformed.
#[must_use]
pub fn decode_tape2(total: usize, bytes: &[u8]) -> Option<ActTape> {
    if bytes.len() < HEADER_BYTES {
        return None;
    }
    if bytes[..4] != MAGIC2 {
        return None;
    }
    let mut patched = bytes.to_vec();
    patched[..4].copy_from_slice(&crate::tape::MAGIC);
    crate::tape::decode_tape(total, &patched)
}

/// The v2 fold over exactly one tick's rows (`rows.len() >= total` is not
/// required; the church of `take(total)` matches the python mirror).
#[must_use]
pub fn fold2_one(tick: u32, prev_head: &[u8; 32], rows: &[TraceRow]) -> [u8; 32] {
    let mut spike_bits = Vec::new();
    let mut vbytes = Vec::new();
    let mut rbytes = Vec::new();
    for row in rows.iter() {
        spike_bits.push(row.spike);
        vbytes.extend_from_slice(&row.v_after.to_le_bytes());
        rbytes.extend_from_slice(&row.r_before.to_le_bytes());
    }
    let sh = sha256(&spike_bits);
    let vh = sha256(&vbytes);
    let rh = sha256(&rbytes);
    let mut msg = Vec::with_capacity(32 + 8 + 96);
    msg.extend_from_slice(prev_head);
    msg.extend_from_slice(&u64::from(tick).to_be_bytes());
    msg.extend_from_slice(&sh);
    msg.extend_from_slice(&vh);
    msg.extend_from_slice(&rh);
    sha256(&msg)
}

/// The whole v2 chain of an audited run, derived from its trace rows
/// (equivalent to the on-line fold by the same argument as ACT-1's fold).
#[must_use]
pub fn chain2_from_rows(total: usize, rows: &[Vec<TraceRow>]) -> Vec<[u8; 32]> {
    let ticks = rows.first().map_or(0, Vec::len);
    let mut head = [0u8; 32];
    let mut out = Vec::with_capacity(ticks);
    for t in 0..ticks {
        let one: Vec<TraceRow> = rows.iter().take(total).map(|r| r[t]).collect();
        head = fold2_one(one.first().map_or(0, |r| r.tick), &head, &one);
        out.push(head);
    }
    out
}

/// The v2 verdict: constraint count plus both anchor checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tape2Verdict {
    /// C1–C7 violations over the taped window (0 = internally consistent).
    pub violations: usize,
    /// `fold2(T-1)` matches the published `head2(T-1)`.
    pub ok_tm1: bool,
    /// `fold2(T)` matches the published `head2(T)`.
    pub ok_t: bool,
}

/// Full v2 judgement: decode, constraints, then BOTH anchor checks.
#[must_use]
pub fn judge2(
    conn: &Connectome,
    bytes: &[u8],
    a_tm2: &[u8; 32],
    a_tm1: &[u8; 32],
    a_t: &[u8; 32],
) -> Option<Tape2Verdict> {
    let tape = decode_tape2(conn.total, bytes)?;
    let violations = constraint_violations(conn.total, &tape);
    let base = tape.tick.saturating_sub(1);
    let w1 = tape.rows.get(..conn.total).unwrap_or(&[]);
    let w2 = tape
        .rows
        .get(conn.total..conn.total.saturating_mul(2))
        .unwrap_or(&[]);
    Some(Tape2Verdict {
        violations,
        ok_tm1: fold2_one(base, a_tm2, w1) == *a_tm1,
        ok_t: fold2_one(base.saturating_add(1), a_tm1, w2) == *a_t,
    })
}

/// The ghost-reaper sweep: same classification precedence as the v1 sweep.
#[derive(Clone, Debug)]
pub struct GhostSweep {
    /// One single-bit flip per byte.
    pub mutations: usize,
    /// Mutants that no longer decode.
    pub malformed: usize,
    /// Mutants caught by the constraint kill.
    pub viol_caught: usize,
    /// Mutants caught by either anchor check.
    pub fold_caught: usize,
    /// Byte offsets of mutants passing EVERYTHING (v2 target: empty).
    pub escaped_offsets: Vec<u32>,
    /// `sha256` over the escaped offset list (LE u32 concatenation).
    pub escape_offsets_sha256: [u8; 32],
}

/// Re-runs all 24,708 single-bit mutants under [`judge2`].
#[must_use]
pub fn tape2_adversary_sweep(
    conn: &Connectome,
    tape_bytes: &[u8],
    a_tm2: &[u8; 32],
    a_tm1: &[u8; 32],
    a_t: &[u8; 32],
) -> GhostSweep {
    let mut buf = tape_bytes.to_vec();
    let mut malformed = 0;
    let mut viol_caught = 0;
    let mut fold_caught = 0;
    let mut escaped_offsets = Vec::new();
    for o in 0..buf.len() {
        let orig = buf[o];
        buf[o] = orig ^ 1;
        match judge2(conn, &buf, a_tm2, a_tm1, a_t) {
            None => malformed += 1,
            Some(v) => {
                if v.violations > 0 {
                    viol_caught += 1;
                } else if !(v.ok_tm1 && v.ok_t) {
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
    GhostSweep {
        mutations: tape_bytes.len(),
        malformed,
        viol_caught,
        fold_caught,
        escaped_offsets,
        escape_offsets_sha256: sha256(&obytes),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectome::{generate, Region, DEFAULT_SEED};
    use crate::oracle::{sentinel_stimulus, SENTINEL_TICKS};
    use crate::sha256::hex32;
    use crate::sim::run;

    fn window() -> (Connectome, Vec<Vec<TraceRow>>, Vec<[u8; 32]>) {
        let c = generate(1, 16, DEFAULT_SEED);
        let stim = sentinel_stimulus(&c, &[0xffu8; 32]);
        let audited = run(&c, SENTINEL_TICKS, &stim, true);
        let rows = audited.rows.clone().unwrap_or_default();
        let log2 = chain2_from_rows(c.total, &rows);
        (c, rows, log2)
    }

    #[test]
    fn the_honest_act2_tape_passes_both_anchor_checks() {
        let (c, rows, log2) = window();
        let tape = encode_tape2(c.total, 23, &rows);
        assert_eq!(tape.len(), 24_708);
        assert_eq!(
            hex32(&sha256(&tape)),
            "448c04114d0c88c8263cbfbaa8cbb245c3bc706ce9663abed71d6e5a3878d0d0"
        );
        assert_eq!(
            hex32(&log2[22]),
            "a874e27e0a2277a1e6484dfe5e969839dacece50a8c8aca9d645c003b7be6dd9"
        );
        assert_eq!(
            hex32(&log2[23]),
            "6fad2f4c983d0134ebe06b422907a4d065766bfb0d61ab6ea08c5dd8a631c527"
        );
        let v = judge2(&c, &tape, &log2[21], &log2[22], &log2[23]);
        let v = v.unwrap_or(Tape2Verdict {
            violations: usize::MAX,
            ok_tm1: false,
            ok_t: false,
        });
        assert_eq!(v.violations, 0);
        assert!(v.ok_tm1 && v.ok_t);
    }

    #[test]
    fn the_lazy_liar_still_dies_on_act2() {
        let (c, mut rows, log2) = window();
        let tam = c.offset(Region::CxEb) + 3;
        for (g, nr) in rows.iter_mut().enumerate() {
            nr[23].v_after = 0;
            if g == tam {
                nr[23].spike = 1 - nr[23].spike;
            }
        }
        let tape = encode_tape2(c.total, 23, &rows);
        let v = judge2(&c, &tape, &log2[21], &log2[22], &log2[23]);
        let v = v.unwrap_or(Tape2Verdict {
            violations: usize::MAX,
            ok_tm1: true,
            ok_t: true,
        });
        assert_eq!(v.violations, 192);
        assert!(!(v.ok_tm1 && v.ok_t), "both switches stay armed on v2");
    }

    #[test]
    fn the_ghost_reaper_leaves_zero_escapes() {
        let (c, rows, log2) = window();
        let tape = encode_tape2(c.total, 23, &rows);
        let rep = tape2_adversary_sweep(&c, &tape, &log2[21], &log2[22], &log2[23]);
        assert_eq!(rep.mutations, 24_708);
        assert_eq!(rep.malformed, 8);
        assert_eq!(rep.viol_caught, 24_300);
        assert_eq!(rep.fold_caught, 400, "396 ghosts + 4 tick bytes, all dead");
        assert!(rep.escaped_offsets.is_empty(), "the ghost class is retired");
        assert_eq!(
            hex32(&rep.escape_offsets_sha256),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "the escape inventory is the empty set"
        );
    }
}
