//! `cargo run --example fly_chip_report`
//!
//! Prints the sizing report: the stylized CNS placed on the BudFly-N1 fabric
//! sketch, then the analytic answer at the published MaleCNS scale, then a
//! live sentinel run. No arguments, no I/O, deterministic.

use budfly::connectome::{generate, Region, DEFAULT_SEED};
use budfly::fabric::{place, real_scale_report, Chip};
use budfly::oracle::sentinel_verdict;
use budfly::sha256::{hex32, sha256};

fn main() {
    let chip = Chip::n1();
    println!("== BudFly chip report ==");
    println!();

    println!("-- BudFly-N1 sketch --");
    println!("  neurons/core cap      : {}", chip.neuron_cap);
    println!(
        "  synaptic SRAM/core    : {} KiB",
        chip.syn_sram_bytes / 1024
    );
    println!("  SOPs/core/cycle       : {}", chip.sop_per_cycle);
    println!();

    for (num, den) in [(1u32, 16u32), (1, 4), (1, 1)] {
        let c = generate(num, den, DEFAULT_SEED);
        let p = place(&c, &chip);
        println!("-- stylized CNS @ {num}/{den} --");
        println!("  neurons               : {}", c.total);
        println!("  edges                 : {}", c.edges.len());
        println!(
            "  cores                 : {} ({}x{} mesh)",
            p.cores, p.mesh_x, p.mesh_y
        );
        println!(
            "  syn/core max          : {} (cap {})",
            p.syn_per_core_max,
            chip.syn_cap()
        );
        println!("  SRAM overflow cores   : {}", p.sram_overflow_cores);
        println!(
            "  avg XY hops/edge      : {:.2}",
            p.avg_hops_e2_x100 as f64 / 100.0
        );
        println!(
            "  dest cores/spike      : {:.1}",
            p.avg_dest_cores_per_spike_x10 as f64 / 10.0
        );
        println!();
    }

    let r = real_scale_report(&chip);
    println!("-- analytic: published MaleCNS scale (166,700 neurons / 25,582,938 connections) --");
    println!("  cores by neuron cap   : {}", r.cores_by_neurons);
    println!(
        "  cores by synapse SRAM : {}  <- binding",
        r.cores_by_synapses
    );
    println!(
        "  mesh                  : {}x{} ({} slots)",
        r.mesh_x,
        r.mesh_y,
        r.mesh_x * r.mesh_y
    );
    println!(
        "  activity model        : {} spiking/tick ({}%), fan-out {}",
        r.active_per_tick, 2, r.avg_fan_out
    );
    println!("  SOPs/tick             : {}", r.sops_per_tick);
    println!(
        "  fabric cycles/tick    : {}  (@64 SOP/core/cycle)",
        r.cycles_per_tick
    );
    println!(
        "  ticks/second @1GHz    : {}   (fly biology runs at ~10-1000 equivalent ticks/s)",
        r.ticks_per_second
    );
    println!(
        "  energy/tick           : {:.2} uJ   (~{:.2} W at that tick rate, order-of-magnitude)",
        r.energy_per_tick_pj as f64 / 1.0e6,
        (r.energy_per_tick_pj as f64 * r.ticks_per_second as f64) / 1.0e12
    );
    println!();

    let c = generate(1, 16, DEFAULT_SEED);
    let digest = sha256(b"budlum-genesis");
    let s = sentinel_verdict(&c, &digest);
    println!("-- sentinel demo: sha256(\"budlum-genesis\") --");
    println!("  stim neurons          : {}", s.stim_neurons);
    println!(
        "  DN_L / DN_R / MDN     : {} / {} / {}",
        s.dn_l, s.dn_r, s.mdn
    );
    println!("  verdict               : {:?}", s.verdict);
    println!("  anchor                : {}", hex32(&s.anchor));
    println!();
    let eb = Region::CxEb;
    println!("note: the verdict is an exploratory readout; the anchor is the settlement-");
    println!(
        "      grade artifact. EB ring size is {} at this scale.",
        c.size(eb)
    );
}
