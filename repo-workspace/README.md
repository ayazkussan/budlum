# repo-workspace — tek PR aynasi

Kod: `ayazkussan/workspace`, dal `main`, base `18167ee`.
Bot bu fork'a push edemedigi (403) icin commit'ler format-patch olarak
aynalaniyor; dagitim kullaniciya ait.

## Uygulama
```
git -C <workspace-klonu> checkout main
git -C <workspace-klonu> am patches/000*.patch
```
Patch sirasi 0001..0005 (sira kritik; hafiza.md append'lari birbiri
uzerine yazilir).

## Icerik (10 commit)
1. DENETIM-2026-09-09G: AR-GE-6 F-12 + Lubot izolasyon raporu + hafiza.
2. DENETIM-2026-09-09H: AR-GE-6 production writer + larva bulgusu +
   Lubot denetim + hafiza.
3. H eki: denetim kapi (38) + CI trigger notu.
4. Sigorta kaydi (patch konumu, gist engeli).
5. Surekli denetim direktifi + tek PR modeli (kullanici kararlari).
6. DENETIM-2026-09-09I: B.U.D. entegrasyon denetimi (canli repoda
   UNWIRED durumlarin yeniden dogrulanmasi) + hafiza.
7. Tam surekli-denetim direktifi kaydi + BAGLANTILIK-LOG (modul
   durumlari SHA'li; VerifyMerkle canli tefidi) + CI bulgusu.
8. Kararlar: upstream PR = CI kaynagi (fork Actions acilmayacak);
   kalici Arena oturumu = zamanlayici (cron kurulmaz).
9. Lubot cagri zincirleri canli dogrulama + hafiza.
10. BAGLANTILIK-LOG Lubot: 4 zincir BAĞLI (SHA'li kanit) +
    src/lubot sapmasi.
