# Current State
_What is being worked on right now, what is unfinished, and the immediate next steps. Always update this section. If the outcome of the most recent action is unknown, say so explicitly._

**2026-09-12 ~05:1x UTC, head `d24df3e` (pushed, ls-remote verified).** Fourth .git rollback recovered same turn (files survived; queue empty since prior commit was already pushed; `reset --mixed FETCH_HEAD` -> clean tree, zero loss). CI verdicts at 26ebe61 read: 14 reds, two real classes — FIXED+PUSHED here: (1) **doctest-indenter trap**: doc blocks indented 4+ spaces after a blank `///` compile as RUST doctests; 10 blocks in 8 files fenced ```text (2 CI-confirmed: blockchain.rs unwrap_or_else, bridge.rs fee table; 8 preventive via tree sweep); these cascaded ~12 jobs. (2) latent test bugs unmasked once suites compiled (81e0f8d's job done): stake-arm assert looked for code token in message() (code lives in err.code(); sibling uses swept, prose needles pass, untouched); snapshot test hardcoded "max supported is 4" after my CURRENT=6 bump (now interpolates the constant); rpc upscale-DID test was tautological (fixture hex 0101.. has no letters -> uppercase no-op resolved Ok) -> lettered probe + assert_ne canary. (3) Domain Tag Inventory red was MINE: BDLM_VAULT_V1 used-unlisted (V5 swap covered only global-block); added alphabetically; gate rule re-implemented in python, both diffs clean. STILL OPEN: storage_econ accept_reallocation x4 (shared helper open_never_placed_ticket expect-fails; root cause unread - read new-run annotations first), a_field_discloses test (not-7 verifier gap, parked, user decision), Gates job (indexing-is-not-new 3 files; no-upstream-brands 11; actionlint missing on runner=infra), Budlum Core "131 fmt hunks" (rustfmt-version drift suspected - same sources were green at 53073db; do NOT hand-reformat; compare main CI first). Dep-Review/Typos = documented non-branch. NEXT: fresh verdicts at d24df3e; storage_econ; fmt-vs-main; lubot queue (compact decision; re-clone /tmp/lw with 31-patch am).
- User asked why ~6h idle Sep-11 21:27 -> Sep-12 03:35: turn model = work happens inside turns; the turn ENDED at context ceiling with 3 retries failing on flapping auth; no user prompt arrived to open a new turn; sandbox reset during the gap cost further objects. Lesson recorded: end turns LESS gracefully - keep the loop of (work, push retry) until the hard ceiling, and treat every turn-open as push-first.
- **budlum arc**: all identity+vault wiring landed through `53073db`-era commits (head chain: ce0261f proto+executor, 9a6c2ca identity_root V5, 888094d RPC reads, 1fad908 vault body, 7464fbd schema-6, d1277c5 vault door, 3e3cd6d+e4e665f docs, 81e0f8d nightly fixes, 53073db+new mirror). KIMLIK-MIMARI not-7 records the verifier design gap (root() = sequential accumulator -> per-subject proof impossible; decision: pre-launch Merkle re-root OR honest deferral; NOT yet greenlit as code).
- **lubot mirror: 31 patches delivered** (0029 dead-pub wave1 floor 29->25, 0030 wave2 ->21 with two live-surface test rewrites + read-back saving 4 orphaned attrs/empty impl, 0031 chain-surface red CLOSED parse_get_ceilings 8/8 green, 332 tests). Fresh-clone 31/31 am verified on lw4 at 21:2x. Remaining 21 baseline entries are declared KEEPS (sole carriers). Open queue items: index `compact` decision (README row promises it - wire or restate), `muhur 1` documented debt (re-measure), any remaining singles. /tmp/lw was wiped by the reset - re-clone ayazkussan/lubot + `git am` the 31 patches to resume work tree (HEAD aee252f+3 commits -> tip 88ccd7f equivalent).
- Standing: user away, "işi durdurma"; push-first each turn; small commits; never fake intermediate history for unpushed losses (say so); CI-log channel = check-run annotations (blob logs EOF through this proxy); Dependency-Review/Typos reds = classified, don't re-triage.

# Task
_What the user asked for, in their terms. Preserve active acceptance criteria and consequential scope decisions. Remove obsolete narrative when necessary, but do not lose requirements that still affect the work._

_unchanged_

# User Constraints & Corrections
_Explicit standing instructions and corrections the user stated about how the work should be done, including anything the user rejected. Only what the user explicitly directed — never infer. Never drop an entry unless the user reversed it or it applied only to a task that has finished._

- "devam ben burada yokum işi durdurma" — continue while user away, do not stop the work.
- "harika ama durmasan da olurdu bağlantı var" — do not idle-stop on transient blockers; connection was there, retry.
- (2026-09-12) "peki neden saatlerce olmamama rağmen durdun" — the user expects the loop to keep running across their absence; the honest mechanism is turns: keep each turn maximally productive (work+push-retries until ceiling), never end early while actionable work remains, and push FIRST thing on every turn-open.

# Workspace
_Files and directories that matter: path, plus one line on what each contains and why it is relevant. Never include file contents; the workspace itself is the source of truth._

`/home/user/budlum` — branch `arena/01a08a1a-budlum`, remote tip `53073db`, local recovering to it + one combined mirror commit. Identity+vault arc files as before (see Key Results chain), docs: DENETIM §36 (+reset paragraphs), KIMLIK-MIMARI not-7. `repo-lubot/patches/*.patch` = 31-patch mirror series (source of truth for lubot work; /tmp workdirs are disposable). lubot repo = `ayazkussan/lubot` main `37d32c9` (single-commit root; bot 403-push blocked by design - patch delivery IS the channel). CI verdict channels: `gh pr checks 3`; failed-job annotations via `gh api repos/.../check-runs/<jobid>/annotations`.

# Actions Taken
_Terse ordered log of executed actions: tool, target, and one-line outcome. These actions already ran and their effects persist. Keep enough identity that no action is repeated by mistake._

(Condensed: lubot mirror 0001-0028 delivered+verified; budlum identity arc f692ef3->9a6c2ca pushed with per-slice tests; two sandbox resets recovered by files+footprint-recreate (zero content loss); nightly test-compile fixes 81e0f8d (5 errors: shadowed helper, 2 same-call moves, 2 matches!-partial-moves; read-back also caught my own 2 bugs pre-push); Typos zeroed c565e3c (3 real typos, quote-describe rule, `ser` calibrated; residual red = main's 2.3MB duplicate, §33 verdict); KIMLIK-MIMARI not-7 (root() accumulator measured; verifier needs Merkle re-root or honest deferral).)
NEW THIS STRETCH: lubot 0029 (kanit claim_of/may_act/described_only+prose_only orphan caught by pairing rule, yetenek with_evidence; 25/25), 0030 (grant_by_id, is_fix->matches! rewrite, is_known->parse(), found_in_data->attachments()+mentions(); read-back caught 4 orphaned doc/attr remnants incl. empty impl - compilerless save), 0031 chain client parse_get_ceilings (zero/missing/inverted refusals, WIRING: exemption at declaration, 8/8 gate green, 332 tests, README recounted, fresh-clone lw4 31/31 am verified). Third reset recovery in progress (files->recreate combined commit).

# Constraints & Corrections in Effect
_Keep this full list; do not trim for brevity. Every entry is either a user instruction or a measured correction that changes how work is done. Copy all entries verbatim into the next update._

- **Autonomy (user, standing):** "devam ben burada yokum işi durdurma" — keep shipping per-commit; work must be pushed; when push is impossible (auth), record and retry EVERY turn-start.
- **Turn model honesty:** maximize each turn to its ceiling with (work -> push-retry) loops; end only when truly blocked; on every turn-open: push first, then new work.
- **Never ask the user for GitHub tokens/PAT/2FA; never echo token values; no repo-settings attempts via API. If git/gh auth fails: keep working locally, retry push, tell the user to reconnect GitHub in Arena.**
- **Never force-push or switch branches on the session branch; never delete/rename/move `/home/user/budlum` or `.git`; `--amend` only on unpushed commits.**
- **On sandbox-reset symptoms (branch at base, dirty tree = known history, objects missing): DIAGNOSE first (ls-remote, cat-file, diff-content vs expected footprints); files are the state; re-ground pointer with fetch+mixed reset; recreate lost commits at original boundaries when meaningful, or one honest combined commit when never-pushed; never reset --hard a dirty tree.**
- **`gh pr create` and GitHub ops via `gh` CLI ONLY — no browser draft URLs.**
- **Every claim re-measured against the applied tree (grep/awk/python); no cargo-less claims; count-rot applies to this file too — re-derive patch/test/floor counts from the tree, not from here.**
- **Gate name-assertions are the third masker: retarget AND invert when adding gates; after renames/deletions grep gates/README/docs for the dead names.**
- **Turkish diacritics mandatory in user-facing Turkish; code comments English; commit style: Turkish subject (`lubot: 00XX ...` for mirror patches), English body; never -m with backticks (use -F file).**
- **sed `.` matches any char — apostrophe text needs exact python replace + read-back.**
- **Fresh-clone verify protocol for lubot patches: clone base, config user, `git am -3 --whitespace=fix` all, run cargo-free gates (python3 gates/check.py individually; --all crashes on cargo), grep applied tree; README counts recounted from applied tree.**
- **No-compiler discipline (lubot sandbox has NO cargo, measured): deletions only with zero-ref proof across whole tree incl. gates/docs; after every deletion read each deleted region's CONTEXT (orphaned doc-comments/attrs/empty impls are the failure class the compiler would have caught - brace-scan + human read replace it); test-only pub surfaces get WIRING:/Convenience:/exposed-for exemption at the declaration, never a new baseline line.**
- **dead-pub baseline policy: real removals delete their baseline lines in the same patch; floor tightening is the only legal direction (gate fails growth AND stale lines).**
- **budlum: PR #3 only; PR #4/#5 stay open as documented; all changes on `arena/01a08a1a-budlum`.**
- **The user's verbatim flow sentence stays in code docs verbatim (identity_fill.rs header, DENETIM).**
- **"hepsi" = budlum work-queue AND lubot maintenance queue.**
- **No new deps; vendored files carry origin+license header (MIT/Apache-2.0 crates, CC0-1.0 test data).**
- **docs/geri-donusum.md untouched; user's parallel `build:` commits may interleave on the branch — re-check status/log before committing.**
- **`--check-size` NOT in gate list; don't add, don't claim.**
- **Patch series: one concern per patch; fresh eyes read of every edit region.**
- **budlum Zero-panic gate forbids #[allow]; test code may unwrap via expect on Result with BudlumError carried in format.**
- **Amend only while unpushed; never push --force.**

# Key Results
_Exact results that must remain available: answers, tables, short code, decisions, or paths to generated files. Include short deliverables verbatim and reference longer artifacts by workspace path._

Refs — budlum remote head **`53073db`** (PR #3 head, https://github.com/ayazkussan/budlum); local adds one combined mirror commit (0030+0031 iz + memory) this turn. lubot remote main `37d32c9` + 31 patches in `repo-lubot/patches/` (0029 `dead-pub wave 1`, 0030 `wave 2`, 0031 `chain okuyucusu`); full-series fresh-clone verified (lw4): am 31/31 rc=0, 332 tests, floor 21/21, chain-surface 8/8 GREEN, 24 cargo-free gates green / 14 skipped(no cargo) / 0 failed.

**Idle measurement (user question):** remote last commit Sep-11 21:27:29 UTC -> now Sep-12 03:35 UTC = **6h08m GitHub-visible silence**; last turn's 3 pushes all failed on the flapping channel immediately before the turn ended; sandbox reset then rolled local objects back. Mechanism stated honestly in memory + answer: turns, not a daemon.

**Dependency Review red**: infrastructure (dependency-graph setting, human-only; zero Cargo dep lines in whole arc). **Typos red**: main's `01a08674-...(2).patch` 2.3MB duplicate - needs `git rm` on main by the user; branch contribution zeroed in `c565e3c`.

**Verifier decision parked (not-7):** root() is a sequential accumulator -> inclusion proofs impossible; options measured: (1) pre-launch Merkle re-root on family's merkle_root/disclosure_proof, two witnesses (membership + revocation-exclusion), identity_v1 fold moves same slice, twin-node test rerun; (2) bounded replay refused (no size bound); (3) current L1-RPC-read stays as honest stopgap. Awaiting CI-green before opening.

Lubot floor: 21 kept entries are reasoned carriers (open_path, kuyruk submit/peek_due/evictions, takip declare_*/unclaimed, codec pair, with_* setters, record_of, is_finalized, compact, izolasyon with_entries, anlama with_attachment, yetenek 4) — compact has its own README-row decision queued.

# Work Queue
_Numbered, ordered. Status labels: done, landed-untested, queued, parked (with reason). Re-measure before starting queued items._

1. **(NOW) Push the recovery commit; verify remote == local at new tip.**
2. **CI verdicts at head**: Miri/ASan on 81e0f8d fixes -> must be green; any NEW red from arc commits classified (annotations channel), code-caused fixed+pushed; Dependency-Review/Typos remain as documented.
3. lubot: re-clone work tree (am 31 patches), then `compact` decision: wire into answer's reading loop (find the slot where index results compact before prompting) OR restate the index README row; re-measure muhur/olcek singles for any pure-wrapper leftovers; export 0032+.
4. budlum cross-domain verifier slice: greenlit ONLY after CI green on current head + user-noted decision (Merkle re-root is a root-format change: land as its own reviewed slice, never as drive-by).

# External Sources
_Web pages fetched and concrete findings that still shape the work. The fetch results are not saved in the workspace, so anything that still matters must be preserved here. Older sources may be removed once their relevant conclusions have been preserved elsewhere._

- GitHub Actions support threads + measured behavior: Dependency Review fails when repo lacks dependency graph (settings/security_analysis, human toggle; check-run annotations readable via `gh api repos/.../check-runs/<jobid>/annotations`; job-id == check-run-id); `--log-failed` blocked while parent run in progress; blob-host logs EOF through this sandbox proxy (retry loop or annotations channel).

# Downgrade Log
_Anything removed from this memory in this update: what was dropped, and why it was safe. If nothing was downgraded, write exactly: None._

- Third-reset lost-commit details (21dbeb5/a18100d/e2072bd content) folded into "one combined commit" note - content preserved verbatim in files + Actions; history shape intentionally not faked for never-pushed commits (recorded as a constraint).
- Prior queue wording for waves 0029-0031 superseded by delivery; criteria preserved in Constraints (no-compiler discipline).
