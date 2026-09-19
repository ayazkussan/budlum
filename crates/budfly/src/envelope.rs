//! BSE-1 — the BudFly Settlement Envelope: the 136-byte court verdict.
//!
//! This is the ONE artifact a settlement layer stores instead of re-running
//! anything. Everything the council commits to fits in a fixed-size record:
//!
//! ```text
//!   "BSE1"         4 B   magic
//!   fact_digest   32 B   the challenged fact
//!   seat anchors  96 B   3 x 32 B per-tick chain finals
//!   seat verdicts  3 B   0 = Abstain, 1 = Affirm, 2 = Reject
//!   council        1 B   settled verdict (rule: unanimity else Abstain)
//! ```
//!
//! CHEAP vs EXPENSIVE honesty: bytes cannot be re-derived without running
//! the fly — the envelope is a COMMITMENT. What any consumer CAN check for
//! free is [`cheap_consistent`]: magic, length, and the council rule
//! re-applied to the seat codes. A malformed or self-contradicting verdict
//! is rejected without a single neuron firing. Expensive checks (anchor
//! re-execution, bisection) remain one dispute away, as designed.
//!
//! BSE-2 appends a 68-byte replay card: `fork_tick` (LE u32), the base head
//! at `fork-1`, and the branch final. The cheap replay check is then one
//! 32-byte compare against the *published* base head — no execution.
//!
//! Bit-exact mirror of `scripts/expansion_check.py` [envelope].

use crate::divan::{CouncilReport, SEATS};
use crate::oracle::Verdict;

/// Envelope size in bytes (frozen).
pub const ENVELOPE_LEN: usize = 136;
/// Magic prefix (frozen).
pub const MAGIC: [u8; 4] = *b"BSE1";

/// Wire code for a verdict byte (fails closed: Abstain is zero).
#[must_use]
pub const fn verdict_code(v: Verdict) -> u8 {
    match v {
        Verdict::Abstain => 0,
        Verdict::Affirm => 1,
        Verdict::Reject => 2,
    }
}

/// Decodes a verdict byte; anything unknown maps to `Abstain` (fails closed).
#[must_use]
pub const fn code_verdict(b: u8) -> Verdict {
    match b {
        1 => Verdict::Affirm,
        2 => Verdict::Reject,
        _ => Verdict::Abstain,
    }
}

/// A decoded BSE-1 record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Envelope {
    /// The challenged fact digest.
    pub digest: [u8; 32],
    /// Per-seat chain finals.
    pub seat_anchors: [[u8; 32]; SEATS],
    /// Per-seat verdicts.
    pub seat_verdicts: [Verdict; SEATS],
    /// Settled council verdict.
    pub council: Verdict,
}

/// Encodes a council report to the frozen 136-byte wire form.
#[must_use]
pub fn encode_council(digest: &[u8; 32], rep: &CouncilReport) -> [u8; ENVELOPE_LEN] {
    let mut out = [0u8; ENVELOPE_LEN];
    out[..4].copy_from_slice(&MAGIC);
    out[4..36].copy_from_slice(digest);
    for (i, a) in rep.anchors.iter().enumerate() {
        let s = 36 + i * 32;
        out[s..s + 32].copy_from_slice(a);
    }
    for (i, v) in rep.verdicts.iter().enumerate() {
        out[132 + i] = verdict_code(*v);
    }
    out[135] = verdict_code(rep.council);
    out
}

/// Decodes a byte slice into an envelope, or `None` if malformed.
#[must_use]
pub fn decode(bytes: &[u8]) -> Option<Envelope> {
    if bytes.len() != ENVELOPE_LEN || bytes[..4] != MAGIC {
        return None;
    }
    let mut digest = [0u8; 32];
    digest.copy_from_slice(&bytes[4..36]);
    let mut seat_anchors = [[0u8; 32]; SEATS];
    for (i, a) in seat_anchors.iter_mut().enumerate() {
        let s = 36 + i * 32;
        a.copy_from_slice(&bytes[s..s + 32]);
    }
    let seat_verdicts = [
        code_verdict(bytes[132]),
        code_verdict(bytes[133]),
        code_verdict(bytes[134]),
    ];
    Some(Envelope {
        digest,
        seat_anchors,
        seat_verdicts,
        council: code_verdict(bytes[135]),
    })
}

/// The free check: the council byte must be what the unanimity rule yields
/// over the seat verdicts. No fly execution, no hashing — a forged or
/// corrupted record that contradicts its own seats dies here.
#[must_use]
pub fn cheap_consistent(env: &Envelope) -> bool {
    let unanimous = env.seat_verdicts[0] == env.seat_verdicts[1]
        && env.seat_verdicts[1] == env.seat_verdicts[2];
    env.council
        == (if unanimous {
            env.seat_verdicts[0]
        } else {
            Verdict::Abstain
        })
}

/// BSE-2 size in bytes (frozen).
pub const ENVELOPE2_LEN: usize = 204;
/// BSE-2 magic prefix (frozen).
pub const MAGIC2: [u8; 4] = *b"BSE2";

/// The replay card appended by BSE-2: the fork point and both chain ends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Envelope2 {
    /// The verdict part (same fields as BSE-1).
    pub base: Envelope,
    /// Fork tick of the counterfactual (0 = no card).
    pub fork: u32,
    /// Base head at `fork - 1` (zero bytes when `fork` is 0).
    pub base_head_at_fork: [u8; 32],
    /// Branch chain final (zero bytes when `fork` is 0).
    pub branch_final: [u8; 32],
}

/// Encodes a council report plus a replay certificate into BSE-2 form.
#[must_use]
pub fn encode_council_card(
    digest: &[u8; 32],
    rep: &CouncilReport,
    cert: &crate::replay::ReplayCert,
) -> [u8; ENVELOPE2_LEN] {
    let mut out = [0u8; ENVELOPE2_LEN];
    out[..4].copy_from_slice(&MAGIC2);
    out[4..36].copy_from_slice(digest);
    for (i, a) in rep.anchors.iter().enumerate() {
        let s = 36 + i * 32;
        out[s..s + 32].copy_from_slice(a);
    }
    for (i, v) in rep.verdicts.iter().enumerate() {
        out[132 + i] = verdict_code(*v);
    }
    out[135] = verdict_code(rep.council);
    out[136..140].copy_from_slice(&cert.fork.to_le_bytes());
    out[140..172].copy_from_slice(&cert.base_head_at_fork);
    out[172..204].copy_from_slice(&cert.branch_final);
    out
}

/// Decodes BSE-2 bytes, or `None` if malformed.
#[must_use]
pub fn decode2(bytes: &[u8]) -> Option<Envelope2> {
    if bytes.len() != ENVELOPE2_LEN || bytes[..4] != MAGIC2 {
        return None;
    }
    let mut with_bse1_head = [0u8; ENVELOPE_LEN];
    with_bse1_head[..4].copy_from_slice(&MAGIC);
    with_bse1_head[4..].copy_from_slice(&bytes[4..ENVELOPE_LEN]);
    let base = decode(&with_bse1_head)?;
    let fork = u32::from_le_bytes([bytes[136], bytes[137], bytes[138], bytes[139]]);
    let mut base_head_at_fork = [0u8; 32];
    base_head_at_fork.copy_from_slice(&bytes[140..172]);
    let mut branch_final = [0u8; 32];
    branch_final.copy_from_slice(&bytes[172..204]);
    if fork > 0 && (base_head_at_fork == [0u8; 32] || branch_final == [0u8; 32]) {
        return None; // a fork without a card is not BSE-2
    }
    Some(Envelope2 {
        base,
        fork,
        base_head_at_fork,
        branch_final,
    })
}

/// The BSE-2 cheap check: verdict self-consistency (as BSE-1) plus, when a
/// fork card is present, the committed base head must equal the published
/// head the verifier trusts (from a chain, a manifest, or the BSE-1 lineage
/// of the same fact). One 32-byte compare, zero execution.
#[must_use]
pub fn cheap_replay_consistent(env: &Envelope2, published_head: &[u8; 32]) -> bool {
    if !cheap_consistent(&env.base) {
        return false;
    }
    if env.fork == 0 {
        return true; // no card: nothing more to bind
    }
    env.base_head_at_fork == *published_head
}

/// BSE-3 magic prefix (frozen).
pub const MAGIC3: [u8; 4] = *b"BSE3";
/// One counterfactual card on the BSE-3 wire (frozen: 68 bytes).
pub const CARD_LEN: usize = 68;
/// Fixed part of a BSE-3 envelope before the cards (frozen: 137 bytes).
pub const ENVELOPE3_HEADER: usize = 137;

/// A counterfactual card: fork tick, base head at `fork - 1`, branch final.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayCard {
    /// Fork tick of the counterfactual (must be > 0).
    pub fork: u32,
    /// Base head at `fork - 1` (checked against the published base chain).
    pub base_head_at_fork: [u8; 32],
    /// Branch chain final (the counterfactual's sealed outcome).
    pub branch_final: [u8; 32],
}

/// A decoded BSE-3 record: the verdict plus a STACK of cards.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Envelope3 {
    /// The verdict part (same fields as BSE-1).
    pub base: Envelope,
    /// Counterfactual cards, in committed order.
    pub cards: Vec<ReplayCard>,
}

impl From<&crate::replay::ReplayCert> for ReplayCard {
    fn from(cert: &crate::replay::ReplayCert) -> Self {
        ReplayCard {
            fork: cert.fork,
            base_head_at_fork: cert.base_head_at_fork,
            branch_final: cert.branch_final,
        }
    }
}

/// Encodes a council report plus a stack of replay cards into BSE-3 form.
#[must_use]
pub fn encode_council_cards(
    digest: &[u8; 32],
    rep: &CouncilReport,
    cards: &[ReplayCard],
) -> Vec<u8> {
    let mut out = Vec::with_capacity(ENVELOPE3_HEADER + cards.len() * CARD_LEN);
    out.extend_from_slice(&MAGIC3);
    out.extend_from_slice(digest);
    for a in rep.anchors.iter() {
        out.extend_from_slice(a);
    }
    for v in rep.verdicts.iter() {
        out.push(verdict_code(*v));
    }
    out.push(verdict_code(rep.council));
    out.push(cards.len() as u8);
    for c in cards {
        out.extend_from_slice(&c.fork.to_le_bytes());
        out.extend_from_slice(&c.base_head_at_fork);
        out.extend_from_slice(&c.branch_final);
    }
    out
}

/// Decodes BSE-3 bytes, or `None` if malformed (length/count mismatch, bad
/// magic, or a fork card with zeroed heads).
#[must_use]
pub fn decode3(bytes: &[u8]) -> Option<Envelope3> {
    if bytes.len() < ENVELOPE3_HEADER || bytes[..4] != MAGIC3 {
        return None;
    }
    let count = bytes[136] as usize;
    if bytes.len() != ENVELOPE3_HEADER + count * CARD_LEN {
        return None;
    }
    let mut with_bse1_head = [0u8; ENVELOPE_LEN];
    with_bse1_head[..4].copy_from_slice(&MAGIC);
    with_bse1_head[4..].copy_from_slice(&bytes[4..ENVELOPE_LEN]);
    let base = decode(&with_bse1_head)?;
    let mut cards = Vec::with_capacity(count);
    for c in 0..count {
        let s = ENVELOPE3_HEADER + c * CARD_LEN;
        let fork = u32::from_le_bytes([bytes[s], bytes[s + 1], bytes[s + 2], bytes[s + 3]]);
        let mut base_head_at_fork = [0u8; 32];
        base_head_at_fork.copy_from_slice(&bytes[s + 4..s + 36]);
        let mut branch_final = [0u8; 32];
        branch_final.copy_from_slice(&bytes[s + 36..s + 68]);
        if fork == 0 || base_head_at_fork == [0u8; 32] || branch_final == [0u8; 32] {
            return None; // a card slot that does not commit is not BSE-3
        }
        cards.push(ReplayCard {
            fork,
            base_head_at_fork,
            branch_final,
        });
    }
    Some(Envelope3 { base, cards })
}

/// The BSE-3 cheap check: verdict self-consistency (as BSE-1) plus, for
/// EVERY card, one 32-byte compare against the published base chain at
/// `fork - 1`. No execution, no hashing.
#[must_use]
pub fn cheap_replay_consistent3(env: &Envelope3, published_base: &[[u8; 32]]) -> bool {
    if !cheap_consistent(&env.base) {
        return false;
    }
    env.cards.iter().all(|c| {
        c.fork > 0
            && (c.fork as usize) <= published_base.len()
            && c.base_head_at_fork == published_base[(c.fork - 1) as usize]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectome::{generate, DEFAULT_SEED};
    use crate::divan::council;

    const FROZEN_HEX: &str = "425345314242424242424242424242424242424242424242424242424242424242424242ae3aa8d0afc6a7db9a9bb222b8c6917aefad1dc32dea271da827be696112e5ed6b14cece457976715767da61b2cb48ef8ff27cb06b2fb9840887aa2c67a1d1fb5400ce9c2b1f00a8159c178b618fc363f3ccddcb9bab3f22ce2c50f6d40612ec00010000";

    #[test]
    fn bse1_encode_is_a_frozen_byte_stream() {
        let c = generate(1, 16, DEFAULT_SEED);
        let rep = council(&c, &[0x42u8; 32]);
        let env = encode_council(&[0x42u8; 32], &rep);
        assert_eq!(env.len(), ENVELOPE_LEN);
        let full: String = env.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(full, FROZEN_HEX);
    }

    #[test]
    fn decode_roundtrips_and_cheap_check_holds() {
        let c = generate(1, 16, DEFAULT_SEED);
        let rep = council(&c, &[0x42u8; 32]);
        let bytes = encode_council(&[0x42u8; 32], &rep);
        let env = decode(&bytes).expect("frozen bytes must decode");
        assert!(cheap_consistent(&env));
        assert_eq!(env.council, Verdict::Abstain);
        assert_eq!(env.digest, [0x42u8; 32]);
        // corruption that breaks self-consistency is rejected for free:
        let mut forged = env.clone();
        forged.council = Verdict::Affirm;
        assert!(!cheap_consistent(&forged));
        assert!(decode(&bytes[..100]).is_none());
    }

    const FROZEN2_HEX: &str = "42534532ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffb8cd253a0904fc4c472dd5aa7342dcd1fcde767b5cdc51d7f5aa3f16fc4e53b2135aa1433e5c9db08cdd427258847bacc3047c4ed2945343d5abc4f7b0dc299c02a6d3844703acb43f41fd3075d625bed6ef9c556d50fa56517e5e047d8bda51000101000a000000090600fb1189fe4e74805461610ba01b18a9f4db6b125c6b0f174a9df613d69a62cfbd0f6769cf555005034d4ee34d1229d32a9b0fe043c69df17a2f3df96148";

    #[test]
    fn bse2_verdict_plus_card_is_a_frozen_byte_stream() {
        use crate::oracle::{sentinel_stimulus, SENTINEL_TICKS};
        use crate::replay::{mdn_early_stim, replay_cert};

        let c = generate(1, 16, DEFAULT_SEED);
        let digest = [0xffu8; 32];
        let rep = council(&c, &digest);
        let base = sentinel_stimulus(&c, &digest);
        let cert = replay_cert(
            &c,
            &base,
            &mdn_early_stim(&c, &base, 10),
            10,
            SENTINEL_TICKS,
        );
        let env = encode_council_card(&digest, &rep, &cert);
        assert_eq!(env.len(), ENVELOPE2_LEN);
        let full: String = env.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(full, FROZEN2_HEX);

        let dec = decode2(&env).expect("frozen bytes must decode");
        assert_eq!(dec.fork, 10);
        assert_eq!(dec.base.council, Verdict::Abstain);
        assert_eq!(dec.base.seat_verdicts[1], Verdict::Affirm);
        let published = [
            0x09, 0x06, 0x00, 0xfb, 0x11, 0x89, 0xfe, 0x4e, 0x74, 0x80, 0x54, 0x61, 0x61, 0x0b,
            0xa0, 0x1b, 0x18, 0xa9, 0xf4, 0xdb, 0x6b, 0x12, 0x5c, 0x6b, 0x0f, 0x17, 0x4a, 0x9d,
            0xf6, 0x13, 0xd6, 0x9a,
        ];
        assert!(cheap_replay_consistent(&dec, &published));
        // a forged published head kills the card binding, for free:
        let mut wrong = published;
        wrong[31] ^= 1;
        assert!(!cheap_replay_consistent(&dec, &wrong));
        // BSE-1 semantics survive: fork == 0 means zero card bytes:
        let mut nocards = cert.clone();
        nocards.fork = 0;
        nocards.base_head_at_fork = [0u8; 32];
        nocards.branch_final = [0u8; 32];
        let env0 = encode_council_card(&digest, &rep, &nocards);
        assert!(env0[136..].iter().all(|b| *b == 0));
        assert!(decode2(&env[..100]).is_none());
    }

    const FROZEN3_HEX: &str = "42534533ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffb8cd253a0904fc4c472dd5aa7342dcd1fcde767b5cdc51d7f5aa3f16fc4e53b2135aa1433e5c9db08cdd427258847bacc3047c4ed2945343d5abc4f7b0dc299c02a6d3844703acb43f41fd3075d625bed6ef9c556d50fa56517e5e047d8bda5100010100020a000000090600fb1189fe4e74805461610ba01b18a9f4db6b125c6b0f174a9df613d69a62cfbd0f6769cf555005034d4ee34d1229d32a9b0fe043c69df17a2f3df961480400000098f5e3a7de07f4b37b5474c601ac8ec6dd619baf278d7a61c0c3a5883f3025e0e0216bec6461444448ea7ae8647f18c0e7da8c0071dc4f08f46be1bcf5dc599b";

    #[test]
    fn bse3_stacks_two_cards_and_verifies_each_for_free() {
        use crate::oracle::{sentinel_stimulus, SENTINEL_TICKS};
        use crate::replay::{mdn_early_stim, replay_cert};
        use crate::sim::run_log;

        let c = generate(1, 16, DEFAULT_SEED);
        let digest = [0xffu8; 32];
        let rep = council(&c, &digest);
        let base = sentinel_stimulus(&c, &digest);
        let cert1 = replay_cert(
            &c,
            &base,
            &mdn_early_stim(&c, &base, 10),
            10,
            SENTINEL_TICKS,
        );
        // card 2: "what if the bump had been cut at tick 4?" — every (0, 8)
        // stimulus window shortened to (0, 4), the frozen edit rule.
        let mut cf = base.clone();
        for (g, w) in cf.windows() {
            if w == (0, 8) {
                cf.set(g, 0, 4);
            }
        }
        let cert2 = replay_cert(&c, &base, &cf, 4, SENTINEL_TICKS);
        let cards = [ReplayCard::from(&cert1), ReplayCard::from(&cert2)];
        let env = encode_council_cards(&digest, &rep, &cards);
        assert_eq!(env.len(), 273);
        let full: String = env.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(full, FROZEN3_HEX);

        let dec = decode3(&env).expect("frozen bytes must decode");
        assert_eq!(dec.cards.len(), 2);
        assert_eq!(dec.cards[0].fork, 10);
        assert_eq!(dec.cards[1].fork, 4);
        let published = run_log(&c, SENTINEL_TICKS, &base);
        assert!(cheap_replay_consistent3(&dec, &published));
        // a wrong base chain kills the binding of every card, for free:
        let mut wrong = published.clone();
        wrong[3][31] ^= 1;
        assert!(!cheap_replay_consistent3(&dec, &wrong));
        assert!(
            decode3(&env[..200]).is_none(),
            "a truncated stack is not BSE-3"
        );
    }
}
