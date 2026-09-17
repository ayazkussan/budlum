//! The Fly Court, end to end: council -> replay -> tournament -> envelope.
//! Prints one human-readable session over the frozen dynamics; CI compiles
//! and (via the workflow step) executes it, so the demo can never rot.

use budfly::connectome::{generate, DEFAULT_SEED};
use budfly::divan::council;
use budfly::envelope::{cheap_consistent, cheap_replay_consistent, decode, decode2, encode_council, encode_council_card};
use budfly::oracle::{sentinel_stimulus, SENTINEL_TICKS};
use budfly::replay::{mdn_early_stim, replay_cert};
use budfly::tournament::tournament;

const CHALLENGER_SEED: u64 = 0xB0DF18;

fn main() {
    let fly = generate(1, 16, DEFAULT_SEED);

    println!("=== FLY COURT SESSION (frozen dynamics, 1/16 scale) ===");
    println!(
        "connectome: {} neurons, {} synapses",
        fly.total,
        fly.edges.len()
    );

    let digest = [0x42u8; 32];
    let rep = council(&fly, &digest);
    println!("\n[council] digest = 0x42..32");
    for (s, (v, a)) in rep.verdicts.iter().zip(rep.anchors.iter()).enumerate() {
        println!("  seat {s}: {:?}  anchor {:02x?}...", v, &a[..4]);
    }
    println!(
        "  unanimous={} council={:?} (fails closed on any split)",
        rep.unanimous, rep.council
    );

    let env = encode_council(&digest, &rep);
    let checked = decode(&env).map(|e| cheap_consistent(&e)) == Some(true);
    println!(
        "\n[envelope] BSE-1: {} bytes, cheap_consistent={checked}",
        env.len()
    );
    println!(
        "  hex: {}",
        env.iter().map(|b| format!("{b:02x}")).collect::<String>()
    );

    let base = sentinel_stimulus(&fly, &digest);
    let branch = mdn_early_stim(&fly, &base, 10);
    let cert = replay_cert(&fly, &base, &branch, 10, SENTINEL_TICKS);
    println!("\n[replay] what if MDN fired at tick 10?");
    println!(
        "  prefix_bound={} branch_counters={:?} verdict={:?} (honest negative: verdict stands)",
        cert.prefix_bound, cert.branch_counters, cert.branch_verdict
    );

    let env2 = encode_council_card(&digest, &rep, &cert);
    let ok2 = decode2(&env2).map(|e| cheap_replay_consistent(&e, &cert.base_head_at_fork)) == Some(true);
    println!("\n[envelope2] BSE-2 verdict + replay card: {} bytes, cheap_replay_consistent={ok2}", env2.len());

    let challenger = generate(1, 16, CHALLENGER_SEED);
    let m = tournament(&fly, &challenger);
    println!("\n[tournament] decisiveness, 8-digest panel:");
    println!(
        "  reference abstains={} challenger abstains={} survivor={:?}",
        m.abstains_a, m.abstains_b, m.survivor
    );

    println!("\nsession anchored: every number above is pinned in goldens.anchor.toml");
}
