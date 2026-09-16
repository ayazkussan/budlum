# BudFly — meyve sineği haritası × donanım × settlement

> **Tek cümle:** Budlum'a, MaleCNS-tarzı bir sinek beyninin spike'larını
> tamsayı-tam yürütülebilir, hash-zincirli ve BudZero-STARK'a hazır kılan
> sıfır-bağımlılıklı bir nöromorfik yürütme katmanı (`crates/budfly`).

FlyCoder (MaleCNS connectome'unu donmuş tutup div ortalayan kapalı döngü)
nasıl "166.700 nöron div ortalıyor" çılgınlığını dürüstçe yaptıysa, BudFly
aynı dürüstlükle başka bir çılgın soruyu cevaplar: **donanım bu haritayı
karşılar mı — ve yürütme bir settlement katmanına kanıt olarak kaydedilir mi?**

## Neler var

| Parça | Dosya | İş |
|---|---|---|
| Jeneratör v1.0 (donmuş) | `src/connectome.rs` | Motif-RNG akışlarıyla MaleCNS-tarzı CNS; 1/16 ölçekte 588 nöron / 2.283 kenar |
| LIF çekirdek | `src/sim.rs` | Q4.12 tamsayı, 1-tick gecikme, saturasyon; tick başına anchor katlama |
| Anchor zinciri | `src/sha256.rs` | `anchor ← SHA-256(anchor‖tick‖SHA(spikebitmap)‖SHA(v))` |
| AIR denetçisi | `src/air.rs` | C1–C7 geçiş kısıtlarının çalıştırılabilir tanımı |
| Nöromorfik kumaş | `src/fabric.rs` | Çekirdek/SRAM/SOP modeli, XY+AER, enerji; gerçek-ölçek analitik rapor |
| Settlement sentinel | `src/oracle.rs` | Digest → stimulus → 48 tick → DN/MDN verdict + anchor |
| Python referansı | `scripts/reference_check.py` | Golden'ları üreten/donduran satır-satır ayna |
| Golden testler | `tests/goldens.rs` | Çapraz-dil bit-özdeşlik sözleşmesi |
| Senaryolar | `tests/scenarios.rs` | Ring bump, determinizm, tamper, yerleşim, gerçek-ölçek |

## Donanım cevabı (BudFly-N1 taslağı, yayınlanmış MaleCNS ölçeği)

```
çekirdek          : 3.123 (56×56 mesh) — sinaps SRAM'i bağlayıcı
SOP/tick          : 510.102 (%2 aktivite, fan-out 153)
çevrim/tick       : 3 (@64 SOP/çekirdek/çevrim)
tick/saniye @1GHz : ~333.000.000  → biyolojinin ~3 kademe üstü
enerji/tick       : ~0.58 µJ
```

Yorum: sineğin beyni — tamamı — iddialı ama sıradan bir çok-çekirdekli
kumaşa sığar; mesele kapasite değil, **yürütmenin kanıtlanabilirliği**.

## Budlum ile bütünleşme durumu (dürüst)

* **Bugün olan:** deterministik, anchor-lu yürütme; AIR tanımı; settlement'a
  kaydedilmeye uygun `(digest, config, anchor)` nesnesi. Verdict anlambilimi
  deneyseldir ve consensus-kritik **değildir** — repo'nun AI doğrulama
  duruşuyla aynı: doğrulanamayan anlama değil, doğrulanabilir icra
  taahhüdüne kayıt. Anchor yoksa kayıt yok (fails-closed).
* **Olmayan (iddia edilmeyen):** BudZero üzerinde fiilen üretilmiş STARK
  kanıtı; gerçek MaleCNS kenar listesi; tape-out doğrulaması.
* **Yol haritası:** BudZero'nun ISA'sı tamsayı vektör-op'ları kazandıkça C1–C7
  doğrudan AIR'a düşürülür; `sim.rs`'nin satır semantiği (v, i_ext, i_syn,
  spike, r) bu düşüş için bilinçli olarak lokal tutuldu (1-tick gecikme
  kararının asıl sebebi budur).

## Gerçek-connectome içe aktarımı

Jeneratör stilizedir. Gerçek kenarlar için format: FlyWire/MaleCNS ihracından
`pre_root_id,post_root_id,syn_count` CSV → `w = syn_count × W_SYN`, ve ata
harita bölge etiketi → `Region`. Ağ-büyüklüğünde veri Git'e girmez (repo
kuralı); içe aktarım scripti şekli `reference_check.py`'deki jeneratörle
aynı kenar türünü (`Edge { pre, post, w }`) üretmelidir.

## Üretim kuralları

- `unwrap`/`expect` üretim yolunda yasak (crate-level lint) — repo panik
  kapısıyla uyumlu.
- `forbid(unsafe_code)`.
- Sıfır bağımlılık — `budlum-note-packing` kuralı.
- Generator v1.0 donmuş: golden değiştiren her PR yeni versiyondur.
