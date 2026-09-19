//! Erasure experiment — the wandering prover (D2, scope-locked).
//!
//! PROTOCOL (frozen v1): the committed fact is the blob's merkle-style root
//! (toy: sha256 over concatenated 32-byte chunks). A verifier issues K = 8
//! chunk-bound canary challenges
//!
//! ```text
//! challenge = sha256("BUDFLY-ERASURE1\x00" || root || idx le16 || epoch be64)
//! ```
//!
//! and the prover answers each with the 1-tick L1 anchor. The pinned
//! avalanche: flipping ONE bit of ONE chunk moves the root, moves every
//! challenge, and changes EVERY answer.
//!
//! HONEST NEGATIVE (pinned in the manifest so nobody retrades it): this
//! proves "the prover ran the pinned map on the challenges" — it does NOT
//! prove the chunk bytes were ever read. Possession-grade soundness needs
//! the chunk bytes feeding the stimulus stream; v1 does not do that. The
//! experiment stands because the protocol and its boundary are pinned
//! together.
//!
//! Bit-exact mirror of `scripts/expansion_check.py` [erasure].

use crate::canary::challenge_canary;
use crate::connectome::Connectome;
use crate::sha256::sha256;

/// Chunk count of the frozen toy blob.
pub const CHUNK_COUNT: u16 = 32;
/// Number of challenges per round.
pub const K: usize = 8;
/// Frozen sample indices.
pub const SAMPLE_IDX: [u16; K] = [0, 5, 9, 13, 17, 21, 26, 31];
/// Domain separator of the erasure challenge (frozen).
pub const DOMAIN: &[u8] = b"BUDFLY-ERASURE1\x00";

/// Deterministic toy chunk bytes (fixture blob, not real storage).
#[must_use]
pub fn toy_chunk(idx: u16) -> [u8; 32] {
    let mut buf = Vec::with_capacity(20 + 2);
    buf.extend_from_slice(b"budfly-erasure-chunk");
    buf.extend_from_slice(&idx.to_le_bytes());
    sha256(&buf)
}

/// Toy blob of [`CHUNK_COUNT`] chunks (fixture).
#[must_use]
pub fn toy_blob() -> Vec<[u8; 32]> {
    (0..CHUNK_COUNT).map(toy_chunk).collect()
}

/// The committed fact: root over concatenated chunks.
#[must_use]
pub fn erasure_root(chunks: &[[u8; 32]]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(chunks.len() * 32);
    for c in chunks {
        buf.extend_from_slice(c);
    }
    sha256(&buf)
}

/// One chunk-bound challenge.
#[must_use]
pub fn chunk_challenge(root: &[u8; 32], idx: u16, epoch: u64) -> [u8; 32] {
    let mut buf = Vec::with_capacity(DOMAIN.len() + 32 + 2 + 8);
    buf.extend_from_slice(DOMAIN);
    buf.extend_from_slice(root);
    buf.extend_from_slice(&idx.to_le_bytes());
    buf.extend_from_slice(&epoch.to_be_bytes());
    sha256(&buf)
}

/// The prover's answer vector over the frozen sample set.
#[must_use]
pub fn erasure_answers(conn: &Connectome, root: &[u8; 32], epoch: u64) -> [[u8; 32]; K] {
    let mut out = [[0u8; 32]; K];
    for (i, idx) in SAMPLE_IDX.iter().enumerate() {
        out[i] = challenge_canary(conn, &chunk_challenge(root, *idx, epoch));
    }
    out
}

/// Number of chunks (as `usize` for indexing).
pub const CHUNKS: usize = 32;
/// Rounds of the v2 walk (frozen: 32 chunks / 8 slots).
pub const ERA2_ROUNDS: usize = 4;
/// Slots per round (frozen).
pub const ERA2_SLOTS: usize = 8;

/// The v2 round protocol's schedule: a root-bound permutation of all
/// chunks via Fisher-Yates fed by the keystream
/// `sha256(DOMAIN | root | "perm" | ctr le16)`, `i` walking 31..=1, the
/// stream consumed two bytes at a time with carry-over. Four rounds x
/// eight slots then cover EVERY chunk exactly once — full-surface coverage
/// by construction, not by coupon-collector luck.
#[must_use]
pub fn era2_permutation(root: &[u8; 32]) -> [u16; CHUNKS] {
    let mut order = [0u16; CHUNKS];
    for (i, o) in order.iter_mut().enumerate() {
        *o = i as u16;
    }
    let mut stream: Vec<u8> = Vec::new();
    let mut ctr = 0u16;
    for i in (1..CHUNKS).rev() {
        while stream.len() < 2 {
            let mut buf = Vec::with_capacity(DOMAIN.len() + 32 + 4 + 2);
            buf.extend_from_slice(DOMAIN);
            buf.extend_from_slice(root);
            buf.extend_from_slice(b"perm");
            buf.extend_from_slice(&ctr.to_le_bytes());
            stream.extend_from_slice(&sha256(&buf));
            ctr += 1;
        }
        let j = usize::from(u16::from_le_bytes([stream[0], stream[1]])) % (i + 1);
        stream.drain(..2);
        order.swap(i, j);
    }
    order
}

/// Round `r`'s chunk indices: `perm[8r..8r+8]`.
#[must_use]
pub fn era2_round_indices(root: &[u8; 32], r: usize) -> [u16; ERA2_SLOTS] {
    let perm = era2_permutation(root);
    let start = (r % ERA2_ROUNDS) * ERA2_SLOTS;
    let mut out = [0u16; ERA2_SLOTS];
    out.copy_from_slice(&perm[start..start + ERA2_SLOTS]);
    out
}

/// The prover's answers over one round of the walk.
#[must_use]
pub fn era2_round_answers(
    conn: &Connectome,
    root: &[u8; 32],
    r: usize,
    epoch: u64,
) -> [[u8; 32]; ERA2_SLOTS] {
    let mut out = [[0u8; 32]; ERA2_SLOTS];
    for (i, idx) in era2_round_indices(root, r).iter().enumerate() {
        out[i] = challenge_canary(conn, &chunk_challenge(root, *idx, epoch));
    }
    out
}

// ---- Reed-Solomon v3: a REAL code over GF(2^8) ---------------------------

/// GF(2^8) exp/log tables under the AES polynomial `0x11d`.
type GfTables = ([u8; 512], [u8; 256]);

fn gf_tables() -> GfTables {
    let mut exp = [0u8; 512];
    let mut log = [0u8; 256];
    let mut x: u32 = 1;
    let mut i = 0usize;
    while i < 255 {
        exp[i] = x as u8;
        log[x as usize] = i as u8;
        x <<= 1;
        if x & 0x100 != 0 {
            x ^= 0x11d;
        }
        i += 1;
    }
    while i < 512 {
        exp[i] = exp[i - 255];
        i += 1;
    }
    (exp, log)
}

fn gf_mul(t: &GfTables, a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
        0
    } else {
        t.0[t.1[a as usize] as usize + t.1[b as usize] as usize]
    }
}

/// Multiplicative inverse; `a` must be nonzero.
fn gf_inv(t: &GfTables, a: u8) -> u8 {
    t.0[255 - t.1[a as usize] as usize]
}

/// Data chunks carried by the RS code (frozen).
pub const RS_DATA_CHUNKS: usize = 24;
/// Coded chunks produced (frozen): any 24 rows reconstruct the data.
pub const RS_CODED_CHUNKS: usize = 32;

/// The first 24 toy chunks, promoted to data words of the code.
#[must_use]
pub fn rs_data() -> Vec<[u8; 32]> {
    toy_blob()[..RS_DATA_CHUNKS].to_vec()
}

/// Full Cauchy generator rows: `M[r][i] = inv(r ^ (64+i))` for `r` in
/// 0..32, `i` in 0..24 — any 24 rows are invertible (Cauchy property).
#[must_use]
pub fn rs_generator() -> Vec<Vec<u8>> {
    let t = gf_tables();
    (0..RS_CODED_CHUNKS)
        .map(|r| {
            (0..RS_DATA_CHUNKS)
                .map(|i| gf_inv(&t, (r ^ (64 + i)) as u8))
                .collect()
        })
        .collect()
}

/// The 32 coded chunks of the frozen data, any 24 recoverable.
#[must_use]
pub fn rs_coded_chunks() -> Vec<[u8; 32]> {
    let t = gf_tables();
    let gen = rs_generator();
    let data = rs_data();
    (0..RS_CODED_CHUNKS)
        .map(|r| {
            let mut out = [0u8; 32];
            for b in 0..32 {
                let mut acc = 0u8;
                for (i, d) in data.iter().enumerate() {
                    acc ^= gf_mul(&t, gen[r][i], d[b]);
                }
                out[b] = acc;
            }
            out
        })
        .collect()
}

/// Recover the 24 data chunks from 24 known coded rows
/// (`(row_index, chunk)`), by GF(256) Gauss-Jordan inversion of that slice
/// of the generator, or `None` if the slice cannot be inverted.
#[must_use]
pub fn rs_recover(known: &[(usize, [u8; 32])]) -> Option<Vec<[u8; 32]>> {
    if known.len() != RS_DATA_CHUNKS {
        return None; // exactly k rows reconstruct
    }
    let t = gf_tables();
    let gen = rs_generator();
    let mut known = known.to_vec();
    known.sort_by_key(|(idx, _)| *idx);
    let mut a: Vec<Vec<u8>> = known.iter().map(|(r, _)| gen[*r].clone()).collect();
    let mut inv: Vec<Vec<u8>> = (0..RS_DATA_CHUNKS)
        .map(|i| {
            let mut row = vec![0u8; RS_DATA_CHUNKS];
            row[i] = 1;
            row
        })
        .collect();
    for col in 0..RS_DATA_CHUNKS {
        let piv = (col..RS_DATA_CHUNKS).find(|&r| a[r][col] != 0)?; // Cauchy: always
        a.swap(col, piv);
        inv.swap(col, piv);
        let d = gf_inv(&t, a[col][col]);
        for j in 0..RS_DATA_CHUNKS {
            a[col][j] = gf_mul(&t, a[col][j], d);
            inv[col][j] = gf_mul(&t, inv[col][j], d);
        }
        for r in 0..RS_DATA_CHUNKS {
            if r != col && a[r][col] != 0 {
                let f = a[r][col];
                for j in 0..RS_DATA_CHUNKS {
                    a[r][j] ^= gf_mul(&t, f, a[col][j]);
                    inv[r][j] ^= gf_mul(&t, f, inv[col][j]);
                }
            }
        }
    }
    Some(
        (0..RS_DATA_CHUNKS)
            .map(|i| {
                let mut out = [0u8; 32];
                for b in 0..32 {
                    let mut acc = 0u8;
                    for (k, (_, chunk)) in known.iter().enumerate() {
                        acc ^= gf_mul(&t, inv[i][k], chunk[b]);
                    }
                    out[b] = acc;
                }
                out
            })
            .collect(),
    )
}

/// The frozen deletion set for stress index `s` (8 of 32 rows deleted).
#[must_use]
pub fn rs_delete_set(s: u16) -> [usize; 8] {
    let d = sha256(&{
        let mut v = b"rs3-set".to_vec();
        v.extend_from_slice(&s.to_le_bytes());
        v
    });
    let mut seen = Vec::with_capacity(8);
    for off in 0..31 {
        let x = usize::from(u16::from_le_bytes([d[off], d[off + 1]])) % RS_CODED_CHUNKS;
        if !seen.contains(&x) {
            seen.push(x);
        }
        if seen.len() == RS_CODED_CHUNKS - RS_DATA_CHUNKS {
            break;
        }
    }
    let mut out = [0usize; 8];
    out.copy_from_slice(&seen[..8]);
    out.sort_unstable();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectome::{generate, DEFAULT_SEED};
    use crate::sha256::hex32;

    #[test]
    fn the_wandering_prover_is_frozen() {
        let c = generate(1, 16, DEFAULT_SEED);
        let blob = toy_blob();
        assert_eq!(blob.len(), CHUNK_COUNT as usize);
        let root = erasure_root(&blob);
        assert_eq!(
            hex32(&root),
            "760e6ebf071924799ffa45d18583101b6cde8650fa92457356ca6ef0f5b9c0c2"
        );
        let answers = erasure_answers(&c, &root, 3);
        let expect: [&str; K] = [
            "755eaa75a8310b12",
            "5a87f98ab7f9396c",
            "868ad8cc550102bf",
            "7135b2308f2f1b16",
            "a4f173125bdac2e7",
            "c193223592a507b0",
            "b123631e2baefe83",
            "cc8aef423b239a96",
        ];
        for (a, want) in answers.iter().zip(expect.iter()) {
            let full: String = a.iter().map(|b| format!("{b:02x}")).collect();
            assert!(full.starts_with(want), "answer prefix must match the pin");
        }
        // avalanche: one flipped bit in chunk 13 moves every answer
        let mut blob2 = blob;
        blob2[13][0] ^= 1;
        let root2 = erasure_root(&blob2);
        let answers2 = erasure_answers(&c, &root2, 3);
        for (a, b) in answers.iter().zip(answers2.iter()) {
            assert_ne!(a, b, "bit flip must move every answer");
        }
    }

    #[test]
    fn the_round_protocol_walks_the_whole_surface() {
        let c = generate(1, 16, DEFAULT_SEED);
        let blob = toy_blob();
        let root = erasure_root(&blob);
        let perm = era2_permutation(&root);
        let expect: [u16; CHUNKS] = [
            12, 21, 22, 28, 2, 1, 7, 5, 0, 15, 3, 29, 24, 9, 13, 17, 16, 8, 23, 10, 31, 19, 18, 14,
            26, 6, 30, 4, 25, 11, 27, 20,
        ];
        assert_eq!(perm, expect, "the frozen Fisher-Yates schedule");
        let mut seen = perm.to_vec();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), CHUNKS, "coverage is total by construction");

        // answers over the four rounds, round-major order, hash-pinned:
        let mut all = Vec::with_capacity(ERA2_ROUNDS * ERA2_SLOTS * 32);
        for r in 0..ERA2_ROUNDS {
            for a in era2_round_answers(&c, &root, r, 3).iter() {
                all.extend_from_slice(a);
            }
        }
        assert_eq!(
            hex32(&sha256(&all)),
            "75ceca8bbc7d744979276fb3668cdee870cd120c34d0b7eee5078672a2ea41d1"
        );

        // per-round avalanche: chunk 13's bit flip moves every answer of
        // EVERY round (the schedule permutes too — the walk itself moves):
        let mut blob2 = blob;
        blob2[13][0] ^= 1;
        let root2 = erasure_root(&blob2);
        for r in 0..ERA2_ROUNDS {
            let idxs = era2_round_indices(&root, r);
            for (j, idx) in idxs.iter().enumerate() {
                let a1 = challenge_canary(&c, &chunk_challenge(&root, *idx, 3));
                let a2 = challenge_canary(&c, &chunk_challenge(&root2, *idx, 3));
                assert_ne!(a1, a2, "round {r} slot {j} must move");
            }
        }
    }

    #[test]
    fn reed_solomon_recovers_every_frozen_deletion_set() {
        // GF self-test: every nonzero a has a * inv(a) == 1:
        let t = gf_tables();
        for a in 1u16..=255 {
            assert_eq!(gf_mul(&t, a as u8, gf_inv(&t, a as u8)), 1);
        }
        let coded = rs_coded_chunks();
        let mut buf = Vec::with_capacity(RS_CODED_CHUNKS * 32);
        for c in &coded {
            buf.extend_from_slice(c);
        }
        assert_eq!(
            sha256::hex32(&sha256(&buf)),
            "8a0d21d902da911ac276dc89d2e2ce5427561fe69efd8b950a54567ff1dab179"
        );
        // the frozen deletion sets (python-pinned):
        let expect_sets: [[usize; 8]; 8] = [
            [2, 4, 9, 12, 14, 16, 17, 23],
            [1, 4, 9, 12, 13, 22, 28, 29],
            [3, 4, 7, 16, 17, 22, 24, 28],
            [11, 12, 14, 18, 21, 24, 25, 27],
            [1, 3, 10, 12, 17, 22, 24, 26],
            [5, 6, 10, 12, 13, 22, 25, 27],
            [3, 4, 9, 13, 16, 22, 27, 29],
            [5, 6, 8, 16, 19, 22, 24, 31],
        ];
        let data = rs_data();
        for (s, expect) in expect_sets.iter().enumerate() {
            let dset = rs_delete_set(s as u16);
            assert_eq!(&dset, expect, "deletion set {s} is frozen");
            let known: Vec<(usize, [u8; 32])> = (0..RS_CODED_CHUNKS)
                .filter(|r| !dset.contains(r))
                .map(|r| (r, coded[r]))
                .collect();
            assert_eq!(
                rs_recover(&known).as_deref(),
                Some(data.as_slice()),
                "set {s} must recover the data"
            );
        }
        // one tampered byte in one surviving chunk poisons the
        // reconstruction — and therefore rings the bell:
        let dset = rs_delete_set(0);
        let mut known: Vec<(usize, [u8; 32])> = (0..RS_CODED_CHUNKS)
            .filter(|r| !dset.contains(r))
            .map(|r| (r, coded[r]))
            .collect();
        known[0].1[0] ^= 1;
        let tampered = rs_recover(&known);
        assert_ne!(
            tampered.as_deref(),
            Some(data.as_slice()),
            "a bitflip in a survivor is detectable"
        );
    }
}
