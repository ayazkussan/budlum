# repo-workspace — tek PR aynasi

Kod: `ayazkussan/workspace`, dal `main`, base `18167ee`.
Bot bu fork'a push edemedigi (403) icin commit'ler format-patch olarak
aynalaniyor; dagitim kullaniciya ait.

## Uygulama
```
git -C <workspace-klonu> checkout main
git -C <workspace-klonu> am patches/*.patch
```
Patch sirasi 0001..0034 (sira kritik; hafiza.md append'lari birbiri
uzerine yazilir). 0034 ayrica `git am` ile temiz bir base'de sinandi
(110 satirlik dosya olusturuyor, rc=0) — apply edilebilirlik iddiasi
gozleme dayali, varsayima dayali degil.

## Icerik (liste 1..17'yi sayiyor; ayna 0034'te — 18..34 denetim dalga kayitlari)
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
11. Fork Actions acildi + workflow envanteri (20 dosya, 0 secret).
12. Merge yasaagi karni + orphan fork main bulgusu + sandbox yenileme.
13. Audit dalga 1: state root kapsami (F-1 HIGH + fix e4f472b) +
    orphan main merge cozumusu (e0c1ca3) + CI basladi (19 run;
    Dependency Review failure, log bekleniyor).
14. Dependency Review sorusturmasi: upstream kontrolu (PR 57 pass),
    hipotezlerin elenmesi, fork log altyapisi EOF bulgusu, kalan
    hipotez + aday duzeltmeler.
15. Audit dalga 2: executor para yolu (E-7 HIGH: public RPC off-block
    consensus mutasyonu — fix fe5461e operator gate; E-1..E-4).
16. E-1 fix kaydi: a8b45e1 (beas kola vesting spend gate'i + test).
17. Dalga 3: F-3 HIGH (global_header_summary node-local degerdi state
    root'ta — operator seal'i self-fork yapardi; fix 8ba7eab) + F-2 /
    domain registry kapanislar (node-local operator policy, tasari
    tutarli) + sandbox yenileme (16 patch'ten yeniden kurulum).

## Glob uyarisi (0034 turunda bulundu, ayni commit'te kapatildi)

Onceki komut `am patches/000*.patch` idi; glob `000*` yalnizca 0001..0009'u
esler. Yani 0010..0034 — yirmi bes patch — talimata uyan bir klon tarafından
HIC uygulanmiyordu ve hicbir sey hata vermiyordu: `git am` kendisine verilmeyen
dosyayi ozetmez. `git am patches/*.patch` kullanimi siralamayi dosya adindaki
sifir dolgudan alir (0001..0034 icin lexicographic = sayisal), bu yuzden guvenli.

Ayni hata `repo-lubot/README.md`'de de vardi; orada 5 patch'in 5'i de `000*`
araligina dustugu icin zararsizdi, ama 6. patch'te sessizce dusururdu — o da
ayni sekilde degistirildi.

Bu vaka direktifin Bolum 8'de aradigi anti-ornenin tam karsiti yonde bir ornegi:
talimati yazan denetim AI'si bulguyu (patch sayisini) guncellemis, ama o bulgunun
uygulanmasini saglayan komutu guncellememisti. Rapor dogru, komut bayat — ve
bayatlik bir basari koduyla geciyor.
