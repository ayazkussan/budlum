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
}
