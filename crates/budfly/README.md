# BudFly

**Meyve sineğinin haritası, donanımın çılgın açısından, Budlum'un kanıt disipliniyle.**
*The fruit fly's map, from the crazy hardware angle, under Budlum's proof discipline.*

BudFly üç iddiayı kod olarak kurar; üçü de `cargo test` ile denetlenir:

1. **Harita çalışır.** `connectome` modülü MaleCNS-tarzı bir merkezi sinir sistemi
   üretir: görsel yol (lamina→medulla→lobula), pusula halkası (FB→EB ring attractor),
   mantar cisim (PN→KC→MBON, APL) ve inen komut havuzları (DNa02-benzeri L/R + MDN veto).
   Ring attractor *ortaya çıkar* — bump testi bunu ölçer, kimse elle koymaz.
2. **Donanım karşılar.** `fabric`, grafiği bir nöromorfik çok-çekirdekli mesh modeline
   yerleştirir (AER spike paketleri, XY routing, pJ enerji muhasebesi) ve yayınlanmış
   MaleCNS ölçeğinde (166.700 nöron / 25,58M bağlantı) silikonun neye ihtiyacı olduğunu
   analitik olarak cevaplar: **3.123 çekirdek, 56×56 mesh, ~3 çevrim/tick,
   saniyede ~333M tick @1GHz, tick başına ~0.58µJ** — sineğin milisaniye saatinden
   üç kademe hızlı.
3. **Yürütme kanıtlanabilir.** `sim` tamamen tamsayı (Q4.12) LIF'tir ve her tick bir
   SHA-256 anchor zincirine katlanır. `air`, BudZero'nun ileride aritmetize edeceği
   geçiş kısıtlarının (C1–C7) çalıştırılabilir tanımıdır. `oracle`, settlement'a
   bağlar: 32-bayt digest girer, anchor'lı transkript çıkar.

```
digest (32B) ──► stimulus (pusula sektörü + lobula bitleri)
                    │
                    ▼
        donmuş connectome (v1.0) — 48 tick LIF, tamsayı
                    │
        her tick: anchor ← SHA-256(anchor‖tick‖spike‖v)
                    ▼
        DNa02_L / DNa02_R / MDN spike sayıları ──► verdict
                    │
                    ▼
           (verdict, anchor) — settlement'a kaydedilebilen nesne: anchor
```

## Çalıştırma

```sh
cd crates/budfly
cargo test                        # goldens + scenarios + birim testler
cargo run --example fly_chip_report   # donanım boyutlandırma raporu
python3 scripts/reference_check.py    # Python referansı: goldens burada dondu
```

## Gerçek vs deneysel (FlyCoder usulü dürüstlük tablosu)

| Bileşen | Durum |
|---|---|
| Devre motifleri (pusula ring attractor, KC kodu, DN rekabeti) | Gerçek nörobilim literatüründen, **stilize** oranlarla |
| Nöron/bağlantı sayıları (jeneratör) | Stilize — MaleCNS veri seti **değil** |
| 166.700 nöron / 25.582.938 bağlantı boyutlandırması | Yayınlanmış gerçek rakamlar üzerinden analitik |
| LIF dinamikleri | Basitleştirilmiş, Q4.12 tamsayı, donmuş sabitler |
| Nöromorfik mesh (AER/XY/enerji) | Model; sabitler mertebe-mertebesine, tape-out iddiası yok |
| Anchor zinciri | Gerçek SHA-256, çapraz-dil doğrulamalı golden'lar |
| Verdict anlambilimi ("finality'yi onaylama") | **Deneysel kodlama** — sinek davulları finalite anlamaz |
| Settlement'a kaydedilen nesne | Anchor (yürütme kanıtı), verdict değil — fails-closed |
| BudZero STARK kanıtı | **Henüz yok** — C1–C7 aritmetizasyona hazır taslak |

## Donmuş katman (generator v1.0)

Şunlar golden anchor'ların parçasıdır; değiştirmek yeni bir jeneratör
versiyonu kesmektir, düzeltme değil: bölge tabanları/floor'ları, motif RNG
akış etiketleri (`0xA11CE/0xC0FFEE/0xDEC0DE/0xF00D`), kenar üretim düzeni,
dinamik sabitler (`V_TH=1.0 Q4.12`, `W_SYN=512`, `I_STIM=4.0`, `REFRAC=1`,
leak `>>4`, 1-tick sinaptik gecikme, ±64 fan-in clamp).

## Bağımlılıklar

Yok — kasıtlı olarak. `budlum-note-packing` ile aynı kural: crate, node'u,
zkVM araçlarını ve denetim scriptlerini üç ayrı workspace'ten erişilebilir
kılmak için bağımlılık ağacı taşıyamaz. SHA-256, RNG ve dinamikler içeride.
