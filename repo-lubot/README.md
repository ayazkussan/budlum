# repo-lubot - the mirrored patch series

Code: `ayazkussan/lubot`, base `main` @ `37d32c9`. The bot cannot push to that
fork (403), so every commit ships here as a `git format-patch` file; distribution
is the user's.

## Applying

```
git -C <lubot-clone> fetch origin main
git -C <lubot-clone> checkout -b lubot-series origin/main
git -C <lubot-clone> am -3 --whitespace=fix patches/*.patch
```

Verified, not asserted: a clean clone of `main` @ `37d32c9` takes all 23 files
with strict `git am -3` - no `--reject`, no fallback - and the resulting tree
has `Cargo.toml` members equal to the 20 directories under `crates/`, and
`docs/CRATES.md` with one row per crate the series adds (12).

## Why this directory was rewritten

The mirror used to document its base as the branch `olcum-disiplini` @ `12bc9ac`.
That branch is gone: `main` is the fork's only ref, one commit, a different tree (it
carries `crates/doc`, `crates/sikistir`, `gates/check.py`,
`training/curriculum/ajan.jsonl`). Against the one base that exists, the old files did
not apply - patch 0002's `Cargo.toml` and `ajan.jsonl` hunks carried context from the
deleted branch, so `git am -3` rejected there and everything after it was unreachable.
A series nobody can apply is a bundle of text files, so the 18 commits were replayed
onto today's main and re-exported.

Where context could not match, the repair was by *class* and it is stated rather than
buried: 0002's training rows are appended (JSONL order is not semantic) and its
workspace-members line is inserted by regenerating the list from the crate directories
that exist. The first attempt at this used `patch -N`, which skips hunks it believes are
already applied and still exits 0; it reported 18/18 clean while the members list never
grew and ten crate manifests were never written. The structural self-check - members ==
crate dirs, every crate has `Cargo.toml` and `src/lib.rs`, no `.rej` anywhere - is what
caught a successful-looking lie, and it runs before anything is exported.

## What was run here

No Rust toolchain exists in this sandbox, so nothing was compiled. What does
exist is python3, and lubot's gate suite is std-only Python, so it was run on
the applied tree: `gates/check.py --list` reports 38 gates (matching the
README's claim), and `--all` passes the first 5 (`reads-not-generates`,
`no-fourth-channel`, `provenance-fails-closed`, `mask-before-storage`,
`no-panic-path`) - after which the runner aborts with `FileNotFoundError:
'cargo'` inside `gate_readme_is_measured`, because that loop catches
`SystemExit` only. That crash is pre-existing in the base repo, not a product
of this series; it is reported rather than patched here, since a gate runner
that fails *softly* when its tool is missing is a decision for the repo owner,
not for a patch bundle. It is also the reason the ratchet number cannot be
verified with cargo: it can only be derived.

## What is NOT verified

Nothing was compiled. This sandbox has no toolchain: rustup, crates.io, the docker
registry and GitHub release assets are all unreachable from it. So
`cargo test --workspace`, `clippy -D warnings` and the rustfmt state of these crates are
unmeasured. The test-count line is the one claim that was rebuilt rather than
carried: patch 0019 sets it to 327, which is `#[test]` attributes in
`crates/**/*.rs` - 178 in the eight base crates plus 149 in the twelve this
series adds, with 0 `#[ignore]` and no doc examples, so cargo has nothing else
to count. Patch 0022 moved it to 325 - the two `tools/chain` ceilings tests left with
the callerless flow they covered - and patch 0023 to 328, the three new
`cli/activation` tests arriving with the wiring. Both moves were recounts of the
applied tree (176 base + 152 series after 0023), never decrements by hand. The base README said 191 while the base tree measures 178, a
13-test overstatement that predates this series; 0019 replaces it with a count
derived from the applied tree instead of inflating the old number. `38 gates`
was checked by counting `def gate_`; `0 pedantic` and `793 corpus records` were
left alone - the first needs clippy, the second is the gate's own corpus
definition, and `training/curriculum/*.jsonl` holds 83 rows there, so those two
are not the same quantity.

Patch 0020 is the other content fix found by checking structure rather than by
hoping: the series' own `docs/CRATES.md` listed ten crates while `crates/`
gained twelve, because the first two (izolasyon, denetim) were never given a
row. Its acceptance test is `rows == crate dirs for the crates the series adds`
(12 == 12), and it changes no code, so it needs no compilation.

## Patch 0021: the gate that makes "is anything wired?" a measured number

`gates/public-api-is-reached` counts production `pub fn` declarations in
`crates/**/src/**/*.rs` against an identifier census of the whole tree (test tails
cut, the declaring file's own declaration lines subtracted), with a
`/// Convenience:` / `WIRING:` / `exposed for` exemption within 14 lines above.
It is a ratchet: `gates/dead-pub-api.baseline` holds the unreached entries measured
on the applied tree - 53 when 0021 shipped, 51 after 0022's deletion, 41 after 0023's
wiring - growth fails, and a
baseline line that stopped being dead fails too, so tightening is the only direction
that moves; 0022 is that rule tripping for real: the deletions made two baseline
lines stale, and the patch that deleted the functions had to delete their entries.

Unlike the compiled parts of this series, the gate was executed rather than
inspected, because it is std-only Python and python3 exists here: `--self-test`
OK, `public-api-is-reached` OK at 53/53, 51/51 after 0022 and 41/41 after 0023, and
both failure directions reproduced
on a scratch copy - a freshly injected unreached `pub fn` failed with
"1 public function(s) nothing in the tree calls", and deleting a live baseline
line failed with "1 baseline entry no longer dead". A missing or unsorted baseline
fails with its own message, since a gate that cannot read its baseline is a gate
that always passes. `--list` reports 39 gates and README plus
`training/ratchet.json` moved to 39 with it, so the ratchet describes the tree.

## How this directory is regenerated

`tools/rebuild_series.py` is the tool that produced patches 0001-0018: it replays
the recorded commit headers (subject, author, real `Date:` header) onto a base
checkout, repairs unmatchable hunks by class with a printed note, and refuses to
export anything until a structural self-check over the *result* passes. Patches
0019 and 0020 are not produced by it - they are authored on top of the replayed
tree, which is why the tool reproduces 18 commits and the directory ships 20.
Smoke-tested from a clean clone of the fork: same self-check line, `members=20
crate_dirs=20 my_crates=12 lines=10581 tests=149`.

The table below is not hand-maintained: `python3 tools/rebuild_series.py --table`
prints it from the patch files themselves, and `SERIES_DIR` points it at another
directory. It was verified by diffing that output against this file - byte for byte -
because a table assembled by a throwaway script on one machine is how a documented
number and the tree it describes first part company.

## Contents

| patch | commit subject | diffstat |
|---|---|---|
| `0001` | lubot yetenek: izolasyon crate (arcbox -> izolasyon siniri) | 3 files, +216 |
| `0002` | lubot yetenek: denetim crate (scan -> validate -> fix, kanitli defter) | 4 files, +531 |
| `0003` | lubot README+ratchet: izolasyon+denetim ölçülür (191 test, 0 pedantic) | 2 files, +6 |
| `0004` | lubot denetim: rustfmt hizali test cagrisi | 1 files, +7 |
| `0005` | lubot kapi: denetim crate'in ledger kurallari (38 kapi, canary'li) | 3 files, +47 |
| `0006` | yetenek: kart bicimi, kanitla terfi, rotaya gore cagri | 2 files, +794 |
| `0007` | olcek: sert tavan, yumusak su hatti, ayrilmis taban, sessiz dusurme yasagi | 2 files, +727 |
| `0008` | kanit: kapsama kilidi, sonra iddia, sonra yol; ucusuz kapanis yok | 2 files, +895 |
| `0009` | mimari: modul tablosu koda uysun, kod tabloya degil | 2 files, +600 |
| `0010` | workspace: dort crate members listesine, envanter dosyasi | 2 files, +16 |
| `0011` | takip: baglantilik iddiasi bir veridir, yorum degil | 2 files, +727 |
| `0012` | muhur: sonradan duzeltilen kayit iz birakmali | 2 files, +643 |
| `0013` | workspace: takip ve muhur members listesine, envanter iki satir | 2 files, +4 |
| `0014` | kuyruk: is kuyrugu, ama yok edilen isin kayitiyla | 2 files, +720 |
| `0015` | erisim: yetki belgesi daralabilir, buyüyemez | 2 files, +974 |
| `0016` | workspace: kuyruk ve erisim members listesine, envanter iki satir daha | 2 files, +4 |
| `0017` | anlama: komutu oku, emri vermeden önce | 4 files, +2462 |
| `0018` | esik: davranis degisikliklerinin aktivasyon cagisi bir veridir | 4 files, +1445 |
| `0019` | lubot README+ratchet: 327 test, uygulanmis agactan sayildi | 1 files, +1 |
| `0020` | envanter: serinin 12 crate'i var, tablo 10 sayiyordu | 1 files, +4 |
| `0021` | lubot kapi: ulasilmayan pub fn ratchet'i (39 kapi, canary'li) | 4 files, +197 |
