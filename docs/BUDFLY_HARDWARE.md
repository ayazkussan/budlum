# BudFly donanım / maliyet / güvenlik defteri

> Her sayı kaynaklı: pinli veri `crates/budfly/goldens.anchor.toml`
> (`[expansion]`, `[fabric]`), hesaplanan her büyüklük "analitik" etiketli.
> Süsleme yok; dürüst-negatifler silinmedi.

## Neden bu PR güvenliği artırıp maliyeti düşürür (özet)

| İddia | Mekanizma | Maliyet |
|---|---|---|
| Fabric "hâlâ dürüst hesaplıyor mu" denetimi | **1-tick canary** (challenge-response, digest'ten türeyen stimulus) | 1 fold ≈ **32 B kanıt / 1 tick** |
| Yalan söyleyen yürütücüyü yakala | **anchor-bisection itiraz oyunu** | en çok **6 anchor takası** (48 tick) + 1 tick tahkim |
| Lezyon testleri → saha arızası dili | `run_masked` bataryası | anchor ×3 + sayaç vektörü |
| Soft-error post-mortem | pinned t=40 wander histogramı | ücretsiz (pinden okuma) |
| Sentinel ₺'si | 3 çekirdekli N1 tile yerleşimi; 48,529 pJ/tick | **≈ 2,3 µJ / 48-tick verdict** |

---

## 1. Canary anchor (firmware kalp atışı)

**Problem:** gömülü nöromorfik donanımın içsel olarak kendi dürüstlüğünü
kısa ve ucuz kanıtlaması zordur. **Çözüm:** sineğin donmuş dinamikleri zaten
deterministik bir referans — 1 tick'lik bir challenge-response zincir
yeter.

- Girdi: 32 B `digest` (zincirden gelen challenge) → `sentinel_stimulus`.
- Çıktı: 1 tick katlanmış `anchor = log[0]` (`canary.rs::challenge_canary`).
- Pin: all-ones digest için `6c221c70…`; **uç nokta otomatik doğrulanır**:
  aynı log yolu 48 tickte sentinel golden `398e126a…`'a oturuyor
  (`canary.self_consistent = True`) → ucuz uç, pahalı sözleşmeye içeriden
  bağlı.

**Kanıt maliyet tablosu (analitik):**
- 1 tick work ≈ 588 LIF update (~15 int-op) + 2.283 synaptic add + 1 SHA-256
  fold (~4 KiB girdi) — mertebesi ~25–35 bin int-op/tick.
- Tek-çekirdekli 100 MHz RISC-V kabaca: **~0,3 ms / canary** (analitik, 1 IPC).
- N1 tile (3 çekirdek, yarım-parallel delivery): 48,529 pJ/tick
  (python pinli `[fabric] energy_per_tick_pJ`) → per-canary üst sınır.

## 2. İtiraz oyunu → gas ucuz fraud-proof

- Divergence: tam tick (pin: 23).
- Envelope: `divergence_tick:u32 + honest_anchor:32B + claimed_anchor:32B
  = 72 B` (bayt bütçesi, zincir üstü ücret modeli host'a göre — kapsam dışı
  etiketli).
- Arbitration: C1–C7 kısıt denetimcisi tek tick'te karar verir;
  `air::count_violations` dürüst trace'te 0, sahtede >0 (pinli test).
- Bisect exchange üst sınırı: `ceil(log2(48)) = 6` (48 ticklik chain için;
  formül `bisect_queries_max`).

## 3. Lezyon bataryası = fault-injection laboratuvarı

Aynı stimulus, üç ablasyon maskesi, pinli işaretler:
- **no_eb**: CX zinciri kaskadı (compass kaybı davranışsal olarak görünür).
- **no_mdn**: veto sönüyor (sentinel verdict'i Abstain'den kayar).
- **no_apl**: prob v1'de sıfır-delta — **duyarsızlık pinlendi** (prob
  kasıtlı kaba; probing v2 odağı: anaerobik PN sürüşü).

Sahada çekirdek arızası: `fabric::fault_row` makina-mesajı üretir;
**%33 kayba kadar throughput sabit** (3 çevrim/tick) — yerleşim boşluğu
kendi kendine yedekleme.

## 4. Wander histogramı = DRAM/soft-error forensics (negatifin enstrümanı)

Kazanamayan attractor'ımız bir işe yarıyor: canlı DS-integrity kanalı.
Pinli t=40 histogram `0:2,1:2,3:3,4:1,5:2,11:1,12:2,13:1,14:3`.

Denetim katmanı: anchor zinciri ZATEN exact — bir bitlik sapma anchor'da
yakalanır; histogram ikinci cephe: **anchor kırıldıktan sonra** hangi
sektör lobu kaydı lokalize etmeye yarayan forenzl iz. Yanlış-kabul riski
yok (anchor'a kodlanmış sadece bir görünüm).

## 5. Validator donanımı — sentinel-grade BOM

- Yerleşim: `place() 1/16` = **3 çekirdek** (2×2 mesh), sinaps/core max
  1617 giriş ×8 B ≈ **13 KiB** (64 KiB cap'in %20'si) — SRAM tavanın altında.
- Debim: 500 SOP/tick skala ölçüsüyle (python `[fabric]` pinli).
- Enerji: 48,529 pJ/tick × 48 tick ≈ **2,3 µJ / sentinel verdict** — bir
  validator node binlerce/quorum turu koşturur.
- **Anchor başına kanıt baytı**: canary 32 B; sentinel verdict raporu
  (anchor 32 + üç sayaç 24 + verdict 1) ≈ **57 B**; dispute 72 B.

Güvenlik yüzeyleri (kapanış notu): RNG akış disiplini (tek çekim sırası,
tek builder — `sentinel_stimulus` refactor bu turda tekleştirildi), digest
bağlanma kapsamı (10..16 & 20..32 unbound, pinli+dokümante), dispute
yalıtımı (lazy-liar fixture in-crate, yanıltıcı olmadan).

## Yolunda kalanlar

- Probing v2: APL duyarlığına özel lezyon (anaerobik PN sürüşü).
- Lezyon-bataryası XBüyütmeler: core-hotspots → `fault_row` simülasyonu
  bağlantısı.
- BudZero C1–C7 zk düşürümü: `air.rs` çalıştırılabilir taslak olarak
  bekliyor; iskambil değil, pipeline.
