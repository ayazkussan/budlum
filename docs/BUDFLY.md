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

## Genişleme probları (v2): sineğin yapısını sömürmek

Beş prob, hepsi `goldens.anchor.toml` `[expansion]` bloğunda donmuş (35 pin)
ve `scripts/expansion_check.py` + Rust testleriyle iki dilde denetleniyor.
Kültür kuralı bu turda da aynı: **naif iddia refüte edildiyse refütasyon
pinleniyor, silinmiyor.**

### Graf sayımı (`analysis.rs`) — haritanın iskeleti
Tamsayı, sıfır dinamik: derece toplamları + duyu→komut BFS jeodezikleri.
Sol lamina **413/588** nörona iner (tek gözden komut katmanına), DNa02
havuzlarına uzaklık **4 sinaps** (DnR max 5). Derece toplam parmak izi:
`dd38f2c5…`.

### Lezyon bataryası (`lesion.rs`) — nöroetoloji + arıza davranışı
Kanık koku probunda (lamina bump + PN paterni, 48 tick) üç ablasyon:
- `no_eb`: pusula zinciri çöküyor — EB 517→0 spike, FB 98→4, INH 103→0,
  MDN 5→0 (kaskad, anchor: `80ed2517…`).
- `no_mdn`: veto havuzu sönüyor, geri kalan sayaçlar aynı (anchor: `56a0e023…`).
- `no_apl`: **ölçülemez** — prob v1'de APL sessizliği MBON sayacını
  değiştirmiyor (delta 0 pinlenmiş). Dürüst negatif: bu prob APL'nin rolüne
  duyarlı değil; zayıflığı saklamıyoruz, istersen v2 odağı budur.

### Pusula wander (`compass.rs`) — attractor iddiasının hakiki sınavı
6-geniş bump 0..8 tick, sonra 64 tick sessizlik: **bump sektöründe
KALMIYOR** (test: hold_sector0 = 0/64, t=40 histogramı 9 kova). Toplu-dönü
istatistiği (ring golden, 4.6°) hâlâ geçiyor ama tick-tick kilit yok —
dondurulmuş parametrelerle ring ancak kayan/difüze bir bump üretiyor.
Donanım notu: kazm için CX_INH kazanı (veya PEN-tarzı ofset sürüş) gerekiyor;
yol haritası, iddia değil.

### Koşullu baskılama (`learn.rs`) — mantar cisim öğreniyor
CS (çift KC, 8..24) × US (MDN, 12..20), iz-kapı (MDN spike t veya t−1,
pre t−1, post t) → KC→MBON ağırlıkları -128/128 tabanıyla baskılanıyor.
Sonuç: **63 kenar değişti** ve MBON yanıtı 8→8→5→6→4→6→3'te çöküyor —
davranışsal US-eşleşme baskısı, tick-tick anchor'lı. İleride gözcü'nün
öğrenilmiş-ağırlık manifesti pinlemesine hazır `learn = cb0b8470…`.
Tuzak notu: katı eş-tick kural bu delay-1 mimaride **imkânsız** (MBON, KC
ateşinden bir tick sonra, KC refraktör haldeyken çaktırır → parity kesişimi
∅). İz-kapı bu yüzden doğrudur.

### Çekirdek-arıza tablosu (`fabric.rs::fault_row`) — donanım dayanıklılığı
3123 çekirdek SRAM/neuron boyutlamalı, throughput değil: **%33 kayba kadar
hız düşmüyor** (k≤512: 3 çevrim/tick, 333M tps), 1024 ölüde 4 çevrim /
250M tps, enerji sabit. Kaybın tümü yoksa None (kuma ölür).

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
