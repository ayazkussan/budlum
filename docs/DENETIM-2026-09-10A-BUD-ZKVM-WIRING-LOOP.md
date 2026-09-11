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

## 9. G-1 / G-2: a protection that was never compiled (found and wired)

**G-1.** `xtask/gates/src/gates/no_upstream_brands.rs` - 315 lines, a complete
gate with `run` and an 8-canary `self_test` - was **declared in no module,
listed in no `Gate`, named in no workflow**. Measured, not guessed:

| quantity | count |
|---|---|
| gate files on disk | 126 |
| `pub mod` declarations inside `mod gates` | 125 |
| declared but never invoked from `main.rs` | 3 (`exact_named_tests`, `named_tests`, `rust_literals`) - **refuted as a finding**: they are shared helpers, called by other gates (`super::exact_named_tests::check_exact_log`, `crate::gates::rust_literals::exclusive_scratch_dir`). Checked before claiming; the count difference is explained, not a hole. |
| files on disk that no declaration reaches | 1: `no_upstream_brands` |

**G-2 - why nothing caught G-1.** The two gates that exist to catch this are
blind in exactly the complementary way:

* `no_orphan_source_files::SCAN_ROOTS = ["src", "budzero", "wallet-core"]` -
  `xtask/` is not scanned, so a dead file inside the gate tooling itself is
  invisible to the orphan gate.
* `gates_are_wired` walks `scripts/check-*.sh` - shell gates only. A Rust gate
  module is not a script, so it is out of scope by construction.

So the file was never compiled either, which is the part that matters: an
uncompiled gate cannot fail, cannot be measured, and its canaries - written to
be run - were never run once.

**Fixed in this commit:**
1. `pub mod no_upstream_brands;` + a `Gate` entry (`name: "no-upstream-brands"`)
   in `xtask/gates/src/main.rs`.
2. Two steps in `.github/workflows/ci.yml` (`--self-test` then the real run),
   with a comment recording *why* the file was dead, so the next reader does
   not re-invent it.
3. **The gate as written would have failed the tree's own licence compliance.**
   Its exemption list was `["LICENSE.md","NOTICE.md","THIRD-PARTY.md"]`
   (root-relative), but the tree's attribution register is `docs/NOTICE` - no
   extension, under `docs/` - and the reading register is
   `docs/PROVENANCE_NOTES.md`. Both contain the names the gate forbids, which
   is exactly what its own header calls the worst outcome ("a gate that forbade
   attribution ... pushes a project toward a licence violation to stay green").
   Exemption is now by **stem** (`LICENSE`, `NOTICE`, `THIRD-PARTY`,
   `PROVENANCE_NOTES`) plus the two audit-mirror directories, whose whole
   content is the record of what was read.
4. Canaries 9 and 10 pin both halves: `docs/NOTICE` passes, `docs/ARCHITECTURE.md`
   with a researched name still fails - an exemption that widens to a directory
   is a disabled gate with extra steps.

**Predicted CI outcome, written before the result:** the gate runs clean.
Replicating its filter outside Rust: 855 files scanned (vacuity floor is 50),
**0 findings**; the third name in the gate's list has no occurrence of its own
anywhere in the tree - which is what the scan above says, and, as §27 records,
what the scan then contradicted.
If CI says otherwise, the gate is wrong and this section gets a follow-up
commit rather than an edit.

**Left open on purpose:** extending `no_orphan_source_files::SCAN_ROOTS` to
`xtask` would catch the next G-1 mechanically. Not done in this commit because
it also has to decide what it finds in `xtask/tools/src/bin/` and the crate-root
rules for a nested workspace - that is a change to the gate's scope, and it
should land green with its own measurement, not as a side effect.

## 10. Headline: the lineage does not compile, and CI cannot say why (fixed)

Step-level evidence (job → first failed step), which does not need the log host:

| job | failing step | prior SHA `9a23126` | my SHA `38a64fe` |
|---|---|---|---|
| `build` (rust.yml) | **Build** = `cargo build --verbose` | failure | failure |
| `Budlum Core` (ci.yml) | **Format** = `cargo fmt --all -- --check` | failure | failure |
| `Typos` | Repository scan (a finding = fail) | failure | failure |
| `Fork-Choice Invariants` | fork-choice tests | failure | failure |

Reading:

1. **`cargo build` fails on this lineage and failed on the previous session's
   SHA too.** Nothing committed on top of it has ever been compiled by CI, and
   the previous session's log entries that say "fix e4f472b", "fix fe5461e",
   "fix 8ba7eab" are, at best, unverified by the repo's own standard - "CI is
   the sole authority". A change to a non-compiling tree cannot be verified by
   a pipeline that dies at its first step: every job after Build is skipped or
   vacuous, and the 23 "pass" checks in the summary are the jobs that do not
   depend on compiling (gitleaks, semgrep, actionlint, zizmor, license, ...).
   That is not a green repo with four red jobs; it is a repo whose verdict is
   unknown and whose green checks are the cheap ones.
2. **`fmt` has been red since before this session**, so the fork's +1539 lines
   of Rust were never run through `cargo fmt` either.
3. **Why nobody caught it in the audit: the log host is unreachable, not
   broken.** `gh api .../logs` and `gh run view --log` both fail on
   `productionresultssa11.blob.core.windows.net` with EOF after ~1s while
   `api.github.com` answers instantly (probe: direct URL, not a CLI quirk).
   The workspace log recorded this as "fork log altyapisi EOF bulgusu" and left
   it as an open GitHub-side mystery. It is an egress property of the audit
   environment. **Closed with a mechanism, not a workaround:** annotations
   travel on the API host, so a failure step that would otherwise print to a
   log now also prints `::error::` lines.

Added in this commit (pure diagnostics, nothing softened):

* `rust.yml` / `build`: an `if: failure()` step re-running
  `cargo build --message-format short` and emitting its first 60 lines as
  annotations.
* `ci.yml` / after `Format`: same for `cargo fmt --all -- --check`, first 120
  lines.
* Both guarded with `set +o pipefail` only inside the diagnostic step - the
  verdict-producing steps are untouched, the original steps still fail, and the
  job is red either way. `security-scans-can-fail` still has nothing to object
  to: no `continue-on-error`, no `|| true` on a gate.

Consequence for the next loop: the build errors become readable, so the real
work is fixing the compile failure itself. Until that is green, **no claim about
any code change on this branch should be written as "verified"**, including my
own `assign_object` wiring in section 8 - which is exactly why that section says
"not compiled" instead of implying otherwise.

## 11. Dead public API: measured, classified, two alarms withdrawn

Inventory: `docs/AUDIT-DEAD-PUB-API-2026-09-10.md`. **205** of 1438 `pub fn`s in
`src/` have no reference outside `#[cfg(test)]`, tree-wide, with no quoted-string
dispatch either. 14 say so in their own comment; 191 do not; 28 of those carry a
security or consensus verb.

Read before filing any of them as a hole, and two claims were withdrawn:

* **Withdrawn:** "`slash_all_roles` dead ⇒ a slashed operator keeps validating."
  Not true. `AccountState::slash_validator` (account.rs:1357) sets
  `slashed/active/jailed/jail_until` and mirrors into the registry; the liveness
  path (blockchain.rs:2733) does the registry sweep *and* the account flags;
  the other pass (blockchain.rs:3978) sets `jailed` with a comment recording
  that the two paths once disagreed on the **state root** because `jailed` is
  hashed. The cross-role sweep is real (`registry::slash_cross_role`, reached
  from `registry::slash`), and the AI-role exclusion from it is deliberate and
  written down. `slash_all_roles` is a **duplicate of a policy that lives one
  layer down**, not a missing enforcement.
* **Withdrawn:** "`is_valid_chain` / `is_authorized_now` uncalled ⇒ validation
  skipped." Both are convenience wrappers; the wrapped functions
  (`validate_candidate_chain`, `PoAWhitelist::contains`) are the ones the
  production path uses.
* **Kept:** `store_bls_key` / `store_pq_key` / `bls_signing_available` /
  `pq_signing_available` - the PKCS#11 capability model exists and is tested,
  and **no binary or provisioning entry point calls it**. That is the directive's
  "HSM/PKCS#11" open point, seen from the other side: not "unsupported", but
  "supported and unreachable".
* **Action taken:** `Validator::slash_all_roles` now declares
  `WIRING: unwired` and names the live path, because its old doc claimed the
  guarantee at the wrong layer. That is the resolution the guard baseline itself
  prescribes ("moving it into a module that honestly declares `WIRING: unwired`"
  lowers the count; deleting a field-write pattern without deciding its shape is
  a different change).

The class, not the count: `no_idle_code` and `guards-are-reachable` both exist,
and the reason `assign_object`, `slash_all_roles` and 190 others sit outside them
is that both gates are keyed on names (`check/verify/validate/...`) or on
*reachability from a test*. A ratchet keyed on the `path:name` pair instead of
the verb is the only version of this that cannot be satisfied by renaming.

## 12. One broken build is wearing six red checks

Triage by step name + annotation exit code (no log host needed - the channel
added in section 10 is what makes this readable at all):

| job (8ebf838) | failing step | exit code | reading |
|---|---|---|---|
| `build` (rust.yml) | Build (`cargo build --verbose`) | 101 | **does not compile** |
| `Budlum Core` (ci.yml) | Format | 1 | fmt debt, pre-existing since `9a23126` |
| Cross-arch determinism (x86 + arm64) | Determinism test (genesis reproducibility) | **101** | not a determinism failure: cargo could not build the test |
| Node Classification | node classification tests | **101** | same |
| StorageProvider Gate | StorageProvider tests | **101** | same |
| Fork-Choice Invariants (38a64fe) | fork-choice tests | **101** | same |
| Miri UB Inspection | Miri - crypto crate | 1 | its own problem, not the build |
| Typos | Repository scan (a finding = fail) | 2 | real findings |
| Dependency Review | Dependency Review | - | advisory on the PR diff |

So the dashboard says "the fork lineage has eight problems". Measured, it says:
**one compile error, and everything that needs `cargo test` is reporting it as
its own failure.** Two consequences worth stating plainly:

1. Nobody should read "Determinism: failure" as "the genesis is
   arch-dependent". It is not evidence of that, and the job produced none. A
   red check that is downstream of a build error is not a finding; it is the
   build error wearing a costume.
2. The count of genuinely open items on this lineage is therefore much smaller
   than the CI page suggests - but the one that remains is at the root, so it
   gates everything else, including my `assign_object` wiring from section 8.

What is *not* explained by the build: Typos (real findings), the Format debt,
Miri's own failure, and the Dependency Review advisory. Those stay open.

The build error text itself: the `Build error surface` step is the first thing
on this lineage that can print it into the API. As of this commit that job is
still running, so the errors land in the next loop's reading; the step is in
the tree now, so it will be there for every run after it.


## 13. Lubot training loop: four crates, 2968 lines, 45 tests

The loop ran against the pattern sources listed in the directive and produced
four new Lubot crates, mirrored as `repo-lubot/patches/0006..0010`. The pattern
record - which source, which mechanism, what was deliberately *not* taken -
lives in the workspace repo as `skills/lubot/2026-09-10A-kalip-defteri.md`
(patch `0038`). No line was copied; the crates are `std`-only Rust written
here, and Lubot's own tree names no source repository.

| crate | holds | the invariant it enforces |
|---|---|---|
| `yetenek` (782 / 13) | skill cards plus the ledger of what proved them | a card is a *trigger*, and it reaches `Active` only when a run that closes it is recorded; a contradicting run demotes it without deleting it |
| `olcek` (715 / 10) | ceiling, watermark, reserved floor, two pools, drop ledger | a refusal carries numbers; an eviction carries a record; a silent truncation is impossible because `verify()` recomputes the sums |
| `kanit` (883 / 12) | scope lock, evidence, findings, paths, closure | no observation before a plan, no support from outside the scope, no closure on prose, and a closure that loses its evidence stops being a closure |
| `mimari` (588 / 10) | the pipeline table checked against the tree | every symbol the table names must be declared in the file the table names; an empty contract, an empty table and a one-row table are all violations |

`kanit` is the direct answer to the failure mode the directive asks to be
detected - an auditor writing the finding text where the fix should be. Prose
is admitted, counted, listed by `described_only()`, and can never close a claim.

**Not compiled.** There is no Rust toolchain in this sandbox and none can be
fetched (see section 7), so these numbers are line and test counts, not a
passing suite. What *was* verified here is the delivery mechanism: the five
patches were applied with `git am` to a scratch tree standing in for Lubot, and
the fourth one is generated with one line of context (`git -c diff.context=1`)
because a three-line-context hunk against the root `Cargo.toml` was measured to
fail on a manifest whose surrounding blank lines differ. That is the same
disease as G-1 and as the 205 dead `pub fn`s, in a new costume: **a crate that
is not in `members` is code that does not exist**, so the wiring hunk is its own
patch (`0010`) at the end of the series - if it fails to apply, four lines are
added by hand, and the four crates still land.

## 14. "The lineage does not compile" was an environment verdict

The annotation channel added in section 10 works. For job `102794189883`
(run 37, `6a2a951`) the API returned eleven annotations - the first CI output
this session has been able to read at all:

```
error: failed to run custom build command for `budlum-core v0.1.0`
Caused by:
  process didn't exit successfully: `.../build-script-build` (exit status: 101)
  --- stdout
  cargo:rerun-if-changed=proto/budlum/network/protocol.proto
Process completed with exit code 101.
```

`build.rs` is 35 lines: it locates `protoc` (`PROTOC` env, three known paths,
then `PATH`) and calls `prost_build` on `proto/budlum/network/protocol.proto`;
the `.expect(...)` is the 101. Grep across all 23 workflows in this fork:

| workflow | compiles? | installs protoc? |
|---|---|---|
| `ci.yml` | yes (250 cargo refs) | yes, 14 steps |
| `determinism.yml` | yes | yes, per-OS (incl. a checksum-pinned Windows download) |
| `miri.yml`, `rust-quality.yml`, `extra-tooling.yml`, `security-hardening.yml`, `benchmark.yml`, `semver.yml`, `diverse-double-compiling.yml`, `supply-chain-extra.yml`, `fuzz-nightly.yml`, `security-audit.yml`, `cargo-vet.yml`, `provenance.yml`, `docker-smoke.yml` | yes | yes (1-11 mentions each) |
| **`rust.yml`** | **yes (3 cargo refs)** | **no** |

So the `Rust` workflow - the one whose verdict section 10 recorded as "the tree
does not compile" - was failing in *codegen*, on a missing apt package, and its
`Run tests` step never started. The fork's source tree is not what that red X
described.

Two changes follow, neither of which touches an assertion:

1. `rust.yml` gains the same `Install protoc` step every compiling workflow
   already has. Adding a dependency cannot make a red check green; it can only
   let the check reach the point where it has an opinion of its own.
2. Both diagnostic steps are reworked for a fact measured from the response:
   **a check run keeps ten annotations and drops the earlier ones.** The 60-line
   tail I emitted became ~10 lines of the *end* of a build-script burst, and the
   actual cause line was truncated away. The steps now emit at most nine
   *selected* lines (for `Format`: up to eight `Diff in` headers plus a count
   line last, since the last ones are what survives). Nobody should "restore"
   the wide tail: a channel that overflows is a channel that lies.

Pre-registered for the next run, before reading it: with protoc installed, the
`Build` step of `rust.yml` either goes green or prints real `error[E....]`
lines from the sources - and the previous "eight independent failures" count
should shrink by exactly the jobs that were only ever downstream of this one. If
the build *still* says "failed to run custom build command", the suspect is apt
package resolution on the runner image, not this repository. What is already
known to be independent of this fix, and stays open: Typos (real findings), the
`Format` debt (a gate step, not a compile step), Miri's own failure, the
Dependency Review advisory, and the two determinism jobs - those workflows
install protoc, so their 101s are their own, and the reading in section 12 that
attributed them to a single compile error is now **corrected**: the shared cause
was `rust.yml` plus the fmt debt, not all eight.

## 15. F-16's actionable half: a lost replica's slot is now ticketed

The repair band measured two things and acted on one. `objects_below_own_repair_margin`
opened tickets for shards with **zero** active replicas; `under_replicated_shards`
- a shard at 1 of a target of 3 - was computed, logged, and left alone, so the
object with the *most* common failure mode got a warning. Reading the two
ticket constructors explains why the band had stopped there: `open_never_placed_ticket`
refuses a shard that already has a live deal (correctly), and
`open_expiry_reallocation` needs a `deal_id` the band never consulted.

The `deal_id`s are there. A shard at 1 of 3 got there by *losing* replicas, and
each loss left a closed deal behind - a free slot. `StorageRegistry::
open_repair_tickets_for_free_slots` now walks the under-target shards and opens
a replacement ticket per free slot, and the maintenance pass counts the result
into `registry_changed`, because a ticket is registry state and an epoch that
opened tickets must persist like any other.

Three decisions in it worth naming, since each is a place where a plausible
patch would be wrong:

* the guard is `(shard_id, replica_index)`, not `shard_id`. "This shard has an
  active deal" and "this slot has an active deal" are different statements, and
  only the second one means paying two operators for one slot - which is the
  exact hazard the never-placed path already refuses;
* only `DealStatus::Expired` history is reopened. A slashed deal's slot belongs
  to the slash path: it already opened the ticket, and it records which operator
  to bar. A ticket opened here would carry `slashed_operator = 0` and hand the
  slot back to the operator that just lost its bond;
* a shard whose whole history is still active yields nothing, and that is the
  stated limit rather than a silent skip: adding a copy that nobody lost is a
  replication request, not a reallocation, and the ticket type has no cause for
  it. Pinning it in a test is what keeps it from being read later as "the sweep
  is broken".

Five tests in `demand_driven_replication_tests` (`src/domain/storage_deal.rs`):
free slot ticketed (2 of 2 slots), slashed slot left alone (1 of 2), all-live
history yields no ticket, idempotence across two epochs (dedupe by
`failed_deal_id`, which is what makes an every-tick sweep safe), and a healthy
object untouched. `cargo fmt`/`clippy`/`cargo test` are not run here - no
toolchain; the file's delimiter balance and line widths were checked
mechanically instead, and the added lines break where rustfmt would break them.

### The AR-GE question this leaves open (written down, not blocking)

*Why asked*: the remaining half of F-16 needs a ticket that is not a
reallocation. *What it would do*: add a `ReallocationCause::ReplicationDeficit`
(or an equivalent registry-side demand queue) that `accept_reallocation_ticket`
can price, so an object at 1 of 3 with no lost deals can be brought up to
target. *Why it is not done here*: the acceptance path prices a *replacement*
against a *failed* deal's bond and deadline; inventing a cause without touching
`accept_reallocation_ticket` would open tickets that no operator can correctly
take, and the ticket map is registry state that reaches the state root - so the
change needs its own epoch-gated maintenance bump, the way
`BDLM_MAINTENANCE_PLACEMENT_V1` did. That is a consensus-visible decision, and
this loop's rule is to leave such decisions documented rather than smuggled in
through a helper.

## 16. Headline, corrected: the fork compiles. The tests are the red thing.

Run 39 (`6fba22a`) is the first run in this session whose steps could be read,
and it refutes section 10's headline. The `build` job of `Rust`:

| step | conclusion |
|---|---|
| Install protoc | **success** |
| Build (`cargo build --verbose`) | **success** |
| Build error surface | skipped (nothing to surface) |
| Run tests | **failure**, exit 101 |

So "the lineage does not compile" was true only of the workflow, not of the
repository: the tree builds. What is actually broken is `cargo test`, and that
is a different job - smaller, specific, and no longer describable as "everything
is red because nothing compiles". Two consequences, stated so they are not lost:

1. Section 12's attribution was wrong in one direction and right in the other.
   Right: several red checks were downstream of a shared cause and were wearing
   its costume. Wrong: the shared cause was not a source-level compile error, it
   was a missing apt package in one workflow - and jobs that install protoc
   (Determinism, and the gate jobs) were never downstream of it, so their 101s
   are their own and must be read individually.
2. Every claim in this report that I marked "not compiled" still stands as
   written, because the sandbox has no toolchain and I did not run `cargo`. What
   changes is what the next run can tell us: the edits in `0d01f0c` (the F-16
   repair sweep and its five tests) will be compiled and executed by CI on the
   next push, so they arrive with a verdict rather than with my word for it.

The `Test failure surface` step added in the same commit as this section is what
makes the next reading specific. A red `Run tests` is one of two bugs that look
alike from the job title: a test that failed, or a target only `cargo test`
compiles (the test modules import more than the library does, so an
unresolved-import in `src/tests/` is a red test step with a green build step).
The surface prints `error[E...]` lines for the second case and the `failures:`
block plus a failing-suite count for the first, with `--no-fail-fast` so one
pass enumerates every suite instead of the first one only.

Pre-registered, before reading run 41: with the tree building, the most likely
shape of a 101 here is a small number of real assertion failures in
`src/tests/*`, because a *test-tree compile error* would have been reported by
`cargo build --all-targets` in another job and was not. If instead the surface
prints `error[E`, the correction to write here is that `--all-targets` was not
run anywhere in this workflow.

## 17. The instrument I added was itself lying, and how that was measured

Run 43 (`81c3b01`) is the first run where `Build` was green, `Run tests` was
red, and the `Test failure surface` step I wrote *ran successfully*. It reported:

```
0 failing suites reported by `cargo test --no-fail-fast`; 36 lines of output were produced
```

Zero, with a red step and 36 lines of output - and every one of my grep
patterns anchored at the start of a line. The reason is in the workflow file,
three lines above the step: `env: CARGO_TERM_COLOR: always`. Cargo therefore
emits `ESC[1mESC[91merrorESC[0m: ...`, so `^error(`\[|`:)` cannot match, and the
count `grep -cE 'test result: FAILED'` returns 0. The same defect was in all
five surfaces I had added (build, format, typos, miri, asan); the earlier one
line of ANSI that survived in run 37's annotations - `^[[1m^[[91merror^[[0m:
failed to run custom build command` - was the evidence sitting in plain sight,
because that step grepped nothing and printed everything.

So the reading of section 14 was right about the cause and *wrong* about the
fix being complete, and the shape of the mistake is worth recording: a
diagnostic that prints nothing when it succeeds and nothing when it fails is
indistinguishable from a clean run. Every surface now strips the escape
sequences before selecting, and the strip was verified locally before the
commit:

```
$ printf '\033[1m\033[91merror\033[0m: failed to run custom build command\n' \
    | sed -e 's/\x1b\[[0-9;]*m//g' | grep -E '^error(\[|:)'
error: failed to run custom build command          # matches after the strip
$ <same input> | grep -E '^error(\[|:)'            # matches nothing before it
```

Pre-registered for run 44, before reading it: if `Build` stays green and the
surface still prints nothing, the next hypothesis is not "the strip failed" but
"the 36 lines are cargo refusing to resolve dev-dependencies" - which would put
the fork's `Cargo.lock` in scope instead of its sources.

The step count is unchanged (all six blocks still pass `bash -n`), the verdict
steps are still untouched, and the annotation budget is still <= 9 lines plus
the count line. What run 44 should now show: either `error[E...]` lines from a
test-only target that does not compile, or a `failures:` block naming the tests
that lost - and the 36-line output means one of the two, not a slow suite.

## 18. The strip fix did not ship: the sed program was missing its command letter

Run 45, 46 and 47 all completed, and run 47 is the one to read because it is the
newest of them (`cancel-in-progress` retires the rest). Its `build` job:

```
Install protoc: success          Build: success
Build error surface: skipped     Run tests: failure
Test failure surface: success
```

and the annotation the surface produced, verbatim:

```
0 failing suites reported by `cargo test --no-fail-fast`; 0 lines of output were produced
```

Run 43 produced `36 lines of output were produced`. So the fix changed the line
count from 36 to 0, which is not "the ANSI lines stopped matching" - the tee'd
file itself became empty. Only one stage sits between `cargo test` and that file,
and the committed text of it is:

```
sed -e '\x1b\[[0-9;]*m//g'
```

There is no `s` in front of the expression. `sed` does not see a substitution, it
sees a stray `\` and then reads the rest as an address, and says so:

```
$ printf 'x\n' | sed -e '\x1b\[[0-9;]*m//g'
sed: -e expression #1, char 17: unterminated address regex
```

`sed` exits, the pipe closes, `tee` writes an empty file, and the surface - whose
`set +o pipefail` line exists precisely so that a red step stays red - reports
"0 lines of output were produced" as a *fact about cargo*. It was a fact about the
one command that was supposed to have been fixed.

Six occurrences, four files: `rust.yml` (build surface, test surface), `ci.yml`
(fmt), `typos.yml`, `miri.yml` (miri, asan). One mechanical substitution, applied
six times, wrong every time. The five that were not executed - `Build error
surface` skipped because `Build` is green, fmt/typos/miri on other workflows whose
own verdicts were already red for other reasons - carried the same defect silently
for the whole time.

### What the local verification missed, and why

Section 17 records a shell transcript proving the strip works. That transcript is
true, and it verified a command *retyped in the transcript*, not the command as it
appears in the file. The check that would have caught it is one line long and now
runs on the file rather than on the intent:

```
prog=$(python3 - <<'PY'
import re
print(re.search(r"sed -e '((?:[^']|'')*)'", open('.github/workflows/rust.yml').read()).group(1))
PY
)
printf 'a\033[31mb\033[m\n' | sed -e "$prog"       # must exit 0 and must print 'ab'
```

`bash -n` was run on all six blocks and passed - it validates the shell grammar,
and a quoted `sed` program is opaque to it. A workflow edit is only verified once
the extracted string has been handed to the program that will consume it.

Measured the same way, on a fixture shaped like real colored cargo output (an
`error:` line behind four escape sequences, a `failures:` block, a `panicked at`
line, a `test result: FAILED` line), the pipeline in the file now selects 5 lines
with the strip and 4 without: the escape-prefixed `error:` line is exactly the one
that was invisible before, which is the whole point of the change.

### The pre-registered prediction, scored

Section 17 committed to a reading before looking: *"if `Build` stays green and the
surface still prints nothing, the next hypothesis is not 'the strip failed' but
'the 36 lines are cargo refusing to resolve dev-dependencies'"*. The first half
came true - green build, nothing printed - and the second half is wrong, and it
was wrong in the way most worth writing down: I had pre-committed to an explanation
of the *content* of the output while the actual failure was in the *existence* of
it. The count line that §16 added as decoration (`N lines of output were
produced`) is what overruled the prediction, because 0 and 36 are different
answers and only one of them is about cargo.

So the failing test list is still unread after four runs, and no statement about
which tests are red belongs in this report yet.

### Corrected next reading

Run 49 or 50 (whichever survives the cancel-in-progress rule) is the first run
where this surface can be believed. Two outcomes, both already actionable:

- `error[E...]` / `could not compile` lines selected - then a test-only target is
  broken, `Cargo.lock` and dev-dependencies are in scope, and section 14's verdict
  ("the fork compiles") needs to be narrowed to "the library compiles".
- a `failures:` block naming tests - then the fork is sound, specific tests lose,
  and the audit goes to those tests one by one.

`Build error surface`'s copy of the same defect also gets fixed here, and it is
worth noting which one of the two surfaces was *ever* executed: the test surface,
because it was the only one whose job reached a state where `if: failure()` was
true. A diagnostic that runs once a month is a diagnostic that is tested once a
month.

## 19. The dead-API ratchet the audit asked for, and the number that would not re-derive

`docs/AUDIT-DEAD-PUB-API-2026-09-10.md` closed with a proposal not taken: a
`dead-pub-api` ratchet, sorted `path:name`, may-only-shrink, one exemption token,
Rust not shell, and - the sentence that decided the shape of this work - *"it must
land with its 205 entries recorded first: a gate that fails on arrival gets
switched off, which the tree has already learned twice."*

That constraint is the reason most of this section is about measurement. A gate
whose baseline does not match its own measure is worse than no gate: it teaches
the next person that the answer to a red ratchet is to switch it off.

Shipped: `xtask/gates/src/gates/dead_pub_api.rs` (437 lines), `pub mod` plus a
`GATES` entry in `xtask/gates/src/main.rs`, two `ci.yml` steps (canary then run)
next to `guards-are-reachable`, a README bullet, and
`.github/dead-pub-api-baseline.txt` with **219** `path:name` lines. It borrows
`strip_test_mods` from the sibling gate by widening that one function to
`pub(crate)` - two gates that disagree about what a production file is would be
worse than one gate with one bug, so the definition is shared rather than copied.

### What the measure says at the current tree

2168 candidate declarations in `src/**` (excluding `*_tests.rs` and `tests/`
files, `#[cfg(test)]` modules removed), **219** with no whole-token occurrence of
their name anywhere in `src/ crates/ examples/ benches/ budzero/ xtask/ ops/
.github/ config/ proto/` across `.rs .toml .yml .md .sh .py .json`, once their own
declaration lines and the fourteen-line `WIRING:`/`Convenience:`/`exposed for`
exemption are subtracted. Re-measured after the gate's own source and the CI edits
were added, because the corpus includes `xtask/`: the gate's module doc mentions a
`registry::seed`, and a name borrowed as an example is a name now counted as live.
Findings 219, baseline 219, new 0, stale 0.

### The audit's own number, and why it is not here

At `8ebf838`, the commit the audit was written at, the rule *as the audit states it*
measures 2037 declarations and 209 unreferenced. Deduplicated by name: 1536 and
212. The audit's 1438 and 205 are neither. No variant tried here reproduces them,
and the honest reading is that the method paragraph in that document is not a full
specification of the extractor that ran. The correction is written into the audit
itself, where the numbers live, rather than left for someone to trip over.

209 - 3 + 13 = 219: three entries released by the exemption token, thirteen added
by walking `pub const fn`, which the audit's extractor skipped. Skipping `const` is
not a cosmetic gap - `pub const fn` is exactly where a pure, tempting, uncalled
helper goes - so the gate is broader than the report and the baseline header says
so with arithmetic instead of an adjective.

### The verification, and the false alarm that produced it

No toolchain here: nothing was compiled, clippy-checked or formatted, and six
commits including this one are unpushed because the GitHub token in this sandbox
expired ("The github.com token in GH_TOKEN is no longer valid"), so CI has not
seen any of it. What could be checked without a compiler was:

- A byte-level re-implementation of the gate's rules, written from the Rust source
  line by line, reproduces the same 219 entries as an independent regex-based
  implementation. Both agree with the committed baseline exactly.
- All seven canary phases of `self_test` were simulated against the same rules and
  each produced the verdict the Rust asserts, for the reason the assert names.
  Two of them were wrong before that simulation and are worth listing, because both
  were the kind of canary that passes while testing nothing:
  - the staleness phase failed for growth instead of staleness until both fixture
    entries were recorded first, after which the only complaint left is the stale one;
  - the exemption phase had a fixture whose helper *was* called by a driver, so it
    proved that a call site counts, not that the doc token exempts. The driver is
    gone; the helper is uncalled and the token alone has to save it.
- Long lines, `#[allow]`s, `unwrap`/`expect`/`panic` in `run()` (the
  `gates_do_not_panic` ratchet may only shrink, and a new file joining its baseline
  would have been a fresh debt), and function length against `too_many_lines`'
  default 100-line threshold: `run()` is 57 lines after the scan, the dead-set, the
  baseline reader and the report were split into four helpers.

### One thing the gate cannot do, kept in the open

A name mentioned in a `.md` file counts as a reference. That is the audit's rule,
kept so the two counts are the same measure, and it means prose can buy what a call
site earns - writing `/// Convenience:` is an exemption you can read in the diff,
and writing a README paragraph is one you can too. The gate does not police the
difference and this section does not pretend it does: what it polices is the
direction of the list, and 219 lines with authors' names on them in `git log` are
harder to wave through than an uncounted drift.

## 20. Forty-seven skipped steps: the gate suite was behind a formatting diff

The dead-API gate was pushed and the CI run came back with its two steps marked
`skipped`. Not failed - skipped. Following that thread is more important than the
gate itself.

`ci.yml`'s `budlum` job is 56 steps: checkout, Rust, protoc, cache, then `Format`
(`cargo fmt --all -- --check`), then clippy, the feature matrix, `Test`,
`cargo doc`, then 39 steps of gates. GitHub Actions skips every step after the
first failure in a job. On this branch `Format` is red - the tree has an
unformatted region that this sandbox cannot repair without a toolchain - so run 50
recorded:

```
1 failure      (Format)
8 success      (checkout, Install Rust, protoc, cache, Format diff surface, ...)
47 skipped     (Clippy, Test, cargo doc, and every gate)
```

The consequences are not subtle. The clippy pedantic/nursery ratchet did not
measure anything. The test suite did not run in this job. `gates-are-reachable`,
`no-new-shell-gates`, `dead-public-api-is-ratcheted` and the other 125 gates did
not execute - including the canaries whose entire job is to prove those gates can
fail. And the job *looked* explainable, because the one step that ran after the
failure was `Format diff surface`, which reported the format diff it was written to
report. The failure was legible; the silence behind it was not.

This is the same disease the workflow already documents at the `Unreachable gate`
pair - "a protection that is not compiled is not a protection" - one level up: a
protection that is compiled but placed after an unrelated red step does not run
either, and a job summary that lists one failure and forty-seven skips reads like
the failure was the story.

The fix is a job boundary, not a reordering. Reordering inside the job moves the
masking to whichever step is now first (`Test` fails while the `budlum-core` test
target does not compile, so putting the gates after `Test` would skip the gates
again). So `ci.yml` has a `gates:` job with its own checkout, toolchain and
protoc, holding the 39 gate steps, and `budlum` keeps format, clippy, tests and
docs, where a formatting failure legitimately stops the job. Two details that
could not be left implicit:

- `badges-are-current` reads `/tmp/libtests.log`, which the authoritative `Test`
  step writes. In the new job that log is regenerated by a `cargo test --lib`
  step carrying `continue-on-error: true`, because the verdict on the suite
  belongs to `budlum` and a copy of another job's verdict, placed in front of 39
  gates, would reproduce the masking exactly.
- Unmasking has a cost that is worth paying loudly: `badges-are-current` now runs
  against a log from a test target that does not compile, so it will report a
  finding. That is the gate being right - the README's test-count badge cannot be
  confirmed while the suite cannot build - and it is what forty-seven skipped
  steps were hiding.

Checked here: the file parses as YAML and yields 26 jobs, 13 steps in `budlum` and
44 in `gates` with all 36 gate invocations (18 canary/verdict pairs plus the
clippy-extra pair, which stays where its input log is produced); no moved step
references a step id or `if:` context from `budlum`, and `grep` confirms the only
cross-step file reference was the badge log handled above. Not checked: nothing
ran - there is no runner or toolchain in this sandbox - so actionlint and the real
scheduling behaviour arrive in the next run, and this branch's `Format` debt stays
red on purpose: the gate suite no longer depends on it.

## 21. The three errors, found by line and column, and one red check that is not mine

Run 51 (`389bb31`) is the first run whose failure surface can be read end to end,
because the surface step now pairs each `error[...]` with its `--> file:line:col`.
`budlum-core (lib test)` had three errors. Two were the ones fixed in the previous
commit (the missing `use` for `Executor`, the `let _ = challenge_id;` silencer of a
name bound in a different test), and clearing them let rustc reach type and borrow
checking, which is where the remaining complaints live - an onion, not a surprise:
a resolution error suppresses the borrowck pass that would have named them.

The located set, all in test-only code:

- `src/chain/storage_economics_tests.rs:525:9` and `:529:19`, two `E0502`:
  "cannot borrow `blockchain` as mutable because it is also borrowed as immutable".
  `all_reallocation_tickets` returns `Vec<&StorageReallocationTicket>`, and the test
  kept the reference across a balance credit and an escrow call. Fixed by copying
  the one field the test later reads (`ticket_id`) out of a scoped block, rather
  than cloning the ticket and keeping the borrow alive for no reason.
- `src/tests/pow_light_client.rs:222:31`, one `E0308`: `timestamp_ms` is `u128`
  while `1_000 + height` infers `u64` from the `height` of `(1u64..=4)`. Fixed with
  `1_000 + u128::from(height)`, which is what the neighbouring lines in that file
  already spell out (`timestamp_ms: 1_000,` needs no annotation because a bare
  literal infers).

Whether the target now compiles is a question for run 52, not for this document:
there is no toolchain in this sandbox, so nothing here was type-checked.

### The Typos check on this PR is not caused by this branch

At run 51 the spell gate was still red, and the local tree with the *same* pinned
version reported nothing - a discrepancy worth chasing to the bottom rather than
calibrating away. Two facts explain it:

- `pull_request` runs check out the **merge ref**, not the branch head, so the
  scan sees files that exist only on `main`.
- `main` (d07225d, "Add files via upload", today) contains
  `01a08674-f696-72cd-b135-24e6cc8e1539 (2).patch` at the repository root: 2 310 869
  bytes, 37 000+ lines, a browser-suffixed duplicate of an Arena patchset artifact,
  unreferenced by any file in the tree.

`typos` 1.48.0 on a worktree of `origin/main` reports exactly 6 findings, every one
of them inside that file (`tha`, `ser` x2, `Objec`, `flate` x2) - the same six words
run 51 showed once the patch mirrors I authored were excluded. So this branch's
contribution is fixed (the `sentinal` x2 in the exported commit messages, now out of
scope for the gate because they are generated text re-verified by `git am`, not
hand-edited here), and the remaining red belongs to `main`: it needs `git rm`, not a
whitelist entry, and not from this PR.

The download path was also measured, because it will matter to the next reader:
`release-assets.githubusercontent.com` - where release tarballs redirect - is
unreachable from this sandbox, the same class of block as the Actions log host. The
pinned binary could not be fetched to reproduce CI's findings; a worktree of
`origin/main` run through the same version from PyPI could, which is the route to
write down.


## 22. lubot mirror: rebuilt onto the only base that survives, 20 patches (`a3896e2`)

The sandbox was reset mid-turn: `/tmp` was wiped and `budlum`'s HEAD was rolled
back to `88970e3` with 43 paths left dirty on disk. Recovery was
`git fetch origin <branch>` → `git stash push -u -m snapshot-restore-safety-1208`
→ `git reset --hard FETCH_HEAD`. The lesson is recorded where it belongs: only
pushed state is durable, and anything in `/tmp` is scratch. The lubot series was
re-derived from the mirror in this repo, which is exactly why the mirror has to
be appliable rather than merely present.

It was not. The mirror documented its base as branch `olcum-disiplini` @ `12bc9ac`,
and that branch no longer exists on the fork: `main` @ `37d32c9` is the only ref.
Against the base that actually exists the files applied 9/18 (0/18 in a fresh
clone until a committer identity was set - an environmental failure that looks
exactly like a stale-context rejection, so it is written down as a trap). So the
18 commits were replayed onto `main` and re-exported, and the tool that does it
now ships as `repo-lubot/tools/rebuild_series.py`.

Four defects were found in my own tooling, each with a rule attached:

| defect | symptom | rule |
|---|---|---|
| `patch -N` skipping hunks | reported "18/18 clean" while `members` stayed 8 and ten crate manifests were never written | a structural self-check over the result must pass before export; a per-patch clean line is not evidence |
| folded `Subject:` header | every rebuilt commit lost its tail ("...`cagisi bir`" instead of "...`cagisi bir veridir`"); subject fidelity 7/18 | unwrap continuation lines, then compare against the pristine originals |
| double-encoded subjects | `=?UTF-8?q?=3D=3FUTF-8=3Fq=3F...` inherited from feeding a previous export back into `git commit -m` | decode until no encoded word remains |
| magic separator date | all 18 rebuilt commits dated `Mon, 17 Sep 2001` | the `Date:` header is the commit date, the `From <sha>` line is git's placeholder |

Fidelity after the fixes: 18/18 subjects identical to the pristine originals, and
`git am -3 --whitespace=fix` of the regenerated files - as committed - into a
clean clone of the real fork applies **20/20 with zero `.rej`**.

Two findings were measured rather than guessed, and one of them is now fixed in
the series itself. lubot's README ratchet line claims `191 tests` while the base
tree contains 178 `#[test]` attributes: a 13-test overstatement that predates
this work. The applied tip measures 327 (178 base + 149 from the twelve crates),
with 0 `#[ignore]` and no doc examples, so 327 is the number cargo can only
confirm, not revise - patch 0019 sets it that way, with the counting method in
the message rather than an inflated carry-forward. Patch 0020 fixes the inventory:
`docs/CRATES.md` listed ten crates while the series adds twelve, because
`izolasyon` and `denetim` never got a row; its acceptance test is `rows == crate
dirs` on the applied tree. And a real gap in lubot's gate runner is reported, not
patched from a bundle: `gates/check.py --all` catches only `SystemExit`, so the
first gate that shells out to `cargo` aborts the whole suite with
`FileNotFoundError` in a toolchain-less environment. Before that abort, 5 of the
38 gates pass on the applied tree; `--list` reports 38, matching the README.

Nothing here was compiled, and the mirror says so in its own "What is NOT
verified" section: rustup, `static.rust-lang.org`, `index.crates.io`,
`registry-1.docker.io` and GitHub release assets were probed and are all
unreachable from this sandbox.

## 23. Six red canaries, one unreadable reason, and the job that now says it

Pushing `154dfb3` produced a run where the new `gates` job executed for the first
time: `Test log for the badge gate` ran, then `Badge canary (independent of the
log)` failed and every later step was skipped. The check-run annotations for it,
and for five other jobs' canary steps, contain exactly one useful fact each:
`Process completed with exit code 101.` That is all a `run:` step can say, and the
log itself is on `productionresultssa6.blob.core.windows.net`, which is blocked
from this sandbox - the same class of block as the Actions log host and GitHub
release assets. So the run could not be read, only described.

What the API does answer is attribution. At the branch point (`88970e3`) the
Budscan job was fully green, its canary included; at `fc08330` - the commit that
added `xtask/gates/src/gates/dead_pub_api.rs` and registered it - Budscan's canary
is failure/101, Coverage's moves from its real red (`Measurement + ratchet gate`)
to `Gate canary (vacuous-gate protection)`, and Node Classification and
Fork-Choice flip the same way. Six jobs, one shared thing: they all invoke
`cargo run --manifest-path xtask/gates/Cargo.toml`. Since `main` exits 1 on a gate
finding, 101 is not a gate finding: it is cargo failing to build the crate, or a
panic in a canary. Both hypotheses survive locally:

- build error in the `xtask/gates` workspace - plausible because that crate is its
  own workspace, so Budscan's green `Clippy (pedantic, -D warnings)` step (a root
  `--workspace` run) proves nothing about it; `gates_are_wired` was written exactly
  because a gate nobody runs is not protection, and the mirror image - a gate that
  nobody *builds* - is the same blind spot;
- panic in the new `self_test` - less likely: the fixture uses no `unwrap`/`expect`,
  every slice is bounded (`saturating_sub` for the lookback window, `j < len` before
  `j + 1`), and `for_each_token`/`read_baseline` cannot index off a char boundary.

Guessing which one is worth nothing to the next reader, so the job was made to say
it. Two new steps in `gates`, both verified by executing their extracted bodies
against a stub `cargo` on `PATH` (the workflow-text rule from §20, applied here):

1. `Build the gate binary` - `cargo build --release | tee`, then
   `exit "${PIPESTATUS[0]}"`, because `bash -e` without `pipefail` would otherwise
   take `tee`'s status and a red build would report success; its
   `Build surface (annotations)` companion strips ANSI, greps `^error` with four
   context lines, and prints a count line last, so a truncated sample cannot be
   mistaken for the whole error and a green build cannot be mistaken for a silent
   red one. Measured: exit 101 propagated, five annotations, `1 error line(s)`.
2. `Canary surface (annotations)` at the end of the job, `if: failure()`, re-runs
   `-- --self-test` and annotates `FAIL |error|thread |panicked` with `-A1`. The
   first version matched `^panicked` and missed the line a real panic prints,
   `thread 'main' panicked at ...` - found by running it against a stub whose
   stderr was a genuine panic, which is the only way this class of bug gets caught
   before a push. Measured: three annotations plus the count line, exit 0, so the
   surface never becomes a second failure on top of the first.

Not done, deliberately: nothing was weakened to make six red canaries green - the
gate is not commented out, no `#[allow]` was added, and the baseline was not
touched (it is settled, and re-deriving it from a tree this sandbox cannot build
would be exactly the unmeasured number this report keeps complaining about). The
next run's `gates` job will name the cause in an annotation; if it is the build, the
fix is a source edit and a re-push, if it is the canary, the canary is wrong and
gets corrected to fail for one reason.

## 24. The six red canaries: closed by the surface, in one round

The push that added `Build the gate binary` answered the question the previous
section left open, in the annotation stream rather than in a log:

```
error[E0308]: mismatched types
   --> src/gates/dead_pub_api.rs:408:35
error[E0308]: mismatched types
   --> src/gates/dead_pub_api.rs:418:35
error: could not compile `budlum-gates` (bin "budlum-gates") due to 2 previous errors
```

Both sites are `write(dir.join("src/lib.rs"), <String>)` inside the dead-pub-API
canary's fixture builder. The fixture writer was a local closure annotated
`(PathBuf, &str)`, and a closure's parameter type is fixed by the first call site:
the fixtures spelled as string literals type-checked, the two built with
`.concat()` - owned `String`s - did not. `a6d1c1c` replaces the closure with a
private `write_fixture(PathBuf, impl AsRef<str>)`, so the spelling that was wrong
is no longer the only spelling available, and it stays private because a `pub fn`
in this file would grow the very public-API baseline this gate ratchets.

Causal chain, worth keeping because it is the general shape of the problem:
`xtask/gates` is its own cargo workspace; the root `Clippy` step in Budlum Core and
the per-crate clippy in Budscan never build it; the first step anywhere that invokes
`cargo run --manifest-path xtask/gates/Cargo.toml` was a canary; a build failure in
a canary step is reported as the gate finding a defect; six jobs went red on
`fc08330` and stayed red for the whole session, including one (`Budscan`) that was
fully green at the branch point. A gate that nobody builds and a gate that finds
something are the same CI shape from the outside.

What the surfaces did: the failure moved onto a step named for the build, carried
rustc's message plus its `-->` location as annotations, and the `Canary surface`
step that re-runs `-- --self-test` reported 3 reason lines instead of silence. The
first version of that surface would have reported nothing at all - its filter was
`^(FAIL |panicked|error)` and a real panic prints `thread 'main' panicked at ...`,
found by executing the extracted step body against a stub `cargo`, which is the
same rule the Format surface was built under (§20) and the reason the rule exists.

Still open after this fix, with the local command that measures it:

- `cargo test --lib` at the root fails with exit 101 in the `gates` job (its
  `Test log for the badge gate` step, deliberately `continue-on-error`) - the real
  verdict is Budlum Core's `Test` step, which is skipped because `Format` is red;
  rustfmt debt cannot be paid from this sandbox, so `cargo fmt --all` is the fix and
  the 8-file list is in the Format annotations;
- whether `budlum-gates` now compiles is the next run's answer, not a claim here;
- the lubot mirror at `repo-lubot/` stands at 21 patches, `git am -3` 21/21 on a
  clean clone of `main`@`37d32c9`, with `tools/rebuild_series.py --table` generating
  the README table from the patch files themselves (verified byte-for-byte against
  the committed table before it was written down).

## 25. Making the loop usable from the outside: order, visibility, and the recovery that worked

Four things changed after the type error was closed, all of them about whether a
maintainer can act on what this repo reports.

**Style checks no longer stand in front of verdicts.** `Format` was the fifth step in
`budlum` (masking Clippy, the pedantic ratchet, the feature canary, `Test`, `cargo
doc`), the fourth in `budzero` (masking `Check`, `Clippy`, `Test`) and the fourth in
`budscan` (masking that job's tests and both canary steps). Each `Format` moved to
the end of its job. The edit was verified as a *pure move* - the sorted line
multisets of the old and new file are identical apart from the load-bearing comment
added above the trio in `budlum` - because a step reorder done by hand is exactly
where a dropped `run:` block hides.

**The gates workspace gets measured, not yet enforced.** `cargo clippy --release
--manifest-path xtask/gates/Cargo.toml --all-targets` now runs in the `gates` job
with `continue-on-error`, and its `Clippy surface (annotations)` step publishes the
headline warnings with their `--> file:line:col` and a total count as `::notice::`.
Monitoring-first is the existing `izleme modu` shape of the pedantic ratchet, and the
reason for it is procedural: a lint surface that becomes visible and binding in the
same commit is a surface that gets its visibility reverted. The separator lines
`grep -A1` inserts between groups are filtered out; an annotation that reads `--` is
noise wearing a diagnostic's clothes.

**Both step bodies were executed, not inspected**, against a stub `cargo` that emits
coloured clippy-shaped output - including the case where the log is empty, which
must still exit 0. The same harness is how the earlier surfaces were checked, and it
is cheap: extract the step's `run:` from the YAML, put a fake binary first on `PATH`,
run `bash -e`. `bash -n` cannot see any of the three bugs this session found in
workflow text (the `^panicked` filter that missed a real panic line, a `printf`
format whose backticks were live command substitutions, and a `-A1` group separator
that would have become an annotation).

**The sandbox reset is now a known procedure, not an emergency.** The repository
directory was re-cloned underneath the session while the working tree kept its
files: HEAD was at `88970e3`, the branch tip and all its commits were on the remote,
and `ea11834` existed only as file content on disk. `git fetch` + `git reset --mixed
FETCH_HEAD` (moves the ref, leaves the worktree alone) put HEAD at the remote tip,
the surviving work appeared as a normal delta - 20 re-exported patches differing by
the `[PATCH NN/21]` counter plus one untracked file - and it was committed as
`28238be`. What did *not* survive was anything outside the repo: the sidecar scripts
in `/home/user` and every `/tmp` clone. That is why the series generator and the
README-table generator now live in `repo-lubot/tools/`, and why `--table`'s output
was diffed byte-for-byte against the committed table before the file claimed it.

Open, with the command that closes each:

- does `budlum-gates` compile now? next run's `Build the gate binary` step, and if it
  is green, whatever the six canaries then say is true about the tree;
- rustfmt debt: `cargo fmt --all` locally, in the first job that has a toolchain; the
  file list is in the `Format diff surface` annotations;
- `cargo test --lib` at the root (Budlum Core's `Test` step, now no longer masked) -
  the suite's own verdict, which this sandbox can only cite from CI annotations;
- GitHub auth from this sandbox expired mid-work (`gh api` → `Bad credentials`,
  `git push` → credential prompt refused), so four commits sit locally:
  the mirror's 21st patch, the report's §24-§25, the `--table` generator and the two
  CI steps above. They push in one go when the token comes back; nothing was stashed
  or rebased to make that true, and the tree is clean while waiting.

## 26. The gates job was masking gates too, and the badge gate is now the finding

`a6d1c1c` proved the fix for the crate (`Build the gate binary` green: `budlum-gates`
compiles, and the two `E0308`s were the whole story), and it exposed the next layer of
the same disease inside the job built to cure it: `Is the badge current` went red and
**35 gate steps after it never ran**, among them the dead-public-API gate whose
baseline is the reason the previous paragraph exists. A step sequence is a mask no
matter what the steps are about.

The job now runs differently. `Build the gate binary` has `id: build`; each of the 36
single-command gate steps appends its merged stdout+stderr to `/tmp/gate-out.log`
(`2>&1 | tee -a`, then `exit "${PIPESTATUS[0]}"` so the step still carries its own
verdict) and runs under `if: always() && steps.build.outcome == 'success'` - guarded by
the build, because a compile failure would otherwise print 36 identical failures and
bury the one line that matters. A final `Tally (every red gate, annotated)` reads the
file, annotates every `FAIL [gate]` headline with its message, prints the count, and
exits 1 so the job cannot go green while a gate is red. Three steps are deliberately
untouched (the two that build and test crates, and the `Test log` step): they can
redden the job by themselves, and the Tally says out loud when it found no headline
while the job was red anyway - the case where "no annotation" must not be readable as
"clean".

Verified by executing the sequence with a stub `cargo` in which exactly one gate fails:
45 run steps executed, every gate after the failure still ran, 37 outputs appended to
the shared file, one headline annotated with its message, Tally exit 1. Locally
verifiable, that is; the `if:` expressions are not - Actions evaluates them on the
runner and the `Repo Lint` job's actionlint is the check for them. The first version of
the Tally annotated `2:FAIL [...]` because `grep -n` prefixes a scratch file's line
number; dropped, since the gate's message already names the gate, the file and the
reason.

A scan across every workflow for the same pattern (`Format`/`lint`/`typos`/`style`
ahead of a `test|build|clippy|canary|gate` step) now returns nothing - the three moves
in §25 plus this job's restructure were the whole class. What the badge gate will
report next is the README's test count against the count the run measures; when the
annotation names both numbers, the badge moves to the measured one. Until then nothing
in this file claims the suite's size, and `cargo` remains unavailable here, which is
why each of these statements is about a step that ran, not about code that was built.

## 27. What the unmasked job reported: four findings, one of them mine

Run 60 (`44302dc`) is the first gates run where the steps after the red ones actually
executed - 48 steps ran, 3 failed, nothing was skipped. That is the restructure paying
for itself, and it changed the queue more than any of the fixes in §26 were written to.

**The gate binary compiles, and the crate is green in the environment it lives in.**
`Build the gate binary` success; `Dead public API canary` and `Dead public API did not
grow` both success, so the 219-entry baseline is being compared against a real scan in
CI for the first time; `B.U.D. core builds and its tests pass` success.

**Finding 1 - the monitoring clippy step found debt in the file written to reduce debt.**
`Clippy (gates workspace, izleme modu)` was reported as `success` by the job - it is
`continue-on-error` - while its surface annotated `6 warning/error headline line(s)`,
and the messages were `error:` lines: three *denied* lints in `dead_pub_api.rs`, at
184 and 207 (`case-sensitive file extension comparison`) and at 282 (`variables can be
used directly in the format! string`). `xtask/gates/Cargo.toml` sets clippy `deny` for
those groups, so under `--all-targets` a lint is a compile error and the test target of
the gate binary does not build at all. Fixed by taking the extension test onto
`Path::extension() == Some(OsStr::new("rs"))` - a helper, `has_rs_extension`, so both
sites read the same rule and a future third one cannot drift - and by inlining the
format argument. No `#[allow]`, and the case-sensitivity is kept exactly as it was,
commented, because accepting `.RS` would be a second undocumented change to what the
audit counts.

**Finding 2 - the badge gate refused to compare, and that is the right answer.**
`FAIL [badges-are-current]: test log records failures; refusing to compare badge against
a red run`. So the plan in §26 - read the measured count off the Tally and set the badge
to it - cannot execute: `cargo test --lib` on this branch has failing tests, and a badge
pointing at a number from a red run is exactly the kind of number the gate exists to
refuse. 2896 stays. The work item is the failing suite, not the badge.

**Finding 3 - a second red gate, and one of the 12 places is this document.**
`FAIL [no-upstream-brands]: 12 place(s) name a product that was read for research`. The
headline is all a `run:` step yields (the file list stays in the log, unreadable from
here), so the places were re-derived: a python replica of the gate's own filter - same
three names, same skip lists, same exemption for the attribution stems and the two audit
mirror directories - over this branch's tree scans **858 files and finds exactly 1**
place: a sentence in §9 of this report that asserts the corpus name "has no occurrence
anywhere in the tree", which the sentence itself falsifies. Reworded, and the earlier
prediction is left standing above it, because a prediction that turns out wrong is the
record; the correction belongs next to it. The other 11 places are not in this branch:
`pull_request` runs against the merge commit with `main`, so they are main's content, and
they go on the queue with the rest of the pre-existing reds rather than being hushed by
editing the exemption list.

**Finding 4 - Budlum Core is masked by a different step, so Format-last was not enough.**
Its job now reads: features, then `Test` - but `Clippy` sits ahead of both and fails
(`-D warnings`), so `Test`, `cargo doc` and `Format` were skipped again. The always-runs
`Format diff surface` did its part: 17 `Diff in` hunks annotated, all in root
`src/chain/` files this session never touched. The lesson is general and now written down
in `docs/CONTRIBUTING.md`: *the rule is not about `Format`, it is about any red analysis
step being ahead of verdicts*. `Clippy` and its ratchet were moved after `Test` and
`cargo doc` - the same one-step move as §25, verified the same way (the move is pure: the
step dicts are unchanged as a multiset, every other job is byte-identical, YAML parses,
26 jobs), and the next run will report the suite verdict with `Clippy` and `Format` still
red where they belong.

**Not verified, still:** `Repo Lint`'s annotation says `actionlint did not run: No such
file or directory`, so none of the new `if: always() && steps.build.outcome == 'success'`
expressions have been checked by a linter - only by the runner agreeing with them, which
is weaker than it sounds: a typo in a step id fails open, silently skipping every gate.
That is the one failure mode the restructure introduced, and it is why `Build the gate
binary` keeps its own step visible as the first thing after the toolchain steps.

## 28. The third masker, and why the answer stopped being ordering

`aaf491e` pushed the Clippy-behind-Test move, and its run answered before the queue
finished with it: in Budlum Core, `Feature matrix: pq-ml-dsa solo` fails, and `Test`,
`cargo doc`, `Clippy` and `Format` were skipped behind it - the identical shape for the
third time, with a different step in the offending position each round (`Format`, then
`Clippy`, now a feature build).

Ordering was the wrong target. A sequence has to break somewhere, and any single
placement only guarantees that the step chosen to be last is the one that gets hidden.
What the gates job already does - and what the other three cargo jobs do now - is remove
the sequence: `id: rust` on the toolchain step (plus `id: protoc` in `budlum`), and
`if: always() && steps.rust.outcome == 'success'` on every cargo step, 21 in all across
`budlum`, `budzero`, `budscan`. A missing toolchain still short-circuits, because there is
then no cargo to run and that failure is the finding; a red *verdict* no longer hides the
next one. No `continue-on-error` was added anywhere: the steps that fail still redden the
job, they just also say what they measured.

Verified as far as it can be without a runner. The edit is a pure move plus one key per
step (step names and bodies identical as ordered lists, every other job byte-identical, 26
jobs, YAML parses), and the guards were then *evaluated*, not read: a small interpreter for
the subset this repository uses (`always()`, `success()`, `failure()`,
`steps.<id>.outcome == '…'`, `!`, `&&`, `||`, parentheses) was run against a simulated red
`Feature matrix`, which reports `budlum` as 13 of 13 steps running, and against a simulated
red `Clippy` in `budzero`, 7 of 7. In the gates job the same simulation with a red
`Build the gate binary` skips exactly the two steps whose guard asks for that build to have
succeeded - which is the guard doing its job.

The restructure has one failure mode that the runner will not catch, and it is now a checked
invariant instead of a hazard: `steps.atypo.outcome` is simply not `'success'`, so every
step behind such a guard is skipped and the job looks green with nothing in it. That is
`ops/scripts/check-step-reachability.py` - it reads step headers (id, name, if) without a
YAML dependency, reports a reference to an id that no step declares and a reference to an id
declared *later* in the same job, and answers `--fail JOB:STEP` with the run/SKIP table. Two
steps in the gates job use it, a canary (`--self-test`) and a verdict (`--check`), neither
guarded on the build so they still report when cargo is broken; measured clean across 23
workflow files. It is not a Rust gate, so it runs the `FAIL [step-reachability]` headline
shape the Tally already greps.

Recording a second-hand result, because it is the argument for shipping checkers with their
own self-test: the first version of that script passed its happy path on the real workflows
and failed its own fixtures, and reading the failures back found two genuine bugs - a
`strip("'\"")` that ate the closing quote of an expression, and a field parser that did not
see `- name:` on a step's first line and so compared empty strings to each other. Both were
invisible until the fixtures existed.

**Not verified:** the runner's own evaluation of those expressions (only Actions and
actionlint judge them, and actionlint is still absent from the runner - `Repo Lint` says so
every run, and my interpreter could differ from Actions somewhere the workflows do not
currently go), and whether the next run is the first one to print `budlum`'s `Test` verdict
while `Feature matrix`, `Clippy` and `Format` stay red.

### 28.1 Two measurements that close the loop opened by the previous sections

The run for `aaf491e` finished the parts this thread was waiting on, and both of them are
numbers rather than arguments:

* `FAIL [no-upstream-brands]: 11 place(s)` - one fewer than the 12 the previous run
  reported, and the only difference between the two trees is the sentence §27 reworded. So
  the attribution was right, and it was proved by a subtraction rather than asserted: 11
  places are main's, 1 was this report's, and after editing one sentence the count moved by
  exactly one. The remaining 11 are on the queue with the other pre-existing reds.
* `gates-workspace clippy: 0 warning/error headline line(s)` - the three denied lints found
  in §27 were the whole of it. A monitoring step whose measurement is zero is a gate
  waiting to be turned on, so `Clippy (gates workspace, izleme modu)` is now
  `Clippy (gates workspace)` without `continue-on-error`: it refuses. It graduated only
  because the job had just become per-step independent - the same command that would once
  have swallowed 35 verdicts now swallows none, which is the reason to stage a check as a
  count before making it red. It carries the job's cargo guard, and the two steps that
  looked optional - the log producer for the badge gate - needed it too: simulated with a
  red clippy step, `Test log for the badge gate` skipped, which would have reached the badge
  gate as a missing log and come out looking like a content finding. After the guard, the
  simulation says every cargo step runs while clippy stays red, and only `Install protoc`
  and `Build the gate binary` remain unguarded, which is correct: they are what the guards
  are measured against.

The checker that proves the guard expressions is now the thing checking this workflow's own
edits - `python3 ops/scripts/check-step-reachability.py` ran clean after both the clippy
graduation and the log-step guard, and `--fail 'gates:Clippy (gates workspace)'` is how the
claim above was produced.

## 29. The Lubot prompt draft: intake, and what twelve questions changed

The user opened PR #4 (`211d366`) with a single file, `lubot-system-full.md` - 3 992 lines,
198 KB - and said: this is the draft I wrote for Lubot, edit it professionally and put it
in your PR. Alongside it came the answers to the second batch of six questions, which
settle the loop's posture:

* **lubot wiring is delete-biased, at my discretion** ("silmeye yatkın dedikten sonra en
  mantıklı süreci kendin ilerle") - so the 53-entry dead-public-API baseline shrinks by
  giving genuinely-needed functions a real caller and deleting or `pub(crate)`-ing the
  rest, crate by crate, esik -> tools -> olcek in that order, "sıkı kodlama" with the
  whole workspace verified before a patch ships.
* **Only this PR carries the work** - the other PRs will be closed by the user; every
  commit ships here, and every commit title names its repo (`budlum:`/`lubot:`) so a log
  line cannot be attributed to the wrong tree.
* **Push per commit** was re-confirmed as the choice; the PR stays open for the user to
  merge. The lubot clone now lives as a bare cache at `/home/user/lubot-upstream` (263 KB,
  outside git, survives what /tmp does not).

The intake itself is now `docs/LUBOT-SYSTEM-PROMPT.md`, and its edit is structure-only by
construction, not by promise: a fingerprint multiset (markdown-insensitive, line-based) of
original vs published shows exactly three removed lines - the stale hand-written contents
bullets - and additions consisting solely of the 37-entry generated contents list and two
organizational headings (`## Part A`, `## Tool surface`), with the 24 tool headings demoted
to H3 under the latter, the 9 stray H1s and 22 other headings that had been pasted inside
XML prompt blocks converted to bold labels, and every `**`-hard-break/trailing-space
artefact of the paste removed. **No prompt instruction was added, removed or reworded** -
that is the whole contract of this pass, and the verification is the proof. Provenance is
in the front-matter comment: the original bytes are one `git show 211d366:lubot-system-full.md`
away for as long as PR #4's commit survives, which is why the PRs get closed by the user,
not deleted by me.

The gate angle, because a doc that enters the tree enters every gate: the file names no
researched product (the three forbidden names: zero occurrences), carries no conflict
markers, and contains no `#[allow]`-class content - CI will judge the rest, including
Typos, which has already been taught to ignore nothing new here since no words were
coined. The one thing CI cannot see coming is size: a 200 KB doc is 5% of the tree's
text weight, and `no-upstream-brands`'s file counter (vacuity floor 50) only goes up, so
no gate baseline moves.

A second thing this turn had to survive: the sandbox re-clone again. It landed mid-commit -
the docs commit existed locally on the *branch-point* tree while every earlier commit lived
only on the remote - and the push raced it. Recovery was the standing habit: fetch,
`reset --hard` to the remote tip (files first, the tree was intact because the re-clone had
restored it), cherry-pick the one fresh commit, push. Net state: `9673840`, nothing
unpushed, working tree clean. The commit-title-repo convention from this turn's answers
exists precisely so those recovery moments read unambiguously from a log.

## 30. The twelve questions answered, and policy's first patch

The first six questions were skipped; their defaults are stated so they stay reversible:
the red backlog keeps cheapest-first order by what the unmasked jobs report, the badge
stays frozen at 2896 while the suite is red and the gate refuses, the reachability checker
stays a std-only script plus its two CI steps, and Semver/Dependency Review stay
report-only.

The six that were answered are in §29; their first product is mirror patch 0022. It is
worth dwelling on one detail, because it is the whole point of building a ratchet: the
deletion commit had to shrink `gates/dead-pub-api.baseline` from 53 to 51 *inside itself*.
The stale-entry rule - a baseline line that stops being dead fails - never had to fire on
real work before; every prior proof was a synthetic canary. The first live trip came from
the policy the user picked, delete-biased with discretion, and it did what it was
designed to do: forbid deleting code and leaving the ledger claiming the debt is still
there. The same reflex moved the README's test count 327 to 325 by recount, not
decrement.

What 0022 deleted, and why the phrasing matters: `tools/chain`'s ceilings flow was not
demonstrably unused by everything - it was unused by anything the tree can show. The
module header promises "a request string in, a response string out" for reading
*outcomes*, and ceilings negotiation sits outside even that. When the only other mention
of a function in the repo is the gate that counts it, the function is a wish. Deleting it
also fixed a symmetry the allow-list had lost: `bud_aiGetCeilings` was permitted by
`is_allowed_method` with no parser behind it, and the surface test now asserts the method
set equals the parser set in both directions - the removal is pinned, so the next person
who re-adds one half has to re-add the other.

Remaining order, per the answer "sırayla hepsi": esik's 10 unreached entries next - they
are mostly accessors and builders of a public ledger type, so the honest question is not
"trim 10 methods" but "who should be reading a feature-activation ledger at all", which
is a wiring question with the same three-way evidence rule; then the tools remainder
(operator's 4, lib's 1, chain's surviving 2), then olcek's 6. Every patch: caller grep
across all file types, gate scan, am on a clean clone, README claims recounted in-patch,
"not compiled" labeled.

## 31. The esik seam, the settlement layer, and a door that had to close

Two more mirror patches shipped, and they are the two halves of the same policy answer:
wire what has a real caller, delete what has none, and let nothing sit in between.

Patch 0023 wired `esik`. The question §29 left open - ten unreached entries whose honest
shape was "public API of a ledger nobody reads" - resolved toward the seam this repo
already had: the inventory command. `cmd_envanter` now loads `training/activation.jsonl`
through a new `cli/src/activation.rs`, builds the registry strictly through `esik`'s own
gates (`declare`, `admit`, `ratify`, each refusal reported with its line number), and
prints the one schedule line valid at a caller-supplied `--at EPOCH`; no clock enters the
inventory, which is why the epoch is an argument and not a measurement. An absent ledger
is a legal empty schedule; a corrupt one is an error. All ten entries got genuine call
sites, the gate's own scan then proved all ten baseline lines stale, and the same patch
deleted them: 51 to 41. One recorded miss during construction: a test asserted the
absence of a word in a line that always carries it; caught in the read-through, not by
a compiler, which does not exist here, and rewritten to assert the count text instead.

Patch 0024 is the user's own named system: the Universal Settlement Layer, `crates/usl`,
the format a cold wallet reads off the stick. Its shape was forced by what the repo can
actually guarantee, and saying so is the crate's header, not a footnote: there is no
crypto here and none will be invented - a std-only crate must not grow signing code it
cannot be audited for, so signing stays delegated to the cold device and USL guarantees
only the byte contract around it. That contract is what `muhur` already holds - a chain
that recomputes digests from the bytes rather than trusting the stored seal, which is
"the entire mechanism" in its own words. A media is therefore one sealed file: the
canonical payout lines (address, `M.mm`, memo), fee, window, and a `SEALED` tail; the
reader replays every line through the same validating constructors, re-seals, and
refuses on the first byte that disagrees. `lubot usl make|check` is the first consumer -
the CLI parses the flags, pairs per-payout `--to`/`--amount` counts, writes the padded
media name, and the `check` path echoes each payout line, which is exactly where the
accessor trio gets its call sites. The crate shipped with zero new baseline entries.

`from_media` replays through `Indexed`, and that is what forced the design's one side
effect: `muhur` has no public way to seal an `Indexed` - its only sealing path was
`chain_mut`, a `pub fn` handing out `&mut Chain` so a caller could append after sealing
and move the tip under the index's feet. The correct shrink was not deleting the entry
but replacing the door: `Indexed::finalize` now seals through the index, `chain_mut` is
deleted, the tree's only user - muhur's own test - was rewritten through the proper
door, and the baseline moved 41 to 40 as deleted surface, not widened exemption. This is
what "delete-biased, but never delete a contract" looks like when both rules apply to
one patch: an escape hatch with a test behind it is a missing method, not an unused one.

Design decisions that will be asked about later, recorded where they were made: amounts
are the layer's own canonical unit (`MINOR_PER_MAJOR = 100` as USL's constant), and no
caller's currency is modeled - each side maps at its own boundary, because a hot side
meaning satoshis and a cold side reading lira is the bug this spelling exists to refuse;
fields cannot contain tab, quote, backslash, control characters, or double spaces, since
the media grammar is space-tokenized and a field that can embed its own separators can
rewrite its line; duplicate payout lines are refused as assembly mistakes, because
paying twice is a different memo; and the window rule fires at `with_maturity`, not only
at `check`, so an author can hold a half-built envelope but cannot seal one that ends
where it begins - `check` repeats it so reader and writer enforce the identical rule.

Verification floor, unchanged in kind: no cargo exists (measured), so every patch's floor
is the full 39-gate run passing on the applied tree, a clean `git am -3` of all 24
patches from bare `37d32c9` (24/24, gates re-run rc=0 there, baseline 40/40, tests
counted 334 by grep on the applied tree, never by decrement), and a line-by-line read
pass before commit. The series README's claims were recounted in-patch: 24 files, 21
crate directories equal to members, 176 base + 158 series tests, the gate-executed
ledger 53→51→41→40.

One workstream closed by measurement, not by choice: pushing the series to
`ayazkussan/lubot` itself was tried from the worktree with the configured credentials -
fetch works, and the dry-run push returns 403 for `arena-ai-coding-agent[bot]`. The bot
cannot write to that repository; the answer-6 fallback from §29 is therefore active: the
mirror under `repo-lubot/` is the delivery vehicle, the system-prompt doc stays
budlum-only, and the human one-command push recipe is owed as a small note in the mirror
README. The queue after USL, from the same recount: olcek 6, operator 4, kanit 4,
yetenek 5, takip 3, kuyruk 3, training 3, anlama 3, the documented `tools/chain` 2
(header-bound debt), and the singles down to zero, each patch following the 0022/0023
shape: caller grep, wire or delete, stale lines deleted in the same commit, counts
recounted, gates rc=0, fresh am, "not compiled" labeled.

## 32. Two deletions, and a gate caught masking its own names

Patches 0025 and 0026 ran the shrink policy against the two crates §31 left queued, and
0026 produced the loop's most uncomfortable finding: a gate of this series' own making
was keeping dead code alive.

0025 deleted `olcek`'s reporting tail - `is_granted`, `ordinary_used`, `closing_used`,
`headroom`, `pressure_admissions`, `was_truncated` - after the esik comparison: esik's
methods existed for a ledger file the repo ships (`training/activation.jsonl`), so the
honest move was to write the reader; olcek's methods existed for a budget ledger nothing
in the tree produces and no artifact awaits, so the honest move is the deletion. The
patch also caught a deletion residue a compiler would have flagged: `Ledger::pressure`
became a written-but-never-read field the moment its getter left, which clippy `-D
warnings` would refuse; it went in the same patch with its two increments, since the
pressure moment survives where callers already see it, as the `Admitted::GrantedUnderPressure`
outcome. Two tests died with the accessors that were their only subject; one test that
asserted a deleted line beside real claims lost only the stale line, and the test whose
name promised the deleted counter was renamed to what it still proves - renaming over
quiet weakening, both recorded in the commit. 40 to 34.

0026 went after `operator.rs`'s registry rules and found them guarded by the suite itself.
`operator-sync-rules` asserted seven names into operator.rs's source text; three of them
(`compute_bond_ok`, `same_model_hash`, `CheckpointWindow`/`both_active`/`old_retired`)
described records the tree never holds - no registration file, no operator set, no window
producer - and were referenced only by their own fixtures, which the ratchet's test-strip
rule already declares not-a-caller. The gate, added by this series in the 001x era to keep
the Aşama 7 report honest, had become the exact mask §28 described: asserting a name's
presence where the honest question is a caller's. Deleting behind it would have failed the
suite "for the crime of having no caller while keeping the rule."

The fix keeps the gate and retargets it: the effort-tier half stays asserted because it
has doors (`answer_budget` checked at three CLI entries, the hashed tier re-verified by
`chain.rs::parse_request`, its self-test fixtures untouched), while the registry half gets
inverted - the gate now refuses the deleted names if they return without a caller, so the
ratchet's one-way direction is preserved by the gate itself, and its docstring says the
list re-grows only in a patch that carries the records. The README's own scope line (11:
registration and bond "live in the node"; Lubot is "a client, not the layer") had already
decided the discretion question; the table row advertising those rules as a tools feature
contradicted it, and both became true by deletion. Recounting the row surfaced a second
finding: it claimed 47 tests when the crate measured 41 before the patch and 41 minus four
deleted fixtures after - a stale doc claim from before this series, caught by the same
rule that catches stale code: counts are recounted from the applied tree, never maintained
by hand, including when the recount embarrasses the claimant. 34 to 30.

Distribution after 0026, measured with `awk -F: '{print $1}' | sort | uniq -c` as §31's
lesson demands: yetenek 5, kanit 4, takip 3, kuyruk 3, grant/training 3, anlama 3,
tools/chain 2 (gate-bound, header-documented), and one each in tools/lib (a getter whose
pair lives), read/perception, muhur (the `is_finalized` branch no reader can reach),
izolasyon, index, grant/lib, denetim. Each next patch repeats the shape: grep every file
type for call sites, wire when an artifact awaits the reader, delete with the same-patch
stale-line removal otherwise, recount docs in-patch, gates rc=0, clean `git am -3` of all
26, and label the standing limit: nothing here is compiled, because no cargo exists in
this environment - measured, not assumed.

## 33. The cold wallet got harder, and the identity layer met the tree it lands in

Two workstreams, one turn's order: harden what shipped, then open what was designed.

**USL, hardened (mirror patch 0028).** The read-through of 0024's own contract found
four holes, and every fix is written assuming the adversary can do what the crate says
they can: recompute the seal, because muhur's digest is public arithmetic, not a key.
(1) The reader ran `check` but not the construction-time rules - a duplicate PAY line
re-sealed into a media parsed through a direct push and was accepted; the duplicate rule
moved into `check`, the read path feeds `add_payout` (the writer's own door), and the
test forges a re-sealed media with a smuggled line and asserts refusal. (2) Numbers were
parseable but not canonical: `SEQ 007` read fine and wrote back a different file, which
breaks byte stability - the one promise a sealed media makes to the next audit - so
every numeric line is now canonical-only and the read path re-renders the manifest and
requires the file's own lines back, in order. (3) `total` accumulated in u64 and could
wrap silently - the one number a human compares off the stick - so sums run in u128
inside `check`, past-range batches are refused at both sealing and reading. (4) `usl
make` overwrote an existing media without comment, the worst failure at a cold wallet:
writing moved into `write_media` (create dir, `create_new`) and the second write to an
occupied sequence fails with the OS saying so; a new batch means a new seq, there is no
overwrite path to reach by mistake. Three tests joined (usl now 9, tree 328→331 by
recount); baseline unchanged - every name was already reached or is reached by cli.

**Identity, coded against the tree.** The architecture doc (`docs/KIMLIK-MIMARI.md`,
user draft stored verbatim plus its spoken addendum) names its anchors, and two of them
do not exist: there is no `VerifierRegistry` and no `DomainFinalityAdapter` in `src/` -
the real, provable pattern is `PermissionlessRegistry` in `src/registry/` and the
anchoring is `GlobalBlockHeader`'s StorageRoot precedent. The doc's schema-version claim
is also stale: `CURRENT_STATE_SNAPSHOT_SCHEMA_VERSION` is already 4, and new subsystems
enter as `#[serde(default)]` optional fields (registry, bns, nft all did) rather than by
bump. The slice therefore shipped without snapshot persistence - that is the state-root
owner's decision, asked as Q1 below, not this module's to make. What did ship
(`src/registry/identity.rs`, 7+1 tests, `f692ef3`/`7aabe74`):

- the DID door: `did:bud:<64 lowercase hex>` with a round-trip rule that *refuses*
  uppercase rather than normalizing it - a DID two spellings apart is two DIDs;
- `IdentityRecord`: closed `MethodKind::MlDsa87` (the node's own post-quantum primitive -
  no new crypto, no BBS supply-chain drift yet), live-by-epoch methods, quorum-guarded
  guardian sets that validate at construction (no self-guardian, no duplicate, no
  unreachable or absent-without-guardians threshold);
- `CredentialCommitment`: fields are salted commitments `H(tag|schema|name|salt|value
  digest)` - the doc's fixed principle "raw data never on chain" is structural here, a
  birth date hashed alone would be a dictionary - and the root is an in-order Merkle
  fold with duplicate-last odd pairing; selective disclosure is a positional sibling
  path (`verify_disclosure` recomputes the leaf from the preimage, wrong salt/value/
  schema/leaf-path all return false, and a re-ordered or "SORT of parsing" file is
  refused as media, same lesson as USL's byte stability);
- the PoA gate inside the registry: writes refuse non-PoA domains *before touching
  state*, so even error shapes do not let a wrong-domain caller probe contents, and a
  `ConsensusKind::Custom("poa-pretender")` is refused because the gate matches the
  variant, not a string;
- lifecycle rules the audit needs: exact re-issue refused, born-dead refused, future
  issuance refused, issuer must be registered or hold a live subject key, revocation is
  not a toggle, and `is_credential_valid` recomputes the root from the fields and
  refuses on disagreement - the recomputation-not-trust mechanism muhur and USL use;
- recovery semantics: quorum counts *unique real guardians* (a stranger's approval is
  not counted, a double-appearance counts once), rotation revokes all live methods at
  `now` and lands the new key - signature verification of approvals is stated as the
  transaction door's job, following `view_grant::GrantAuthorization::verify`, because a
  registry that half-checks signatures is a registry that checks none.
- the digest trio (`credential_issue_digest`, `credential_revoke_digest`,
  `recovery_digest`): every axis a signature could be moved across (chain id, issuer,
  subject, root, epoch, rule tag) changes the digest, and the rules cannot collide on
  crafted material because each folds its domain tag first - the grant layer's
  discipline, inherited rather than reinvented.

The consent flow the addendum describes - a wallet screen naming which field opens, the
value landing only in the requester's wallet, "each piece of information is an NFT" - was
measured before writing code: `src/storage/view_grant.rs` already implements exactly that
*binding shape* for confidential storage (grants bound to grantees, digest-revoked,
epoch-opened). Identity supplies the per-field commitments that make "which one field" a
checkable question for that layer; whether field commits also enter `NftRegistry` as
per-field tokens is a state-design fork, asked as Q5 below, not assumed here.

Lubot-side state after the same turn: mirror at 28 patches, baseline 29 (USL added zero,
0028 added zero), tree tests 331, gates rc=0, fresh-clone am 28/28. Queue unchanged after
that: kanit 4, yetenek 5, takip 3, kuyruk 3, grant 3+1, anlama 3, chain 2 (gate-bound),
muhur/read/izolasyon/index/denetim singles. budlum CI was measured saturated this turn
(12 queued/pending runs); the identity module's compile proof is CI, and its output is
recorded the moment it clears.

## 34. Six answers, and persistence with the version as the refusal

The identity questions were answered in one batch while CI sat queued; every answer moved
a plan, and two moved measurements in this report itself: `src/domain/finality_adapter.rs`
DOES exist with a `PoAFinalityAdapter` (§33's claim was about the doc's exact type name
`DomainFinalityAdapter`, which no file defines - the correction is recorded in the doc's
new appendix rather than by editing §33's prose).

The persistence slice implements the Q1 answer, "bump to 5", and the bump's meaning is
the repo's own argument for why `poa_onboarding` shipped WITHOUT one, inverted: an absent
field may default silently when "empty" is the truth about old state, but identity state
is *digestive* - a node that quietly dropped a revocation set would validate a snapshot of
revived credentials. So `identity: Option<IdentityRegistry>` sits `#[serde(default)]` for
loading, yet hashes into the snapshot digest only under `schema_version >= 5`, and
`migration_report` refuses above-CURRENT versions by name. The consequences are pinned in
tests rather than asserted in prose: a v4 digest is byte-identical whether the field is
None or Some (old pinned digests stay reproducible; the new test checks exactly this), the
three legacy-blob tests (v2, v3, v4) drop the key from their assembled blobs, assert its
absence at the byte level, and refuse to fabricate a value from the default path, and the
canary that locks the serialized field set grew "identity" in the same edit its own text
demands ("extend both old-blob tests in the same edit" - three, now).

`IdentityRegistry::root()` folds records (subjects, per-method keys WITH their revocation
epochs, current credential root, guardian sets with their thresholds), credential ids and
the revocation set, in `BTreeMap` order, under a domain tag; `is_empty()` gates whether
the account state root folds it at all (`b"identity_v1"` in `calculate_state_root`, the
bns pattern verbatim), so "no identity state" and "identity state hashing to zeros"
cannot share an anchor. A twin-node test proves two registries built through different
call sequences agree on the root with only the root exchanged.

The slice also caught, before commit and by reading the serde not the diff:
`BTreeMap<[u8; 32], _>` serializes its KEYS as JSON arrays, and the snapshot is JSON -
a populated registry would fail at runtime write time, not at compile time, and the
tests here could not have caught it (empty maps serialize fine). Registry credential and
revocation storage became hex-keyed strings with the `[u8; 32]` public handle unchanged;
the hex is order-preserving, so the root fold and every deterministic iteration are
untouched. The same trap in the same repo is why `Address` has a manual `Serialize`; the
lesson generalizes to this registry and is written at the field.

State of the queues at this point: budlum - identity module (10 tests), persistence,
digest trio; pending next: document-fill (the Q5 flow: template slots bound to
disclosures, requester-bound receipt digests), folder-NFT over `NftRegistry`/`deed`,
`identity_root` on `GlobalBlockHeader` with its proto conversions, and the full-cycle tx
door (Q6) through mempool/RPC. lubot - 28 mirror patches, baseline 29, 331 tests, gates
rc=0; shrink queue waits behind the identity work by the user's ordering. CI remained
queued/pending through the whole turn (12 runs measured); "not compiled" labels stand and
these four commits are exactly what its compiler gets to grade.

## 35. What the tx reconnaissance measured, and what was therefore written

Full-cycle tx wiring (Q6) was scoped before being attempted, and the scope won: adding a
`TransactionType` variant touches six production files (proto conversions are hand-mapped
per variant in BOTH directions, rpc admission, blockchain replay audit, mempool gates),
neither `protoc` nor `cargo` exists in this sandbox (measured), and a blind six-file edit
would make PR #3 red for plumbing rather than reviewable as identity. What landed instead
is the door's content as pure, tested code: `IdentityTx`/`GuardianApproval`/
`authorize_recovery` in the registry module (derived-address rule and digest rule verbatim
from the grant layer; unsound approvals die before counting; the no-feature build refuses
everything, so recovery is unavailable rather than guessable), and the folder layer in
`src/socialfi/vault.rs` (memberships never copies, ordered open, all-checks-before-any-
change move, forest kept a forest by transitive cycle refusal, `BDLM_VAULT_V1` root in
the registry family, `Referenced` split from `NonEmpty` when the read-through caught the
first name describing an empty folder as non-empty). One read-through catch each side:
the vault's own doc comment promised what the new variant now delivers, and the identity
re-export list grew with the types rather than after them. The collision search also
returned one finding for the record: budlum's code comments already use "USL genesis" for
the hash-domain activation event (bns/registry.rs, core/block.rs) - unrelated to the
lubot-side crate of that acronym; no change, noted so a future reader does not "align"
two different things sharing letters. Remaining in the identity arc, in order: the single
executor arm + proto variant (when a protoc-bearing environment or CI iteration can own
it), `identity_root` on `GlobalBlockHeader` (Q2's field, same proto slice), the
presentation RPC read path, and folder↔`NftRegistry` ownership checks at the tx door
(the vault is ownership-blind by design; the executor consultates `NftRegistry` beside
it - a sentence that will move from docs to code with the proto slice).
