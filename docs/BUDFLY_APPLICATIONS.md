# Sineğin tam kullanım haritası — BudFly'ı "komple" nereye koyarız?

> Kural: her satır ya **SEVK** (CI'da donmuş pinle çalışıyor), **ÖNERİ**
> (backlog, akıl yürütmesi değişmedi), **DENEY** (sınırı bilinçli zorlanıyor)
> ya da **RED** (nedenleriyle gömüldü). RED satırı silinmez — haritanın
> omurgası budur.

## A. Kimlik / güven / peer yüzeyi

| # | Yüzey | Sinek ne kanıtlar? | Maliyet | Yenilik | Durum |
|---|---|---|---|---|---|
| A1 | **Proof-of-exact-build handshake** (`attest.rs`) | "Bu peer, bu donmuş konektomu bu donmuş kod yolundan koşturuyor" — (peer, epoch, nonce) bağlı challenge; L1/L2/L3 merdiveni | L1: 1 tick ≈ 32 B | Kapalı validator setinde davranışsal build-parmakizi; TEE değil, kripto değil — MAP ITSELF the asset | **SEVK** |
| A2 | Cüzdan imza-FSM nabzı (wallet-core) | Kritik state geçişlerinin digest'ini epoch başı canary'e kat: "imzalama akışı pinlenmiş FSM'di" | 1 tick/epoch | Donanım-cüzdan attestation kültürünün Rust'a sıfır-bağımlılık taşınması | ÖNERİ |
| A3 | **Zaman-basan VDF-benzeri beacon** | yok | — | Dinamikler ucuz-tekrarlıdır: "yavaşlık" iddiası yalan olur | **RED** — time-hardness yok |
| A4 | Açık-dünya kimliği | yok | — | Map'i elinde tutan replay eder | **RED** — kapalı set sınırı |

## B. Zincir / mutabakat yüzeyi

| # | Yüzey | Sinek ne kanıtlar? | Maliyet | Yenilik | Durum |
|---|---|---|---|---|---|
| B1 | SENTINEL (RoleId 10, oy-hakkısız validator) | olgu transkripti mührü (anchor), jury quorum, 136 B/204 B zarf | ~2,3 µJ/verdict, zarf C1 | Donmuş spike dinamikleriyle "oy kullanmayan karar mekanizması" | SEVK (sentinel+divan+BSE-1/2) |
| B2 | Fraud-proof tahkimi (ACT-1 bant) | ihtilaflı tek tick'in delili, connectome'suz: kısıt katili + fold katili | 24,7 KB/tick | STARK'sız çalışan kanıt bandı; zk bunu 32 B'a bastırır | SEVK |
| B3 | Lezyon-bataryası → saha arızası dili | çekirdek arızası `run_masked` imzası (no_eb/no_mdn/no_apl) | anchor ×3 + sayaç | Makina-arızası = davranış imzası; probing v2 bekler | SEVK |
| B4 | Donanım turnuvası + sezon ligi | 4-tohumlu sezon sıralaması, tie-break ayak-bağlamalı | panel ×N council | Sıralama tablosu hakem-imzalı; çaylak şampiyon | **SEVK** (turnuva + lig) |

## C. Yazılım süreçleri yüzeyi

| # | Yüzey | Sinek ne kanıtlar? | Maliyet | Yenilik | Durum |
|---|---|---|---|---|---|
| C1 | CI reprodusibilite kanıtı (determinism.yml'e giden köprü) | build X koşunca bu anchor; cross-dil Python≡Rust ≡CI | golden tablosu 100 pin | Tek tablo, üç kez, her PR'da | SEVK (golden disiplini) |
| C2 | Donanım kısmı/PL raporu | 3-çekirdek tile yerleşimi + fault tablosu + %33 kapasite-tamponu | python pinli | Lezyon→silikon fault haritası | SEVK (fabric+fault) |
| C3 | DRAM/kozmik-ışın forensics | wander histogram sapması = hata lokalizasyonu | pinden okuma (bedava) | "Kazanamayan attractor" görevde | SEVK |

## D. DENEY kulvarı (sınırı zorluyoruz)

| # | Yüzey | Teklif | Sınır | Durum |
|---|---|---|---|---|
| D1 | BudZero zk aritmetizasyon | C1–C7 = circuit'in çalışabilir spec'i; ikili ACT-1 test vektörü kanonik (`budfly-fixtures-v1`) | bud-zero pipeline ayrı proje; vektör burada prover orada | **DENEY** — spec + fixture SEVK |
| D2 | Depolama erasure prover | root-bağlı chunk challenge -> K×L1 cevap; avalans pinli | posesyon DEĞİLDİR: pin geri alınamaz | **DENEY** — protokol + sınır birlikte pinlendi |
| D3 | Lubot fact-finality anchor'ı | c1 payload hash → sentinel digest; validator RoleId 10 | Lubot runtime bu repoda yok — prompt+patch serisi | ÖNERİ (doc: BUDFLY_SENTINEL_VALIDATOR) |

## Komuta zinciri — sinek her yerde aynı şeyi söyler

```
     challenge digesti (olgu / peer / epoch / nonce / blob root)
        │  canary (1 tick)  ← nabız
        ▼  sentinel (48 tick) ← transkript
   divan (3 seat)  ← quorum, oybirliği-kapan
        ▲  dispute → tape (ACT-1) ← tahkim, connectome'suz
        │  replay (prefix-bound) ← kontrgerçek
        ▼  envelope BSE-1/BSE-2 ← 136/204 baytlık karar
```

Tek ilke: **ucuz katman pahalı sözleşmeye birebir oturur** — canary tick1,
48-tick zincirin ilk başıdır; bant fold'u yayınlanmış baştır; jüri
anchor'ları zarfın içindedir. Bir katmanı yalan söylerse üst katman
düşer.

## Nerede kullanılmaz (haritanın kuzeyi)
- Açık dünya kimliği / anti-Sybil (A4).
- Zaman-zorluğu isteyen protokoller (A3).
- "Sinek anladı" anlatılarının hiçbiri (kültür kuralı: anchor taşınır, hikmet değil).
