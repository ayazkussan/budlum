# repo-lubot — tek PR aynasi

Kod: `ayazkussan/lubot`, dal `olcum-disiplini` (PR#1), base `12bc9ac`.
Bot bu fork'a push edemedigi (403) icin commit'ler format-patch olarak
aynalaniyor; dagitim kullaniciya ait.

## Uygulama
```
git -C <lubot-klonu> fetch origin olcum-disiplini
git -C <lubot-klonu> checkout olcum-disiplini
git -C <lubot-klonu> am patches/*.patch
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
