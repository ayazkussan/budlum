# Denetim dongusu 2026-09-10A - B.U.D. wiring + BudZKVM gate, re-verified live

English by policy: this file lives under `docs/` in the budlum tree, which the
`tree_is_english` gate keeps in English. The Turkish version of this loop's
record is `repo-workspace/patches/0034-*.patch`, which is the workspace-repo
log entry.

## 0. Method, refs, and the honest limits of this run

Refs compared:

| name | sha | what it is |
|---|---|---|
| local snapshot | `88970e3` (tree `6e4a196`) | fork `ayazkussan/budlum` main `d07225d` + mirror commit |
| PR 50 head | `f455ba4` | `refs/pull/50/head` of `budlum-xyz/budlum`, branch `usl`, OPEN, 100 commits |
| upstream main | `acf48ef` | `Universal Settlement Layer (#49)`, 2026-09-02 |

Measured drift (tree comparison, not commit graph - the fork has no common
ancestor with upstream, it is a zip upload, see `repo-workspace/README.md`):

* local -> PR 50 head: **59 files differ, 48 exist only in the local tree**
  (all of them session infra: `repo-workspace/`, `repo-lubot/`, three extra
  workflows, `.quality/`, `ops/scripts/`) and **11 modified**, where the local
  tree carries ~5.4k lines that PR 50 head does not have.
* local -> upstream main: 422 files, upstream main is ~31k lines *behind*.
* PR 50 head -> upstream main: 387 files, +26481/-5037.

So: the fork lineage is **ahead** of the PR that CI gates. Anything "fixed"
here that is not in `f455ba4` is fixed only in the mirror. That is the standing
structural risk of this setup and it is logged, not hidden.

Limitations of this run, stated before any claim:

1. **No Rust toolchain in this sandbox** (`rustc`/`cargo` not found, no
   `target/`, no cargo registry). Therefore every statement below is **static
   evidence** (call-site reachability, not test results). By the charter's own
   rule, CI is the sole authority, so nothing here says "green".
2. The `32-base` rule referenced by section 4 of the directive is **not in the
   mirrored workspace content** (`grep` over `repo-workspace/patches/*.patch`:
   no hit). It lives in the workspace repo, which is not cloned here. Applied
   policy for this run: no `sleep`, no `timeout`-as-wait, no waiting loops;
   background work is polled by state, not by clock. If the 32-base rule says
   something more specific, it needs to be mirrored into this repo to be
   followable.

## 1. B.U.D. integration verification (per-loop re-verification, live)

| capability | state | evidence (same at all three refs unless noted) |
|---|---|---|
| coding audit (parity column check) | **wired** | `StorageRegistry::verify_coding_audit` (`src/domain/storage_deal.rs:2168`) is called from the chain actor at `src/chain/chain_actor.rs:4106`; `column_is_correctly_encoded` is reached through it (`src/domain/storage_deal.rs:2189`) |
| erasure re-encode on the read/registration path | **wired** | `storage::verify_object_encoding` (`src/storage/erasure.rs:731`) called from `src/rpc/server.rs:2293`; `encode_object` reached from inside it (`src/storage/erasure.rs:735`) |
| repair trigger | **wired - the charter's "last known state: UNWIRED" is REFUTED** | `objects_below_own_repair_margin()` is consumed by the epoch maintenance pass: `src/chain/chain_actor.rs:2728` (local) / `:2684` (PR 50 head) / `:2652` (upstream main). The code comment records the historical gap: "`objects_needing_repair` has existed since erasure coding landed and nothing called it" |
| repair *action*, not just band detection | **partial, gap is self-declared** | `src/chain/chain_actor.rs:2734` `F-16`: tickets are only opened for shards with **zero** active replicas; "A shard the registry never placed cannot be ticketed with the current type; that gap is logged." So a degraded-but-nonzero object is detected and logged, not repaired |
| shard placement | **coded, not wired for the part that matters** | see section 2 |
| `verify_answer_challenge_zk_proof` | intentionally unwired | `.github/unwired-guards-baseline.txt` = 3, with per-entry justification; callers are meant to use `answer_challenge` |

## 2. Finding BUD-P-1 (new): placement spreading runs only in tests

`storage::assignment::assign_object` (`src/storage/assignment.rs:212`) is, by
its own doc, "the location index a reader queries and a repair rebuilds from".
Its distinguishing behaviour versus calling `assign_shard` per shard is the
used-set spread, and the doc states the threat it removes:

> one high-scoring address can otherwise win many shards of the same object,
> and its departure loses all of them at once.

Callers of `assign_object`: `src/storage/assignment.rs:297, 304, 462, 472, 529`
— **all inside the `#[cfg(test)]` module**. No production call site at local
HEAD, at PR 50 head, or at upstream main.

Production reaches the placement primitive only as advice:
`StorageRegistry::annotate_expected_holders`
(`src/domain/storage_deal.rs:3066`) calls `assign_shard(..., 1)` per pending
ticket, and its own doc says the result is "a **recommendation**" written
"to make divergence visible". So:

* rendezvous hashing: on a production path, advisory only;
* per-object anti-correlation spreading: **never executes outside tests**;
* consequently the failure mode the spreading was written for (one operator
  holding a whole code word) is unmitigated on the live path, while the gate
  count in `.github/unwired-guards-baseline.txt` stays green, because the
  baseline tracks guards *named* check/verify/validate/... and `assign_object`
  is named `assign_*`.

That last sentence is the process finding: **the ratchet has a naming hole.**
A refusal named `verify_*` is tracked; a placement policy named `assign_*`
is not. Same class of hole as the `github_pat_` prefix that was missing from
the redactor until it was added - a list that enumerates by name misses what
isn't named that way.

Severity: HIGH for the security property (single-operator loss of an object =
data loss at `k` threshold with `n` shards, no fault tolerance), MEDIUM for the
consistency property (writer and rebuild both use `assign_shard`, so they agree
- they agree on the *unspreadded* answer, which is why nothing currently breaks).

Deliberate non-action: the fix is not a mechanical call-site swap. Wiring
`assign_object` changes which operator is expected to hold each shard, i.e. it
changes operator-visible duty assignment and therefore the economic/reputation
layer. Per the escalation ladder this is `stop-and-ask_user`, not
`act-and-log`. The candidate designs are in section 5.

## 3. BudZKVM: VerifyMerkle gate, re-checked live

`budzero/bud-isa/src/lib.rs`:

* `MainnetActivation::default()` = `verify_merkle_enabled: false`,
  `verify_inference_enabled: false`, privacy three `true` (line ~148).
* The charter asks, every loop: "is it disabled because an external opcode
  audit is expected, or is there a new problem?" **Answer for this ref: external
  review; not a new problem, and not a missing constraint.** The code states
  both halves:
  * the old reason ("unfinished path verification") was real - "the root
    comparison looked at the `merkle_current` cell of the original row, and no
    constraint forced the output of round 64 to be written into that cell", and
    "a prover that writes the claimed root there itself ... produced a proof
    that verified. That was measured, and it did.";
  * that gap is closed, with a named rejection test
    (`rejects_verify_merkle_root_not_produced_by_the_path`);
  * the gate stays closed because "the remaining condition is an **external
    review**, not a missing constraint ... Do not turn this flag on merely on
    the strength of this comment."
* `VerifyInference` is closed for a **different** reason and the code says so:
  "no verification circuit behind it at all and returns a hard-coded zero",
  cross-referenced to `docs/AI_VERIFICATION_STATUS.md`, which keeps the
  claimed-output binding row as `open`.

Status carried forward: `BudZKVM: KAPALI-bilincli-ve-gerekceli` (disabled,
reason known and honest). The bit-test `verify_merkle_enabled -> true` is not
an available action in any loop: it is an audit deliverable, not a code change.

## 4. Anti-pattern check the directive asks for: findings pasted as code

Checked for "the auditing AI dropped the finding text into the source instead
of writing code":

* `src/tests/f20_priority_findings.rs` (332 lines, 27 asserts): asserts call
  real functions (`first_placeholder_peer`, `check_mainnet_validator_key_policy`,
  `StorageRegistry::...`) with real subjects; the module doc says it "pins the
  remaining holes ... names the findings that remain fail-closed rather than
  pretending a missing AIR is now live" - the correct direction.
* Finding ids inside `src/**/*.rs` appear as provenance comments
  (`/// F-03 / F-04 / ...`, `// F-16:`, `/// Audit 2026-09-09, F-3:`), 14 files,
  no case where a finding sentence stands in for an implementation.
* Vacuity sweep: `todo!` / `unimplemented!` / `panic!("not implemented`) in
  `src/**.rs`: **0**. `#[ignore]`: **1** occurrence (not inspected this loop).
* No `#[cfg(test)]`-only production entry point was found among the audit
  helpers other than `assign_object` (section 2), which is a *missing call
  site*, not a pasted comment.

Verdict for this pass: **no paste-instead-of-code instance found** in B.U.D.,
the ZKVM gates, or the finding-lock tests. This is a negative result of one
targeted scan, not a tree-wide proof.

## 5. Open points, ranked, with the one question that needs a human

1. BUD-P-1 `assign_object` unwired (section 2). Options:
   (a) wire `assign_object` into `annotate_expected_holders` and the rebuild
   path - changes operator duty assignment, needs a semantics decision on the
   fallback when `|validators| < n` (the current spread silently falls back to
   the full pool, i.e. spreading is best-effort);
   (b) keep production as-is and demote `assign_object` to an explicitly
   `WIRING: unwired` module with the reason, so the audit trail stops implying
   an index that nothing builds;
   (c) wire the *comparison* only: production already writes advice, so add the
   divergence check that the `annotate_expected_holders` doc says is missing
   ("Today there is no comparison at all between who took a ticket and who the
   placement computation chose") - smallest blast radius, makes the real
   placement measurable before changing it.
2. F-16 self-declared gap: never-placed shard cannot be ticketed.
3. `tree_is_english` measurement not re-run: 130 diacritic-bearing lines in
   `src/`, 14 in `budzero/`, 9 in `crates/` by my own scan, against a 23-line
   baseline file - the gate's counting rules (two signals, allowed dirs,
   per-file counts) differ from a raw grep, so **no violation is claimed**;
   running the gate is a CI job and needs a toolchain.
4. Timing-Safe / Determinism / Diverse-Double-Compiling were **red** on the
   previous session's branch (`arena/01a089e3-budlum`, run #29, plus CI, Rust,
   Security Audit, Miri, Typos, Supply Chain Extra, Dependency Review #27).
   Root causes not yet triaged here; the fork does run Actions, so a push to
   this branch reproduces them with readable logs.

## 6. What this loop changed

No production code. One document (this file) and the workspace-repo mirror
patch `0034`, which carries the same record in the workspace log language and
the `BAGLANTILIK-LOG` row updates:

* `B.U.D. erasure coding / coding audit: BAGLILI` (SHA-backed, section 1)
* `B.U.D. repair trigger: BAGLILI` - charter's UNWIRED assumption refuted
* `B.U.D. placement spreading (assign_object): KODLANMIS AMA BAGLANMAMIS` - new
* `BudZKVM VerifyMerkle: KAPALI, gerekce = external review` (unchanged verdict,
  re-derived live)

## 7. Kanit baglantisi (CI)

* commit `d4b6b8d` -> push `arena/01a08a1a-budlum` -> PR `ayazkussan/budlum#3`.
* Fork workflows are keyed on `push: [main]` + `pull_request: [main]`, so a bare
  branch push triggers nothing; the PR is what starts CI. 19 runs queued on
  `d4b6b8d` (main CI run id `34449158073`, cargo-deny/nextest jobs pending).
* Prediction, written before the outcome: this commit touches no Rust source,
  so the jobs that were red on the previous branch's run #29 (Determinism,
  Diverse Double Compiling, Typos, Miri, Supply Chain Extra) will be red here
  too. If that holds, those failures are **pre-existing on the fork lineage**,
  not introduced by this loop - which is the whole point of running a
  doc-only commit through the pipeline. If a job flips green that was red, the
  baseline itself is flaky and that is a separate finding.
* No merge: PR #3 is left open by policy; `budlum-xyz/budlum#50` untouched.

## 8. Second half of the loop: BUD-P-1 wired (user decision `a`)

Decision recorded: wire the spread into production, not demote, not only compare.

* `StorageRegistry::annotate_expected_holders` no longer places one shard per
  ticket. It groups the pending, unannotated tickets by `manifest_id` and hands
  each group to `storage::assignment::assign_object`, so several missing shards
  of one object cannot all be advised to the same operator while another staked
  validator is free. The fallback pool rule inside `assign_object` is what makes
  this best-effort rather than a refusal: a validator set smaller than the code
  word still gets an advisory for every shard.
* `assign_object`'s contract now states the subset case explicitly (the repair
  path passes the missing shards, not the whole code word), and the module's
  `WIRING:` block was rewritten: **wiring a rule is not the same as having the
  rule run**, which is the sentence this finding deserves.
* Two tests in `src/storage/assignment.rs`:
  `a_multi_shard_subset_still_spreads` (the new property) and
  `a_single_shard_subset_places_exactly_as_the_per_shard_rule` (the regression
  lock - an object with one shard to repair must be placed exactly as before,
  so the change cannot silently move anything).
* Stale doc killed: `annotate_expected_holders` claimed "Today there is no
  comparison at all between who took a ticket and who the placement chose".
  `placements_that_diverged` exists and the maintenance pass logs it
  (`src/chain/chain_actor.rs:2917`). The comment described a world the code had
  already left behind - found while writing the new doc, not by a gate.

### Consensus blast radius, checked before writing

`expected_holder` is not a metric. `StorageRegistry::root()`
(`src/domain/storage_deal.rs:1063`) folds every ticket through bincode
(`:1103`) and `storage_root` is committed in the block header
(`src/core/block.rs:41,126`). So a changed recommendation **is** a changed state
root, and the ordering rule (BTreeMap key order = ticket-id order) is now part
of the consensus-relevant contract - it is written in the comment for exactly
that reason. No activation gate was invented: the tree is pre-genesis
(`MainnetActivation` defaults closed, mainnet bootnodes are placeholders per
F-01), so there is no history to replay into a fork. **After genesis this same
change would need an epoch-gated V1/V2 tag on the placement entropy**, and the
`BDLM_MAINTENANCE_PLACEMENT_V1` tag is where that would go.

### What this loop cannot claim

* **Not compiled.** The user chose "install a toolchain"; it is not possible
  here: `static.rust-lang.org`, `sh.rustup.rs`, `crates.io` all return no
  connection (probed), `apt-cache` has no `rustc`. So `cargo fmt`/`clippy`/
  `test` did not run. First verifier is CI on `ayazkussan/budlum#3`. If `fmt`
  disagrees with my wrapping, the next commit applies exactly what CI prints -
  I am not going to guess twice and call it a fix.
* No end-to-end advisory test. Building a second pending ticket on the same
  object means `open_deal -> open_challenge -> finalize_missed_challenge`
  twice, and `operator_cooldowns` is keyed by operator alone
  (`src/domain/storage_deal.rs:712`) - the fixture would need a second
  operator or a wider cooldown, which is a test-infra change, not this change.
  Logged as an open item; the primitive-level lock covers the rule.
* `unwired-guards` naming hole remains open (baseline counts only
  check/verify/validate/require/enforce/assert/reject/refuse/deny/guard;
  `assign_*` is invisible). Fixing it means widening a gate, which will find
  more names than this one - separate change, next loop.

