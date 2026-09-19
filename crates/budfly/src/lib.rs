//! # BudFly
//!
//! A connectome-native, provably-executed neuromorphic substrate for Budlum:
//! the fruit fly's brain map, looked at from the hardware angle.
//!
//! Three claims, all checkable in `cargo test`:
//!
//! 1. **The map runs.** [`connectome`] generates a MaleCNS-*styled* CNS —
//!    visual pathway, compass ring attractor, mushroom body, descending
//!    action-selection — that actually exhibits the emergent dynamics those
//!    circuits are famous for (the ring-attractor bump test verifies it).
//! 2. **Hardware meets it.** [`fabric`] places the graph on a many-core
//!    neuromorphic mesh model (AER spikes, XY routing, energy accounting)
//!    and sizes real silicon for the published 166,700-neuron / 25.5M-edge
//!    MaleCNS scale.
//! 3. **Execution is provable.** [`sim::run`] is integer-only and produces a
//!    hash-chained anchor per tick; [`air`] is the executable specification
//!    of the transition constraints a BudZero STARK would arithmetize;
//!    [`oracle`] ties it to settlement: digest in, anchored transcript out.
//!
//! Zero dependencies, `forbid(unsafe_code)`, no `unwrap`/`expect` on
//! production paths — same rules as the rest of this repository's gates.
//!
//! Bit-exactness with the Python reference (`scripts/reference_check.py`) is
//! enforced through the golden anchors in `tests/goldens.rs`.

#![forbid(unsafe_code)]

pub mod air;
pub mod analysis;
pub mod attest;
pub mod budtime;
pub mod canary;
pub mod compass;
pub mod connectome;
pub mod dispute;
pub mod divan;
pub mod envelope;
pub mod erasure;
pub mod fabric;
pub mod hardening;
pub mod league;
pub mod learn;
pub mod lesion;
pub mod oracle;
pub mod reflex;
pub mod replay;
pub mod rng;
pub mod sha256;
pub mod sim;
pub mod tape;
pub mod tape2;
pub mod tournament;
pub mod zk;

pub use connectome::{generate, Connectome, Region, DEFAULT_SEED, N_REGIONS};
pub use fabric::{place, real_scale_report, Chip, Placement, RealScaleReport};
pub use oracle::{sentinel_verdict, SentinelReport, Verdict};
pub use sim::{run, RunResult, Stimuli, TraceRow};
