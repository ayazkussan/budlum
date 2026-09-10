# Dead-in-production public API inventory (2026-09-10)

Measured from the fork lineage `8ebf838`, statically. Method, so the numbers can
be re-derived and disputed:

* every `pub fn` in `src/**` that is **not** inside a `#[cfg(test)]` module;
* a name counts as referenced if it appears anywhere outside a `#[cfg(test)]`
  module in `src/ crates/ examples/ benches/ budzero/ xtask/ ops/ .github/
  config/ proto/` (`.rs .toml .yml .md .sh .py .json`), minus its own
  declaration lines;
* a name is also treated as live if the exact string appears as a quoted
  literal anywhere (RPC/CLI dispatch tables), which is how `--self-test` style
  gate names stay visible.

Result: **205 of 1438** `src/` public functions have no reference outside
tests. 14 of them say so in their own doc comment within 14 lines above
the definition ("Convenience: ...", "exposed for the CLI", `WIRING:`); **191
are silent** - no hint, no doc, nothing calls them. 28 of the silent
ones carry a security or consensus verb in the name.

## The silent + security-named list (28)

| fn | where |
|---|---|
| `active_validator_count` | `src/consensus/poa.rs:247` |
| `audit_events` | `src/registry/poa_compliance.rs:242` |
| `bls_signing_available` | `src/crypto/pkcs11.rs:402` |
| `broadcast_slashing_evidence_sync` | `src/network/node.rs:142` |
| `consensus_invalid_relay_front_running` | `src/registry/evidence.rs:351` |
| `consensus_invalid_relay_wrong_relay` | `src/registry/evidence.rs:372` |
| `export_audit_csv` | `src/registry/poa_compliance.rs:250` |
| `export_audit_json` | `src/registry/poa_compliance.rs:246` |
| `get_checkpoints` | `src/consensus/pos.rs:189` |
| `get_slashing_evidence` | `src/consensus/pos.rs:171` |
| `is_checkpoint_height` | `src/chain/finality.rs:197` |
| `is_consensus_validator` | `src/core/account.rs:142` |
| `is_fully_valid` | `src/ai/execution/verify.rs:43` |
| `is_valid_chain` | `src/chain/blockchain.rs:4672` |
| `is_validator` | `src/consensus/pos.rs:460` |
| `kq_wallet_tee_attestation` | `src/account_abstraction/tee_attestation.rs:233` |
| `new_zk_invalid_attestation` | `src/consensus/qc.rs:415` |
| `pq_signing_available` | `src/crypto/pkcs11.rs:408` |
| `private_key_bytes` | `src/crypto/primitives.rs:703` |
| `record_invalid` | `src/network/gossip_dedup.rs:214` |
| `record_valid` | `src/network/gossip_dedup.rs:195` |
| `reseal_after_manual_edit` | `src/chain/snapshot.rs:931` |
| `sign_attestation` | `src/consensus/qc.rs:655` |
| `sign_v6` | `src/core/transaction.rs:753` |
| `slash_all_roles` | `src/core/account.rs:159` |
| `store_bls_key` | `src/crypto/pkcs11.rs:233` |
| `store_pq_key` | `src/crypto/pkcs11.rs:246` |
| `with_validator` | `src/chain/genesis.rs:233` |

## Classification, after reading the top entries (not assumed)

* `is_valid_chain` (`src/chain/blockchain.rs:4672`) - a convenience wrapper over
  `validate_candidate_chain`, which **is** called. No enforcement is lost.
  `is_valid` (its sibling) is used. Verdict: dead convenience, low risk.
* `is_authorized_now` (`src/registry/poa_onboarding.rs:435`) - documented as
  "Convenience: ... Allocates a whitelist snapshot; for hot paths use
  `PoAWhitelist::contains` on a cached snapshot", and `PoAWhitelist::contains`
  is what the validation path uses. Verdict: alternate entry point, not a
  missing check. (My hint scan missed it only because the sentence sits 15 lines
  above the `fn`.)
* `store_bls_key` / `store_pq_key` / `bls_signing_available` /
  `pq_signing_available` (`src/crypto/pkcs11.rs`) - the vendor/HSM capability
  model is implemented and tested, but **no binary or provisioning path calls
  it**. This is the same open point the directive lists as "HSM/PKCS#11": the
  library half exists, the operator half does not. Verdict: real gap, tracked
  elsewhere, not a false alarm and not a new one.

## What the number does and does not mean

It does **not** mean 205 bugs. `pub` items in a library are allowed to be
unused inside the tree. It means: the repo has one ratchet for refusal-shaped
names (`.github/unwired-guards-baseline.txt`, currently 3, and its own doc
admits it counts by *name prefix*) and none for this class. `assign_object` - the
finding that closed in this session - was invisible to the gate exactly because
the gate looks at `check/verify/validate/require/enforce/...` and not at
`assign/select/place/build/derive/compute`.

## Proposed next step (not taken in this commit)

A `dead-pub-api` ratchet with the same shape as the guard baseline: a sorted
`path:name` list, "may only shrink", plus one exemption token (`WIRING:` or a
`/// Convenience:` line) that a reviewer can see in the diff. Written as a Rust
gate in `xtask/gates`, not a shell script (`no-new-shell-gates` pins that set),
and it must land with its 205 entries recorded first - a gate that fails on
arrival gets switched off, which the tree has already learned twice.
