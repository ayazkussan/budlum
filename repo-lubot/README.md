# repo-lubot — tek PR aynasi

Kod: `ayazkussan/lubot`, dal `main`, base `37d32c9bdf50e8a28ac590d8ec31a83629002b4a`.
Bot bu fork'a push edemedigi (403) icin commit'ler format-patch olarak
aynalaniyor; dagitim kullaniciya ait.

2026-09-10 notu: onceki base `12bc9ac` (dal `olcum-disiplini`, PR#1)
silinmis; 5 yama main uzerine rebase edildi. Tek elle mudahale:
0002'deki `ajan.jsonl` satiri (dosya icerigi ayrismis, anchor yok)
dosya sonuna eklendi, icerik ayni + JSON dogrulandi; diger 4 yama
degisikliksiz uygulandi.

## Uygulama
```
git -C <lubot-klonu> fetch origin main
git -C <lubot-klonu> checkout main
git -C <lubot-klonu> am patches/000*.patch
```
Patch sirasi 0001..0005 (sira kritik). Base disinda uygulaniyorsa
`git am -3` veya dosyalar elle kopyalanir.

## Icerik (5 commit)
1. izolasyon crate (lubot-izolasyon): session izolasyon siniri, checkable
   contract (4 test).
2. denetim crate (lubot-denetim): scan -> validate -> fix kanitli defter
   (9 test): evidence-gated closure, High/Critical waiver attester zorunlu,
   degisen bulgu eski closure'u bozar, complete() gate, verify() canary.
3. README + ratchet: 191 test (178+4+9), 0 pedantic, layout tablosu.
4. rustfmt hizalamasi (1 test cagrisi).
5. kapi: review-crate-holds-ledger-rules (38. kapi, canary'li self-test).
