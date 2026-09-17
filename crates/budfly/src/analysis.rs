//! Graph census of the frozen connectome: exact, integer-only, pinned.
//!
//! Bit-exact mirror of `scripts/expansion_check.py` [census]. Answers "what
//! does the map look like as a graph" — degree sums per region and BFS
//! geodesics from the left lamina to the command layer — before any claim
//! about dynamics.

use crate::connectome::{Connectome, Region, N_REGIONS};
use crate::sha256::{hex32, sha256};
use std::collections::VecDeque;

/// Out-degree sum per region, index order.
#[must_use]
pub fn out_degree_sums(conn: &Connectome) -> [usize; N_REGIONS] {
    let mut out = [0usize; N_REGIONS];
    for e in &conn.edges {
        out[conn.region_of(e.pre as usize).idx()] += 1;
    }
    out
}

/// In-degree sum per region, index order.
#[must_use]
pub fn in_degree_sums(conn: &Connectome) -> [usize; N_REGIONS] {
    let mut inc = [0usize; N_REGIONS];
    for e in &conn.edges {
        inc[conn.region_of(e.post as usize).idx()] += 1;
    }
    inc
}

/// `"r:out:in|..."` in region index order hashed with SHA-256 — the one-line
/// census fingerprint a consumer can pin without reading the edge list.
#[must_use]
pub fn degree_sum_hash(conn: &Connectome) -> String {
    let o = out_degree_sums(conn);
    let i = in_degree_sums(conn);
    let mut s = String::new();
    for r in 0..N_REGIONS {
        if r > 0 {
            s.push('|');
        }
        s.push_str(&format!("{r}:{}:{}", o[r], i[r]));
    }
    hex32(&sha256(s.as_bytes()))
}

/// Multi-source BFS from every neuron of `src`; distances by gid (-1 =
/// unreachable). Mirrors the Python `deque` FIFO walk; levels are order-
/// independent by definition of BFS, so counts and distances are frozen.
#[must_use]
pub fn bfs_from(conn: &Connectome, src: Region) -> Vec<i32> {
    let mut d = vec![-1i32; conn.total];
    let mut dq = VecDeque::new();
    for g in conn.offset(src)..conn.offset(src) + conn.size(src) {
        d[g] = 0;
        dq.push_back(g);
    }
    while let Some(g) = dq.pop_front() {
        for e in &conn.adj[g] {
            let q = e.post as usize;
            if d[q] < 0 {
                d[q] = d[g] + 1;
                dq.push_back(q);
            }
        }
    }
    d
}

/// `(reached, min, mean_x10, max)` geodesics inside one region of a distance
/// field, over reachable members only.
#[must_use]
pub fn pool_geodesics(dist: &[i32], conn: &Connectome, region: Region) -> (usize, i32, i64, i32) {
    let ds: Vec<i32> = (conn.offset(region)..conn.offset(region) + conn.size(region))
        .map(|g| dist[g])
        .filter(|&x| x >= 0)
        .collect();
    if ds.is_empty() {
        return (0, -1, -1, -1);
    }
    let n = ds.len() as i64;
    let sum: i64 = ds.iter().map(|&x| i64::from(x)).sum();
    (
        ds.len(),
        ds.iter().copied().min().unwrap_or(-1),
        sum * 10 / n,
        ds.iter().copied().max().unwrap_or(-1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectome::{generate, DEFAULT_SEED};

    #[test]
    fn census_is_frozen() {
        let c = generate(1, 16, DEFAULT_SEED);
        assert_eq!(
            degree_sum_hash(&c),
            "dd38f2c532e4dfbb1d888606451768d1bb3c42067aa156bcbcaafa7a3b0fb60f"
        );
        let d = bfs_from(&c, Region::LamL);
        assert_eq!(d.iter().filter(|&&x| x >= 0).count(), 413);
        assert_eq!(pool_geodesics(&d, &c, Region::DnL), (6, 4, 40, 4));
        assert_eq!(pool_geodesics(&d, &c, Region::DnR), (6, 4, 41, 5));
        assert_eq!(pool_geodesics(&d, &c, Region::Mdn), (4, 4, 40, 4));
    }
}
