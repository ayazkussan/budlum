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
Patch sirasi 0001..0017; 0010 ve 0013 kok `Cargo.toml`'a dokunur ve bu ikisi
tek satir baglamla uretildi (`git -c diff.context=1`). Neden olculdu: uc
satirlik baglam, bos satir duzeni farkli olan bir kok manifeste dustu; tek
satirla iki duzende de gecti. 0013'ün `docs/CRATES.md` huntesinin on-goruntusu
bu serinin kendisi tarafindan olusturulur, yani baglam garantili - zincir
sira ile uygulandiginda.
Base disinda uygulaniyorsa
`git am -3` veya dosyalar elle kopyalanir.

## Icerik (17 commit, 0006..0017 bu turda eklendi)
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
11. takip crate (lubot-takip, 715 satir / 12 test): baglanti iddiasi veri
    olarak tutulur - sembol, Wired/Unwired, ve Wired ise cagri noktasi + rota.
    `recompute(tree, commit)` agaci tekrar tarayip iki yonu de kirar: bagli
    iddianin cagrisi kaybolmussa `SiteNoLongerHolds`, baglisiz denilen sembole
    cagri geldigini goruyorsan `UnwiredClaimNowHolds` (yalniz kuculen bir
    ratchet'in goremedigi yon), commit degistiyse once `CommitMoved`, hic iddia
    yoksa `Empty`. Budlum'daki bayat `WIRING:` yorumlari, 205 olu `pub fn` ve
    hic declare edilmemis kapi hastaliklarinin Lubot tarafindaki karsiligi;
    metin aramanin cagri grafigi olmadigi crate'in kendi dokumaninda vekalet
    olarak adlandiriliyor - bu yuzden dogruluk degil sapma raporlar.
12. muhur crate (lubot-muhur, 631 satir / 11 test): sadece-eklenen zincir; her
    girdi onceki ucu mühurler, `finalize()` son ucu saklar ve `verify()`
    baytlari yeniden sayar. Yeniden yazma (`Rewritten`), kuyruk silme
    (`TailMoved`), orta girdi koparma (`Gap`), finalize-edilmemis zincir
    (`NeverFinalized`), ayri indeksin suruklenmesi (`IndexDrift`) ayri hatalar.
    FNV-1a seciminin siniri - saldirgan zinciri yeniden hesaplayabiliyorsa bu
    katman yetmez, anahtarli insa gerekir - dokumanda yazili; anahtarli gereken
    yerde bunu bir tip soyleyecek, bir yorum degil.
13. workspace: takip ve muhur `members` listesine + envanterin iki yeni satiri
    (`docs/CRATES.md`). Envanterin kendisi de bir baglanti iddiasidir; ihlal
    sutunu da bu yuzden ayni commit'te guncelleniyor.
14. kuyruk crate (lubot-kuyruk, 708 satir / 11 test): sinirli bakim is kuyrugu.
    Degismez: `submitted == in_flight + done + dead_letters + dropped + refused`,
    `verify()` bunu sayaclardan yeniden sayarak. Dolu kuyruk yalniz daha ucuz
    sinifi cikarir; `Repair` bir `Repair`'i cikarmaz - sigmiyorsa gelen red
    *sayilir*. Ayni key'e ikinci gonderim merge edilmez: bir shard icin iki bilet
    iki operatörün parasidir. Deneme hakki biten is olu-mektuba gider ve
    liste yer acmak icin kisaltilmaz. `fail` `Some(Dead)`/`None` döndürür -
    "vazgectik" bir hata degil, raporlanacak bir sonuc.
15. erisim crate (lubot-erisim, 962 satir / 13 test): yetki defteri. Kapsama
    ayiracli yol (`src/storage` → `src/storage/deal.rs` evet,
    `src/storagesecreta`/`src/storage-deal` hayir - saf `starts_with`'in
    yaptigi sey buradaki testin tek amaci); devir alma parent'in KAYITLI
    sinirlarina karsi kontrol edilir ve genisleyen boyut adlandirilir; iptal
    silmek degildir ve ebeveyn iptali alt agaci da iptal eder (aksi halde iptal
    edilen kok, iptaldan once cikarilmis dar kopya uzerinden calismaya devam
    eder); saat okunmaz, `at` parametredir - `>=` siniri o yuzden test
    edilebilir. Anahtar/paraf yok: defter neye izin verildigine karar verir,
    tokenin gercekligine iddia etmez. `verify()` izi kayitlara karsi sayar.
16. workspace: kuyruk ve erisim `members` listesine + envanterin iki satiri.
17. anlama crate (lubot-anlama, 2443 satir / 24 test): komutu, emretmeden once
    oku. Kelime kelime kayit: her kelimenin ne kattigi (rol) ve nereden bilindigi
    (stated / inferred / assumed). Kirildigi yerler: kelimesi desteklemeyen kapsam
    (`ScopeWithoutWord`), gevsek eslesip duzeltmesi kaydedilmemis kelime
    (`SilentCorrection`), listelenmemis varsayim (`UnlistedAssumption`), ek
    metnindeki emrin eylem listesine inmesi (`InstructionInData`), ve sozcukle
    desteklenmeyen her eylem (`ActionWithoutWord`: `claim()` bir iddiadir,
    inanilmaz - `Command` iddiasi kelime listesinde geri arandirilir).
    Negatiflik (V+ma/me) sozlukten ONCE bakilir ve yalniz bilinen filere
    uygulanir: `yazma` asla `WriteCode` olmaz, `yuzme` de bir yasak uydurmaz;
    `silmesin` iki katmanli dusurulur (`sil`+`me`+`sin`) - bir olumsuzlugu emir
    okumak bu tablonun yapabilecegi en kotu hata. Tolerance kademeli: <=6 harf
    hic, 7-9 bir, >=10 iki; bes harfli bir kelimenin tek mesafesi iki fiili ayni
    anda esitleyebilir, o yuzden orada tahmin yok. Tirnak ici veridir:
    `Role::QuotedData` eslesmez, eylem uretmez, sorgu dogurmaz. Ek dosyalarinin
    emirleri `Attachment::new`'de karantinaya alinir; `verify()` karantinayi
    degil SONUCU kontrol eder, cunku tip karantinayi kendisi kuruyor - ve
    `mentions()` tek kapi oldugu icin ek metin yalniz sozlugu zenginlestirir.
    Kayit elle duzeltilebilir (`revise`, `extend_scope`); ikisi de dogrulama
    ister ve `verify()` o duzenlemelerin biraktigi izi arar. Ambiguity listesi
    kayitla birlikte kurulur, ayri duzenlenemez - "sessizce secildi" hastaligi
    kontrolle degil kurulusla engellenir; bu sinir crate'in `verify()` dokumaninda
    yazili.

Test ratchet'i: 191 -> 399. Yeni testler 116 = yetenek 13 + olcek 10 + kanit 12
+ mimari 10 + takip 12 + muhur 11 + kuyruk 11 + erisim 13 + anlama 24. Lubot'un `training/ratchet.json` dosyasi
0003'te 191'e baglandi; o sayiyi buradan degistirmiyoruz - guncelleme Lubot
kosusunda `cargo test` gercekten kostuktan sonra, gercek sayiyla yapilir.

Dokuz crate de std disinda bagimlilik
kullanmiyor, I/O yapmiyor, saat/rastgele okumuyor; toplam 8427 satir, prod
tarafinda sifir unwrap/expect. Delimiter dengesi (paren/brace/bracket,
stringler ve yorumlar cikarildiktan sonra) dokuz dosyada da sifir - bu kum
havuzunda yapilabilen tek yapi kontrolu buydu. Ek olarak her satir 100
karakteri gecmiyor (Turkce karakter karakterle sayilir, byte ile degil) ve
hicbir fonksiyon 100 satiri asmiyor: `too_many_lines` pedantic'te uyaridir ve
`-D warnings` onu hataya cevirir. `read()` bir keresinde 276 satira cikmisti;
`read_word` + `apply_glossary` + `infer_the_obvious_work` + `record_the_gaps`
olarak kirildi.

**Derlenmedi.** Rust araci yok ve indirilemiyor; dolayisiyla "testler gecti"
diye bir iddia yok, iddia su: patch'ler uygulaniyor, dosyalar yerinde, kurallar
test olarak yazili. Lubot'ta kosulacak komut:
`cargo test -p lubot-yetenek -p lubot-olcek -p lubot-kanit -p lubot-mimari -p
lubot-takip -p lubot-muhur -p lubot-kuyruk -p lubot-erisim -p lubot-anlama`.
