# 02 — PR50 / Deneme / Seed Durumu (2026-09-10)

## PR50 (universal PR)

`budlum-xyz/budlum#50` — "Universal Settlement Layer", `usl` → `main`,
**OPEN**. Yazar: `lubosruler` (+ CodeRabbit bot commitleri). Direktifteki
"universal PR bulguları" = bu PR'ın içeriği; "PR50'ye commit" = bu PR'ın
`usl` dalı (bu oturumdan yazılmaz; aktarım kullanıcıda).

## Deneme (`budlum-xyz/deneme`)

- `main`: yalnızca `README.md`. Tanım: PR#50-`usl` ağacının 20 parçalı
  denetim kopyası; parça 20 birleşince ağaç `usl` ile bayt-bayt aynı.
- **490+ OPEN PR**: `parca-08` … `parca-20`, her biri 2-3 bölüme ayrılmış
  (`parca-NN-p1/p2`, ör. #464 … #493).
- Örnek #493 (`parca-20`, 2/2): +21846/−3728; CI workflow'ları, baseline
  dosyaları (`unwired-guards`, `idle-code`), README'ler.
- "Denetçi AI onayı" bu PR'ların review/CI durumudur. Tümü bu turda
  okunamaz; tabakalı örnekleme kuyrukta (`03-SIRADAKI-IS.md` #4).
  Not: parça PR'ları ağaç-rekonstrüksiyonu taşır; bulgu-yorumu ile
  içerik-üretimi ayrımı örneklemede yapılır.

## Seed (`budlum-xyz/seed`)

Küçük çekirdek (Rust + `web/`): içerik → taşıyıcı → tarif hattı
(QR/fountain/commitment). 8 PR'ın tamamı dependabot (npm/cargo bump).
Denetim-bulgu PR'ı yok. "Bulgunun dümdüz koda atılması" taraması
kuyrukta (`03-SIRADAKI-IS.md` #7).

## Workspace okuma notu

`budlum-xyz/workspace` yok; kaynak `ayazkussan/workspace` (`main`).
`32-base` = tur bütçe kuralı (`DEVIR-DOSYASI.md` §6.1: 8 araştırma +
1 CI kontrolü + 6 skill + 17 kod; tur `ask_user` ile biter). Bu oturum
kısıtlarına uyarlanarak uygulanır: tek dal, kısa chat özeti + detay
dosyalarda, R&D kararları toplu `ask_user`.
