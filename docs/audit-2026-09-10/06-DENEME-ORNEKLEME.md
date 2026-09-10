# 06 — Deneme Örneklemesi v1 (2026-09-10)

## Yöntem

Tabakalı örnekleme, dilim 1: en yeni parça (#493, parca-20 2/2) + eski
parça (#464, parca-08 3/3). Ölçülen: meta + review durumu + CI sinyali.
İçerik denetimi sonraki dilimlerde (parça başına 1 PR).

## #493 — parca-20 uyarlamasi, bolum 2/2 (PR #440 bolundu)

- +21846/−3728, 100 dosya. Örnek yollar: `.github/*` (workflow'lar,
  baselinelar), `.quality/*`, README'ler, `bud/*` (format, fuzz, kani...).
- Review: YOK. CI: CodeRabbit pass (tek sinyal; deneme CI izi yok).
- Nitelik: ağaç-rekonstrüksiyon parçası (usl içeriği taşıyor).

## #464 — parca-08 uyarlamasi, bolum 3/3 (PR #428 bolundu)

- +199264/−1536. Review: [] (YOK).

## Hüküm

Onay kapısı KAPALI: örneklenen dilimde denetçi onayı sıfır. Direktif
§8'deki "denetçi onayı alınmadan PR'a commit atılamaz" koşulu
karşılanmıyor → bulgu-kodlama akışı bu dala bulgu olarak girer,
"onaylı iyileştirme" sayılmaz (sınıflandırma korunur).

## Sonraki dilim

Her parçadan 1 PR: meta+review+CI taraması (ucuz döngü); içerik
derinlemesine yalnızca review'li/CI-kırmızılı PR'larda.
