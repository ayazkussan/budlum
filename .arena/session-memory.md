# Current State
_What is being worked on right now, what is unfinished, and the immediate next steps. Always update this section. If the outcome of the most recent action is unknown, say so explicitly._

**RECOVERED + ALL PUSHED; monitoring CI.** Mid-turn incident, resolved: sandbox was reset under us — working files held the full `fee87c4` tip state but `.git` rolled back to session base `88970e3` (branch pointer AND object store; the 5 local commits' objects were gone; user fixed auth meanwhile: "bağlantı var"). Recovery was content-level: fetch origin → branch back to pushed `9a6c2ca` → remaining dirty diff verified to equal EXACTLY the 5 lost commits' file footprints (spot-checks: account.rs diff had 0 execution_domain lines = base already carried pushed slice; identity_tx_door.rs present from ce0261f) → 5 commits recreated at original boundaries with original messages → pushed `888094d..3e3cd6d` + docs addendum `d8c885f` (§36 gained the reset paragraph; "same content, new hashes" recorded). Remote tip = local = **`d8c885f`**, tree clean.
- **CI at `d8c885f` draining**: sole fail so far = Dependency Review = KNOWN INFRA (repo lacks dependency-graph setting, human-only toggle at Settings→code security; arc touched zero Cargo dependency lines — do NOT retry as code). Everything else pending (Budlum Core, Gates, Domain Tag Inventory, coverage, E2E...). 7 commits of arc (ce0261f, 9a6c2ca, + 5 recreated) await first compile verdict — the hand-mapped proto tables (prost codegen in CI build), new executor arms, snapshot schema-6 tests, and the two hand-written test files are what gets graded. **NEXT: poll `gh pr checks 3` / `gh run list` until drain; classify every red against the arc; fix code-caused reds immediately and push to `arena/01a08a1a-budlum` (retry push each turn if auth dies again; never force-push; amend only while unpushed).** Known previously-unreadable job fails at older head (Miri UB, Cross-arch determinism arm, Udeps) — check their verdicts against CURRENT head, they may have been head-local or infra.
- Identity arc DONE (proto+executor `ce0261f` → identity_root V5 `9a6c2ca` → RPC reads `888094d`); vault arc DONE (body `1fad908` → schema-6 `7464fbd` → door+locks `d1277c5`). Both documented DENETIM §36 + KIMLIK-MIMARI status (open decisions closed).
- THEN queue: cross-domain VERIFIER slice (commitment + Merkle proof vs finalized `identity_root` in receiving domains — GlobalBlockHeader already anchors it; KIMLIK-MIMARI's separate line, NOT yet scoped/recon'd); then lubot shrink queue (28 patches through 0028, 331 tests, baseline 29) per user ordering.
- Standing: user "devam ben burada yokum işi durdurma" — keep shipping.

# Task
_What the user asked for, in their terms. Preserve active acceptance criteria and consequential scope decisions. Remove obsolete narrative when necessary, but do not lose requirements that still affect the work._

_unchanged_

# User Constraints & Corrections
_Explicit standing instructions and corrections the user stated about how the work should be done, including anything the user rejected. Only what the user explicitly directed — never infer. Never drop an entry unless the user reversed it or it applied only to a task that has finished._

- "devam ben burada yokum işi durdurma" — continue, user away, do not stop the work.
- "harika ama durmasan da olurdu bağlantı var" (2026-09-11, after I ended a turn stopped on the auth outage) — do NOT idle-stop on transient blockers when the user says the connection is back; retry and keep going. Lesson internalized: per-turn push-retry-first, and small pushed commits are the actual protection against sandbox state loss (proven this turn: zero content lost because every slice was committed+pushed per-commit).

# Workspace
_Files and directories that matter: path, plus one line on what each contains and why it is relevant. Never include file contents; the workspace itself is the source of truth._

`/home/user/budlum`, branch `arena/01a08a1a-budlum`, tip `d8c885f` (pushed; PR #3 head https://github.com/ayazkussan/budlum). Arc files (all committed): `src/core/transaction.rs` (Identity tag45 / Vault tag46 variants, `encode_identity_tx`+`encode_vault_tx` canonical encoders beside Pollen trio, gas arms contract_call_gas*2, tamper test incl. op-tag pins in `v29_signing_tests`), `src/core/account.rs` (`execution_domain: ConsensusKind` + `vault: VaultRegistry`, 4 constructors each, `identity_v1`/`vault_v1` state-root folds + root-sensitivity test), `src/execution/executor.rs` (Identity+Vault arms at match tail w/ amount frame rules; NftTransfer/NftBurn vault locks before registry call), `src/consensus/mod.rs`+`poa.rs` (`ConsensusEngine::domain_kind()` default-PoS-deny, PoAEngine override), `src/chain/blockchain.rs` (5 execution_domain stamps: new_with_genesis ~864, reorg ~4951, get_state_snapshot ~5041, both snapshot applies; `identity_root` in build_global_header empty-gated like ai_root), `src/settlement/global_block.rs` (identity_root field, BDLM_GLOBAL_BLOCK_V5 presence-tagged fold, trio tests), `src/crypto/domain_tags.rs` (V5 inventory swap), `src/chain/snapshot.rs` (CURRENT=6, `vault: Option<VaultRegistry>` `>=6` gate, SCHEMA6_KEYS, canary+"vault", 3 legacy-blob tests updated, schema6 twin + v5-blob upgrade tests), `src/socialfi/vault.rs`+`mod.rs` (VaultTx 5 ops, execute_vault_tx, require_owner, UnknownToken/NotOwner, 4 module tests), `src/network/proto_conversions.rs` (Identity/Vault both directions, bincode thin-wrapper, roundtrip cases, corrupt-payload cases), `proto/budlum/network/protocol.proto` (IDENTITY=45/VAULT=46, oneof 52/53, ProtoIdentityTx/ProtoVaultTx), `src/rpc/api.rs`+`server.rs` (bud_identity{Resolve,Credential,VerifyPresentation}; global_header_to_json lists aiRoot+identityRoot), `src/chain/chain_actor.rs` (3 identity read commands/handlers/wrappers), `src/rpc/tests.rs` (presentation_fixture, setup_with_blockchain, 4 read-path tests), `src/tests/identity_tx_door.rs` (5) + `src/tests/vault_tx_door.rs` (5, both registered cfg(test) in `src/tests/mod.rs`). Docs: `docs/DENETIM-2026-09-10A-BUD-ZKVM-WIRING-LOOP.md` §36 (incl. reset paragraph), `docs/KIMLIK-MIMARI.md` status (both decisions closed). `repo-lubot/patches/0001..0028` queue waits. `/tmp/lw` HEAD `8d50940`.

# Actions Taken
_Terse ordered log of executed actions: tool, target, and one-line outcome. These actions already ran and their effects persist. Keep enough identity that no action is repeated by mistake._

(1–256 condensed: lubot mirror 0001-0028 delivered+fresh-clone verified (tip 37d32c9, 331 tests, baseline 29); identity arc f692ef3→f676134 pushed; §33-35 reports; Dependency Review infra verdict via check-run annotations + zero-Cargo-change proof; executor slice ce0261f [variant/tag45/encoders/gas/executor arm w/ domain-from-state decision: ConsensusEngine::domain_kind default-PoS + 5 Blockchain stamp sites; proto IDENTITY=45+bincode wrapper; Rust backslash-continuation bug caught and fixed]; identity_root 9a6c2ca [measured "7 sites" claim WRONG: 3 literal sites, serde-blob wire needs no proto; V5 fold + inventory swap + aiRoot-stale-RPC-list fix]; then ada29c8/8fff34d/738a02c/eeb9430/f67... fee87c4 — presentation RPC reads (3 commands+handlers+wrappers+fixture tests), vault body (VaultTx+require_owner, shadowed-`from` arm bug caught+rewritten, cloned-registry mint fix), schema-6 persistence (all canary/blob/upgrade tests), vault door (tag46, encode_vault_tx, transfer/burn locks, proto pair, 5 door tests; apply-helper code+message fix, step-closure borrow fix), docs §36+status.)
257. Push retries after 9a6c2ca all failed (token dead); work kept local; turn ended with blocker summary.
258. User: "bağlantı var" — retried push: git reported non-fast-forward vs base + `cat-file fee87c4` invalid + branch reset to 88970e3 with 83 dirty files = sandbox .git rolled back, files = full tip. Diagnosed before touching: `git diff` content showed pushed-history + my-5-commits = tree IS fee87c4 state.
259. Recovery: `git fetch origin +refs/heads/arena/...:refs/remotes/origin/...` (9a6c2ca restored as object) → `git reset --mixed origin/arena/01a08a1a-budlum` → residual dirty set verified == exactly the 5 local commits' footprints (0 execution_domain lines in account.rs diff, identity_tx_door.rs in tree, 17 identity_root in global_block.rs) → recreated 5 commits via original file boundaries + reconstructed original messages (888094d rpc/actor, 1fad908 vault body, 7464fbd schema-6, d1277c5 door, 3e3cd6d docs) → pushed OK → docs addendum d8c885f (reset paragraph in §36) pushed.
260. `gh pr checks 3` at d8c885f: Dependency Review fail (known infra), all else pending.

# Constraints & Corrections in Effect
_Keep this full list; do not trim for brevity. Every entry is either a user instruction or a measured correction that changes how work is done. Copy all entries verbatim into the next update._

- **Autonomy (user, standing):** "devam ben burada yokum işi durdurma" — user away, do NOT stop the work; keep shipping per-commit; work must be pushed, never left local; when push is impossible (auth), record and retry every turn.
- **Never ask the user for GitHub tokens/PAT/2FA; never echo token values; no repo-settings attempts via API. If git/gh auth fails, the only correct move is: keep working locally, retry push each turn, and tell the user to reconnect GitHub in Arena.**
- **Never force-push or switch branches on the session branch; never delete/rename/move `/home/user/budlum` or `.git`; `--amend` allowed only on unpushed commits.**
- **On sandbox-reset symptoms (branch rolled back, dirty tree = known history): DIAGNOSE before touching (ls-remote, cat-file, diff-content vs expected footprints); the files are the state — re-ground the pointer with fetch + mixed reset, recreate lost commits at original boundaries; never reset --hard a dirty tree of unknown origin.**
- **`gh pr create` and all GitHub ops via `gh` CLI ONLY — no browser-URL draft pages.**
- **Every claim about the repo is re-measured (grep/awk/python) against the applied tree; no cargo-less claims. Counting claims in docs/comments rot — §36 records another instance (7→3 constructor sites) even for this session's own memory notes.**
- **Gate name-assertions are the third masker: `grep -v` filters drop the gate's own EXPECT strings; retarget AND invert on first run when adding a gate.**
- **Turkish diacritics mandatory in user-facing Turkish text (title/body/comment); code comments English; commit messages: Turkish subject (prefix `lubot: 00XX ...` for mirror patches, `<konu>: ...` for budlum), English body.**
- **Never `git commit -m` with backticks in the message (command substitution); use a message file (-F). Apostrophes in -m also bite: heredoc-to-file + -F is the standing pattern.**
- **sed `.` matches any char — apostrophe-containing text (`Kullanıcı'nın`) needs exact string replace + re-read verification.**
- **Fresh-clone verify protocol for mirror patches: bare-cache clone (`--reference-if-able`), set user.name/email in NEW clone first, `git am` all, fmt+test+xtask gates rc=0, grep the applied tree; README claim counts recomputed FROM APPLIED TREE.**
- **When a deleted getter blocks compile, grep the BACKING field, not the accessor name.**
- **CI-log fetching: `gh run view --log` EOFs on ~350KB zips — use `--log-failed`, per-job `--log --job`, or annotations for completed jobs; a FAILED JOB inside an IN-PROGRESS run has no logs at all ("run is still in progress") — classify via check-run annotations instead.**
- **Baseline policy (dead_pub_api): real-removal + `git commit --amend` into the same patch; the gate is self-sufficient with a non-empty baseline.**
- **"Only PR #3 used for budlum; PR #4/#5 stay open as documented, no unmerge/copy-cherry" + "all changes on the single arena/01a08a1a-budlum branch" + the 12-item budlum work-queue order.**
- **The user's verbatim flow sentence is preserved in code docs (`src/registry/identity_fill.rs` header, DENETIM); don't reword those quotes.**
- **"hepsi" = the full plan: budlum work-queue AND lubot maintenance queue; the lubot shrink queue waits behind the identity arc per the user's ordering.**
- **No new deps; vendored files carry a top-source comment naming origin and license (MIT/Apache-2.0 for Rust crates, CC0-1.0 for test data).**
- **`docs/geri-donusum.md` untouched. `build: ...` commits (user's parallel session) may interleave on the branch — re-check `git status`/`git log` before committing.**
- **`--check-size` is NOT in the gate list; don't add it; don't claim CI enforces it.**
- **Patch series: one concern per patch; self-review each edit with a fresh read of the exact lines (this session caught the Rust backslash-continuation and the shadowed-`from` bug that way).**
- **Zero-panic gate forbids `#[allow]`; test code needs `BudlumError`-carrying `?`.**
- **When an unpushed commit's message/content needs fixing, `--amend` is safe; never amend pushed history.**

# Key Results
_Exact deliverables: branch names, SHAs, URLs, tables, exact numbers, and file paths the user will need. Quote precisely; copy forward verbatim._

Refs — budlum `arena/01a08a1a-budlum` tip **`d8c885f`** PUSHED (remote == local, tree clean; PR #3 head). Arc chain on remote: ...→f676134→**ce0261f** (proto+executor)→**9a6c2ca** (identity_root V5)→**888094d** (presentation RPC reads)→**1fad908** (vault body)→**7464fbd** (schema-6 persistence)→**d1277c5** (vault door)→**3e3cd6d** (§36 docs)→**d8c885f** (reset addendum). NOTE: 888094d..3e3cd6d are RECREATED commits (same content, new SHAs) after the sandbox reset killed originals ada29c8/8fff34d/738a02c/eeb9430/fee87c4. lubot `main` = `37d32c9` (28 patches; 331 tests, baseline 29, gates rc=0, fresh-clone verified). `/tmp/lw` HEAD `8d50940`.

**Dependency Review red verdict**: infrastructure — GitHub repo setting (Dependency graph) missing; needs human enable at settings/security_analysis; zero dependency change in arc proved via git log; not a code defect; do not attempt repo settings via API.

**Identity+vault wiring as landed**: TransactionType Identity(45)/Vault(46) + canonical preimage encoders + gas arms; single executor arms delegating to registry::execute_identity_tx / socialfi::execute_vault_tx, frame amount-must-be-zero rules; domain via ConsensusEngine::domain_kind() (default PoS deny, PoAEngine override) stamped into AccountState.execution_domain at 5 sites, never snapshotted; proto IDENTITY=45/VAULT=46 oneof 52/53 bincode thin-wrappers hand-mapped both directions + roundtrip + corrupt-payload cases; GlobalBlockHeader identity_root Option<Hash32> in BDLM_GLOBAL_BLOCK_V5 presence-tagged fold (bump=activation); snapshot CURRENT=6 with `>=6` vault gate + canary + 3 legacy-blob + v5-upgrade + schema6-twin tests; vault_v1/identity_v1 state-root folds empty-gated; bud_identity{Resolve,Credential,VerifyPresentation} (refusal=answer, malformed=call-error, epoch from actor); NftTransfer/NftBurn vault locks (vault_member_locked/vault_folder_not_empty); error codes also: identity_amount_must_be_zero, identity_tx_failed, vault_amount_must_be_zero, vault_tx_failed; test files identity_tx_door.rs (5) + vault_tx_door.rs (5) + vault.rs module (4) + rpc/tests.rs (4+fixture) + tamper/prop tests inline.

**Lubot state**: verified through 0028 — 331 tests, 39 gates rc=0, baseline 29. Shrink queue (kanit 4 + yetenek 5 islands; then takip 3, kuyruk 3, grant 3+1, anlama 3, singles; tools/lib+chain 2 + muhur 1 documented debt) waits next; re-measure distribution before planning each patch.

**Open reds (budlum)**: Dependency Review = infra; everything else pending at d8c885f at record time. Older-head unreadable fails (Miri UB, Cross-arch determinism, Udeps) — verdicts against current head UNKNOWN, check after drain; plus carried: badge refusal (2896 frozen vs red suite), brand gate 11, Clippy/Feature-matrix hunks in src/chain/*, actionlint absent.

# Work Queue
_Numbered, ordered. Status labels: done, landed-untested, queued, parked (with reason). Re-measure before starting queued items._

1. **(NOW) CI at `d8c885f`: poll `gh pr checks 3` until drain; classify every red against the 7 arc commits (hand-mapped proto tables + prost codegen + arms + snapshot-6 tests are what's graded); fix code-caused reds, push; ignore/record infra reds (Dependency Review). Retry push first thing EVERY turn while auth may flake.**
2. Cross-domain verifier slice (KIMLIK-MIMARI: commitment + Merkle proof against finalized identity_root in receiving domains — anchor field already landed in 9a6c2ca) — scope/recon first: cross_domain message plumbing + DomainCommitment paths not yet measured.
3. lubot shrink queue per user ordering: mirror as patches 0029+ onto 3ee81bf/271d622 lineage; re-measure distribution before each patch.
4. Carryovers: badge refusal, brand gate 11, actionlint, stale-count sweeps — classify-first each turn.

# External Sources
_What was found outside the repo that still matters — URLs for re-checking, plus the conclusion that was drawn. Older sources may be removed once their conclusions are preserved in code or docs._

- GitHub Actions support threads + current docs consensus + measured behavior (id=244): Dependency Review action fails the step when the repo has no dependency graph (`settings/security_analysis` enable is human-only; check-run annotations are the readable channel: `gh api repos/.../check-runs/<jobid>/annotations`); job-id == check-run-id for workflow jobs; `--log-failed` unavailable while parent run in progress.

# Downgrade Log
_Anything removed from this memory in this update: what was dropped, and why it was safe. If nothing was downgraded, write exactly: None._

- "5 commits queued for push / push when auth returns" state — superseded: all recovered+pushed at d8c885f; recovery method preserved in Actions 259 + a new constraint entry.
- Prior turn's blocker-report details trimmed to their durable lessons (retry-first, small-push habit); commit lists preserved in Key Results chain.
- Actions 1–256 remain condensed; full method details live in Key Results + repo docs (§33–§36).
