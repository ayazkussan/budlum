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
Patch sirasi 0001..0010 (0010 en son; glob degil dosya adinin tamamini ver).
Base disinda uygulaniyorsa
`git am -3` veya dosyalar elle kopyalanir.

## Icerik (10 commit)
1. izolasyon crate (lubot-izolasyon): session izolasyon siniri, checkable
   contract (4 test).
2. denetim crate (lubot-denetim): scan -> validate -> fix kanitli defter
   (9 test): evidence-gated closure, High/Critical waiver attester zorunlu,
   degisen bulgu eski closure'u bozar, complete() gate, verify() canary.
3. README + ratchet: 191 test (178+4+9), 0 pedantic, layout tablosu.
4. rustfmt hizalamasi (1 test cagrisi).
5. kapi: review-crate-holds-ledger-rules (38. kapi, canary'li self-test).
6. yetenek crate (lubot-yetenek, 782 satir / 13 test): beceri karti = tetik +
   cikis kaniti; kabul ancak karti kapatan bir kosu kaydedilirse; celiski
   kartin satirini silmez, Status::Rejected'a dusurur. Rotasyon: kart ancak
   baglam onun yetenegini isterse cagrilir; cagrilmayan kart basarisiz sayilmaz
   ve cagrilmama listesi rapor edilir. Bir kart reddedilir: baska bir aracinin
   adini isim/adimlarinda ode tasisiyorsa, tetigi bos ise, adimi bos ise, ya
   da cikis kaniti yalniz soz ise.
7. olcek crate (lubot-olcek, 715 satir / 10 test): sert tavan + yumusak su
   hatti + ayrilmis taban, iki havuz. Normal is tavan-taban'i doldurur, su
   hattini gecmek iddia ister ve sayilir; kapanis isi tabani harcayabilir
   ("okuyacak yer kaldi, dogrulayacak kalmadi" cumlesi kurulamaz). Sigmayan
   sey sayiyla reddedilir; tahliye edilen her satir dusurme defterine yazilir;
   verify() toplamlari yeniden sayip sayilmadan cikmis maliyeti bulur.
8. kanit crate (lubot-kanit, 883 satir / 12 test): kapsama kilidi + plan
   kilitli gozlem -> iddia -> bag -> yol -> kos -> kapanis. Anlatim kapanis
   yapamaz; yalniz sozle desteklenen bulgu described_only()'de durur (denetim
   AI'si bulgu metnini koda yapistirdi hatasinin makinece karsiligi). Kapanis
   sonrasi kanit cekilirse verify() kapanisi gecersiz kilar.
9. mimari crate (lubot-mimari, 588 satir / 10 test): modul tablosundaki her
   sembol, tablonun "burada" dedigi dosyada bir bildirim anahtariyla gecmek
   zorunda; sozlesmesi bos satir, bos tablo ve tek satirlik tablo gecermez.
   Bayat WIRING yorumu / isim degismis sembol hastaliginin kapiya baglanmis hali.
10. workspace: dort crate members listesine + docs/CRATES.md envanteri. Bu
    patch tek satir baglamla uretildi (`-c diff.context=1`), cunku kok
    Cargo.toml'un geri kalanini gormuyoruz; farkli bir duzende bile uygulandigi
    ayri bir klon uzerinde test edildi.

Uygulama sirasinda 0006..0010 sirasi kritik degil (0010 en son), ama 0010
uygulanmazsa dort crate `cargo test`'e girmez: kapiye baglanmayan kod gibi,
listeye baglanmayan crate de yoktur - gorunmez kodun iki bicimi.

Test ratchet'i: 191 -> 236 (yeni 45). Dort crate de std disinda bagimlilik
kullanmiyor, I/O yapmiyor, saat/rastgele okumuyor; 2968 satir, prod tarafinda
sifir unwrap/expect (not: hicbirisi bu kum havuzunda derlenmedi - Lubot'ta
`cargo test -p lubot-yetenek -p lubot-olcek -p lubot-kanit -p lubot-mimari`).
