//! ACT-1 — the Arbitration Check Tape: a one-tick fraud-proof that a
//! verifier can judge WITHOUT the connectome.
//!
//! Where the bisection lands on a disputed tick `T`, the honest executor
//! answers with this tape: the transition rows of `T-1` and `T` (the window
//! the C1–C7 catalogue needs — C6 glues the two ticks; genesis only when
//! `T <= 1`). Wire layout (frozen):
//!
//! ```text
//!   "ACT1"      4 B
//!   tick        u32 LE
//!   row_count   u32 LE
//!   rows, ordered by (tick, gid):
//!     v_before i32 LE | i_ext i32 LE | i_syn i32 LE |
//!     v_after  i32 LE | r_before u32 LE | spike u8      (21 B/row)
//! ```
//!
//! TWO INDEPENDENT KILL SWITCHES, no execution of the connectome anywhere:
//!
//! (i)  constraint kill — evaluate the C1–C7 catalogue over the window rows
//!      (the same `air` predicates a future STARK would arithmetize);
//! (ii) fold kill — recompute `head(T) = sha256(prev_head | T be8 |
//!      sha256(spike_bytes) | sha256(v_after LE))` and compare to the
//!      published chain head.
//!
//! The frozen lesson (pinned): the lazy liar of `dispute.rs` fails BOTH —
//! 192 constraint violations AND a fold mismatch.
//!
//! Bit-exact mirror of `scripts/expansion_check.py` [act].

use crate::connectome::Connectome;
use crate::sha256::sha256;
use crate::sim::TraceRow;

/// Magic prefix (frozen).
pub const MAGIC: [u8; 4] = *b"ACT1";
/// Bytes per taped row (frozen).
pub const ROW_BYTES: usize = 21;
/// Header size in bytes (frozen).
pub const HEADER_BYTES: usize = 12;

/// A decoded ACT-1 tape.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActTape {
    /// The disputed tick (rows cover `tick - 1` and `tick`, or just 0).
    pub tick: u32,
    /// Window rows ordered by (tick, gid).
    pub rows: Vec<TraceRow>,
}

/// Encodes the window rows of an audited run for disputed `tick`.
#[must_use]
pub fn encode_tape(total: usize, tick: u32, rows: &[Vec<TraceRow>]) -> Vec<u8> {
    let first = tick.saturating_sub(1);
    let count = u32::try_from((tick - first + 1) as usize * total).unwrap_or(0);
    let mut out = Vec::with_capacity(HEADER_BYTES + count as usize * ROW_BYTES);
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&tick.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    for t in first..=tick {
        for neuron_rows in rows.iter().take(total) {
            let r = &neuron_rows[t as usize];
            out.extend_from_slice(&r.v_before.to_le_bytes());
            out.extend_from_slice(&r.i_ext.to_le_bytes());
            out.extend_from_slice(&r.i_syn.to_le_bytes());
            out.extend_from_slice(&r.v_after.to_le_bytes());
            out.extend_from_slice(&r.r_before.to_le_bytes());
            out.push(r.spike);
        }
    }
    out
}

/// Decodes a tape against the expected neuron count, or `None` if malformed.
#[must_use]
pub fn decode_tape(total: usize, bytes: &[u8]) -> Option<ActTape> {
    if bytes.len() < HEADER_BYTES || bytes[..4] != MAGIC {
        return None;
    }
    let tick = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let count = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    if bytes.len() != HEADER_BYTES + count * ROW_BYTES {
        return None;
    }
    let ticks_in_tape = count.checked_div(total).unwrap_or(0);
    let expected_ticks = if tick == 0 { 1 } else { 2 };
    if ticks_in_tape != expected_ticks {
        return None;
    }
    let mut rows = Vec::with_capacity(count);
    for i in 0..count {
        let s = HEADER_BYTES + i * ROW_BYTES;
        let b = &bytes[s..s + ROW_BYTES];
        let i32at = |o: usize| i32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
        rows.push(TraceRow {
            tick: tick.saturating_sub(1) + (i / total) as u32,
            v_before: i32at(0),
            i_ext: i32at(4),
            i_syn: i32at(8),
            v_after: i32at(12),
            spike: b[20],
            r_before: u32::from_le_bytes([b[16], b[17], b[18], b[19]]),
        });
    }
    Some(ActTape { tick, rows })
}

/// Kill switch (i): C1–C7 over the taped window, grouped per neuron.
/// Returns the violation count; 0 means the tape is internally consistent.
#[must_use]
pub fn constraint_violations(total: usize, tape: &ActTape) -> usize {
    let first = tape.tick.saturating_sub(1);
    let ticks_in_tape = (tape.tick - first + 1) as usize;
    let mut by_neuron: Vec<Vec<TraceRow>> = vec![Vec::new(); total];
    for (i, row) in tape.rows.iter().enumerate() {
        if ticks_in_tape > 0 {
            by_neuron[i % total].push(*row);
        }
    }
    crate::air::count_violations(&by_neuron)
}

/// Kill switch (ii): recompute the chain fold for the tape's LAST tick from
/// its own rows. The caller compares this against the published head.
#[must_use]
pub fn recompute_fold(total: usize, tape: &ActTape, prev_head: &[u8; 32]) -> [u8; 32] {
    let first_row = tape.rows.len().saturating_sub(total);
    let mut spike_bits = vec![0u8; total];
    let mut vbytes = Vec::with_capacity(total * 4);
    for (i, row) in tape.rows.iter().skip(first_row).enumerate() {
        if i < total {
            spike_bits[i] = row.spike;
        }
    }
    for row in tape.rows.iter().skip(first_row).take(total) {
        vbytes.extend_from_slice(&row.v_after.to_le_bytes());
    }
    let sh = sha256(&spike_bits);
    let vh = sha256(&vbytes);
    let mut msg = Vec::with_capacity(32 + 8 + 32 + 32);
    msg.extend_from_slice(prev_head);
    msg.extend_from_slice(&tape.tick.to_be_bytes());
    msg.extend_from_slice(&sh);
    msg.extend_from_slice(&vh);
    sha256(&msg)
}

/// Full judgement: both kill switches, returning (violations, fold head).
///
/// The connectome is only needed for `total` bookkeeping; no neuron fires.
#[must_use]
pub fn judge(conn: &Connectome, bytes: &[u8], prev_head: &[u8; 32]) -> Option<(usize, [u8; 32])> {
    let tape = decode_tape(conn.total, bytes)?;
    Some((
        constraint_violations(conn.total, &tape),
        recompute_fold(conn.total, &tape, prev_head),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectome::{generate, DEFAULT_SEED};
    use crate::oracle::{sentinel_stimulus, SENTINEL_TICKS};
    use crate::sim::{run, run_log};

    #[test]
    fn the_honest_tape_survives_both_kill_switches() {
        let c = generate(1, 16, DEFAULT_SEED);
        let stim = sentinel_stimulus(&c, &[0xffu8; 32]);
        let audited = run(&c, SENTINEL_TICKS, &stim, true);
        let log = run_log(&c, SENTINEL_TICKS, &stim);
        let rows = audited.rows.clone().unwrap_or_default();
        let tape = encode_tape(c.total, 23, &rows);
        assert_eq!(tape.len(), 24_708);
        assert_eq!(
            crate::sha256::hex32(&sha256(&tape)),
            "5ddaf94829af4cbb21983e0c14c8e30c8cd8b618135bc1c8f451ba613e95bd30"
        );
        let (viol, fold) = judge(&c, &tape, &log[22]).unwrap_or((usize::MAX, [0u8; 32]));
        assert_eq!(viol, 0);
        assert_eq!(fold, log[23], "fold must land on the published chain head");
    }

    #[test]
    fn the_lazy_liar_tape_fails_both_switches_independently() {
        let c = generate(1, 16, DEFAULT_SEED);
        let stim = sentinel_stimulus(&c, &[0xffu8; 32]);
        let audited = run(&c, SENTINEL_TICKS, &stim, true);
        let log = run_log(&c, SENTINEL_TICKS, &stim);
        let mut rows = audited.rows.clone().unwrap_or_default();
        // lazy liar twin: flip CxEb+3 spike at 23, all v_after bytes zero
        let tam_gid = c.offset(crate::connectome::Region::CxEb) + 3;
        for (g, neuron_rows) in rows.iter_mut().enumerate() {
            let row = &mut neuron_rows[23];
            row.v_after = 0;
            if g == tam_gid {
                row.spike = 1 - row.spike;
            }
        }
        let tape = encode_tape(c.total, 23, &rows);
        assert_eq!(
            crate::sha256::hex32(&sha256(&tape)),
            "d29ee09013792abaf8207d72a195af2501c48aad9c671dbd5281594d7893e097"
        );
        let (viol, fold) = judge(&c, &tape, &log[22]).unwrap_or((usize::MAX, [0u8; 32]));
        assert_eq!(viol, 192, "constraint kill fires without the chain");
        assert_ne!(fold, log[23], "fold kill fires without the constraints");
    }

    #[test]
    fn garbage_is_not_a_tape() {
        let c = generate(1, 16, DEFAULT_SEED);
        assert!(decode_tape(c.total, b"ACT!!").is_none());
        assert!(decode_tape(c.total, &[0u8; 12]).is_none());
    }
}
