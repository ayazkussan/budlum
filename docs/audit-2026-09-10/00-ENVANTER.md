# 00 — Envanter (2026-09-10)

Kapsam: `ayazkussan/budlum`, dal `arena/01a089e3-budlum`, HEAD `88970e3`
(`budlum: workspace-mirror 0033 (09N 5n erasure)`; önceki dal
`arena/01a08674`'ün ucu). Checkout sığdır (tek commit); önceki dalga
fix SHA'ları (`e4f472b`, `e0c1ca3`, `fe5461e`, `a8b45e1`, `8ba7eab`)
ağaçta yok — önceki bulguların varlığı dosya içeriğiyle doğrulanır.

## Sayılar

| ölçüm | değer |
|---|---|
| `.rs` dosyası (`src`+`crates`+`bud`+`budzero`) | 516 |
| `.rs` satırı (aynı kapsam) | 267.764 |
| `src/` modülü (`lib.rs`) | 32 |
| `crates/` | 4 (`ai-inference`, `budscan`, `note-packing`, `wallet-core`) |
| `bud/` | bağımsız crate + fuzz/kani |
| `budzero/` | 8 crate (`bud-cli`, `bud-compiler`, `bud-isa`, `bud-node`, `bud-proof`, `bud-state`, `bud-vm`, `verifier-registry`) |
| `xtask/` | `gates`, `tools` |
| CI workflow (`.github/workflows`) | 23 |

Boyutlar: `src` 7.8M, `crates` 1017K, `bud` 1.8M, `budzero` 1.8M,
`proto` 20K, `config` 52K, `docs` 312K, `ops` 96K.

## Modül README kapsamı (charter: README yoksa modül deneysel)

HAS (6/32): `ai`, `bns`, `budlumxyz`, `pollen`, `socialfi`, `storage`.

MISSING (26/32): `account_abstraction`, `ai_inference`, `bin`, `chain`,
`cli`, `consensus`, `core`, `cross_domain`, `crypto`, `deed`, `domain`,
`execution`, `gateway`, `light_client`, `mempool`, `network`, `privacy`,
`prover`, `registry`, `relayer`, `rpc`, `sdk`, `settlement`, `sharding`,
`tests`, `tokenomics`.

## Mühendislik kapıları (kodda)

- `src/lib.rs`: `#![forbid(unsafe_code)]` (unsafe girerse build FAIL).
- `unwrap`/`expect` üretim kodunda yasak (test muaf).
- CI bekçileri: `.github/unwired-guards-baseline.txt` (`3`, yalnızca
  düşebilir), `.github/idle-code-baseline.txt`; kullanan: `ci.yml`.

## Toolchain (kritik kısıt)

Sandbox'ta `cargo`/`rustc`/`rustup`/`nix` yok (`PATH`'te bulunamadı).
`rust-toolchain.toml` `1.97.1` pinliyor. Yerel `build`/`test`/`clippy`
çalıştırılamaz; doğrulama yolu: (1) bu dalın PR'ında koşacak fork CI,
(2) stdlib-only Python ölçüm (`tools/audit/wiring_check.py`).

## Harici erişim özeti

| repo | durum |
|---|---|
| `budlum-xyz/budlum` | erişilebilir; PR50 (`usl`) OPEN |
| `budlum-xyz/lubot` | erişilebilir |
| `budlum-xyz/deneme` | erişilebilir; 490+ OPEN PR (parça uyarlamaları) |
| `budlum-xyz/seed` | erişilebilir; 8 PR (tamamı dependabot) |
| `budlum-xyz/workspace` | YOK (çözümlenemiyor) |
| `ayazkussan/budlum` (fork) | bu checkout'un origin'i |
| `ayazkussan/workspace` | okuma kaynağı (116 kök girdisi, `skills/` 16, `memory/hafiza.md` 6797 satır) |
| `ayazkussan/lubot` (fork) | `main` tek dal; `olcum-disiplini` silinmiş |
