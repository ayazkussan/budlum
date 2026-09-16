//! Golden anchors: the Rust implementation must reproduce, bit for bit, the
//! anchors produced by the Python reference (`scripts/reference_check.py`).
//!
//! These numbers are the cross-language determinism contract:
//! same generator v1.0 -> same edges -> same spikes -> same SHA-256 chain.
//! If any of these fail, *the implementation* moved, not the data.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use budfly::connectome::{generate, Region, DEFAULT_SEED};
use budfly::oracle::{sentinel_verdict, Verdict};
use budfly::sha256::{hex32, sha256};
use budfly::sim::{run, Stimuli};

fn unhex(s: &str) -> [u8; 32] {
    let b = s.as_bytes();
    assert_eq!(b.len(), 64);
    let mut out = [0u8; 32];
    let nib = |c: u8| -> u8 {
        match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            _ => panic!("bad hex"),
        }
    };
    for i in 0..32 {
        out[i] = (nib(b[2 * i]) << 4) | nib(b[2 * i + 1]);
    }
    out
}

/// Compass-bump scenario: stimulate a quarter of the EB ring for ticks 0..8,
/// run 64 ticks, pin the anchor head.
#[test]
fn ring_bump_anchor_is_golden() {
    let c = generate(1, 16, DEFAULT_SEED);
    let off = c.offset(Region::CxEb);
    let n = c.size(Region::CxEb);
    let mut stim = Stimuli::new();
    for i in 0..(n / 4) {
        stim.set((off + i) as u32, 0, 8);
    }
    let r = run(&c, 64, &stim, false);
    assert_eq!(
        hex32(&r.anchor),
        "aedfd9426fd3d9f0e79afa1a889cb7bfb88e1e7a8cc0c33b3561718ea127aea3"
    );
    // The bump must actually exist: EB fired during the run.
    assert!(r.region_spikes[Region::CxEb.idx()] > 0);
}

struct Golden {
    digest: [u8; 32],
    verdict: Verdict,
    dn_l: u64,
    dn_r: u64,
    mdn: u64,
    anchor: &'static str,
}

#[test]
fn sentinel_goldens() {
    let c = generate(1, 16, DEFAULT_SEED);
    let genesis = sha256(b"budlum-genesis");
    assert_eq!(
        hex32(&genesis),
        "d9cbd5cb19d40295d4f194d458ddc7ebf6295544e30d71e377774f33c1bff209"
    );
    let mut inc = [0u8; 32];
    for (i, b) in inc.iter_mut().enumerate() {
        *b = i as u8;
    }
    let cases = [
        Golden {
            digest: [0u8; 32],
            verdict: Verdict::Abstain,
            dn_l: 0,
            dn_r: 0,
            mdn: 0,
            anchor: "50f5f2ab0f7342fb412c7b781ea3b9dee5f45160a8a5a22ace3283b784d18173",
        },
        Golden {
            digest: [0xffu8; 32],
            verdict: Verdict::Affirm,
            dn_l: 7,
            dn_r: 6,
            mdn: 1,
            anchor: "398e126a88776b4f15f7da9bfb7bd927f47fc32e8a92128a354380ce8d429cc3",
        },
        Golden {
            digest: genesis,
            verdict: Verdict::Affirm,
            dn_l: 5,
            dn_r: 3,
            mdn: 6,
            anchor: "ac4e8042233658c31ac154f7ee7fd7ee0707959373e057f60743e41b68ae0eb4",
        },
        Golden {
            digest: inc,
            verdict: Verdict::Reject,
            dn_l: 1,
            dn_r: 3,
            mdn: 0,
            anchor: "01b3ff076fa7850317ff1883e18b58f1e17d77dd2efaa12eb24b8db943699cb2",
        },
    ];
    for (i, g) in cases.iter().enumerate() {
        let r = sentinel_verdict(&c, &g.digest);
        assert_eq!(r.verdict, g.verdict, "case {i} verdict");
        assert_eq!(r.dn_l, g.dn_l, "case {i} dn_l");
        assert_eq!(r.dn_r, g.dn_r, "case {i} dn_r");
        assert_eq!(r.mdn, g.mdn, "case {i} mdn");
        assert_eq!(
            unhex(g.anchor),
            r.anchor,
            "case {i} anchor: reference mismatch (got {})",
            hex32(&r.anchor)
        );
    }
}
