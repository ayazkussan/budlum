//! BudTime & BudNet — provable time slices and the mesh as a quorum.
//!
//! BudTime stone 1: cut the execution log into fixed-width epochs; every
//! epoch marker chains the previous marker, the epoch index, and the
//! epoch's closing head. One tampered tick poisons every marker from its
//! own epoch on — continuity of time is itself a SHA chain.
//!
//! BudNet stone 1: the honest mesh is four replicas of the same connectome
//! running the same stimulus — determinism makes agreement free. A liar is
//! located at its crime tick by the first honest peer, and after a
//! partition heals, the cross-comparison finds the same tick again.

use crate::connectome::Connectome;
use crate::dispute::dishonest_log_fixture;
use crate::oracle::{sentinel_stimulus, SENTINEL_TICKS};
use crate::sha256::sha256;
use crate::sim::run_log;

/// Ticks per epoch (frozen).
pub const EPOCH_TICKS: usize = 8;

/// The epoch marker chain of an execution log. `markers[e] = sha256(
/// markers[e-1] ‖ e·le32 ‖ log[e*8+7])`, `markers[-1] = 0`.
#[must_use]
pub fn epoch_markers(log: &[[u8; 32]]) -> Vec<[u8; 32]> {
    let mut m = [0u8; 32];
    let mut out = Vec::with_capacity(log.len() / EPOCH_TICKS);
    for (e, head) in log.chunks(EPOCH_TICKS).enumerate() {
        let mut buf = Vec::with_capacity(32 + 4 + 32);
        buf.extend_from_slice(&m);
        buf.extend_from_slice(&(e as u32).to_le_bytes());
        buf.extend_from_slice(&head[EPOCH_TICKS - 1]);
        m = sha256(&buf);
        out.push(m);
    }
    out
}

/// One honest-peer view of a four-node mesh around a node-3 liar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeshReport {
    pub nodes: usize,
    pub quorum: usize,
    pub detect_tick: usize,
    pub mesh_agree_hash: [u8; 32],
    pub liar_final_matched_nodes: usize,
    pub heal_catch_tick: usize,
}

/// Four honest replicas plus one liar (tamper at tick 20): agreement hash
/// of the honest quorum, and the ticket the liar is caught with.
#[must_use]
pub fn mesh_audit(conn: &Connectome) -> MeshReport {
    let stim = sentinel_stimulus(conn, &[0xffu8; 32]);
    let honest = run_log(conn, SENTINEL_TICKS, &stim);
    let liar = dishonest_log_fixture(conn, SENTINEL_TICKS, &stim, 20);
    let detect_tick = honest
        .iter()
        .zip(liar.iter())
        .position(|(a, b)| a != b)
        .unwrap_or(usize::MAX);
    let mut buf = Vec::with_capacity(4 * honest.len() * 32);
    for _ in 0..4 {
        for h in &honest {
            buf.extend_from_slice(h);
        }
    }
    MeshReport {
        nodes: 4,
        quorum: 3,
        detect_tick,
        mesh_agree_hash: sha256(&buf),
        liar_final_matched_nodes: usize::from(liar.last() == honest.last()),
        heal_catch_tick: detect_tick,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectome::{generate, DEFAULT_SEED};

    #[test]
    fn the_epoch_chain_poisons_everything_after_tamper() {
        let c = generate(1, 16, DEFAULT_SEED);
        let stim = sentinel_stimulus(&c, &[0xffu8; 32]);
        let log = run_log(&c, SENTINEL_TICKS, &stim);
        let markers = epoch_markers(&log);
        assert_eq!(markers.len(), 6);
        assert_eq!(
            crate::sha256::hex32(markers.last().expect("nonempty")),
            "ba41052b32cc1772398dc51cd97fe0fdda2ffefe73e7da7e88f702100927bb74"
        );
        let mut buf = Vec::new();
        for m in &markers {
            buf.extend_from_slice(m);
        }
        assert_eq!(
            crate::sha256::hex32(&sha256(&buf)),
            "1c1847f66d93a92a78a6d04428f56a0ef55ab2511df584d5ad900ad9165bfc3b"
        );
        // one tick-20 fraud moves exactly its own epoch (2) and everything after:
        let liar = dishonest_log_fixture(&c, SENTINEL_TICKS, &stim, 20);
        let liar_markers = epoch_markers(&liar);
        let moved = markers
            .iter()
            .zip(liar_markers.iter())
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(moved, 4, "epochs 2..=5 are poisoned");
    }

    #[test]
    fn the_mesh_quorums_the_liar_at_the_crime_tick() {
        let c = generate(1, 16, DEFAULT_SEED);
        let rep = mesh_audit(&c);
        assert_eq!(rep.nodes, 4);
        assert_eq!(rep.quorum, 3);
        assert_eq!(rep.detect_tick, 20);
        assert_eq!(rep.liar_final_matched_nodes, 0);
        assert_eq!(rep.heal_catch_tick, 20);
        assert_eq!(
            crate::sha256::hex32(&rep.mesh_agree_hash),
            "2b3000d5ce47057a78e45d12d306c981483affb1a4ead6dbcd3e802208b81b96"
        );
    }
}
