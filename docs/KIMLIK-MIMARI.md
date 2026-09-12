# Kimlik ve Kimlik Bilgisi (Identity/Credential) Alt Sistemi Mimarisi

## Durum

Uygulama sürüyor - kodda olan ve sırası gelen, EK'te ve DENETIM §33-§36'da ölçümüyle yazılı. İki açık uygulama kararı kapandı: proto+executor dilimi tek kolla düştü (`Identity(IdentityTx)`, elle eşlenmiş wire tablosu, Pollen deseni); `identity_root`'un aktivasyon epoch'u sorusu "bump kendi başlatması" ile çözüldü (`BDLM_GLOBAL_BLOCK_V5`, pre-launch, çift-hesap penceresi yok - snapshot şema-5 bump'ıyla aynı kural). Executor kolunun domain sorusu ağacın kendi cevabıyla kapandı: motor `domain_kind()`'ını bildirir, `AccountState::execution_domain` state'i her kurulduğunda/yer değiştirdiğinde o damga; transaction asla domain taşımaz. Vault kapısı da üç dilimle geldi (şema-6 kalıcılık + `VaultTx` kolu + transfer/burn kilitleri) - sıra, cross-domain DOĞRULAYICI tarafında (commitment + Merkle proof), o bu dokümanın ayrı satırı.

## Amaç

Budlum'un evrensel L1 iddiası, doğrulanabilir kimlik desteğini zorunlu kılıyor. Bu doküman, kimlik/credential alt sisteminin Budlum'un mevcut mimarisine tam olarak nasıl oturduğunu tanımlar.

## Referans

W3C Verifiable Credentials 2.0, SD-JWT VC, `did:solidus` metodu, BBS+ selective disclosure, validator-backed anchoring.

## Mimari Kararlar (özet)

| Soru | Karar |
|---|---|
| Native protokol mü, registry katmanı mı? | **Registry katmanı.** Konsensüs kuralı değişikliği yok. |
| Hangi domain(ler)de yaşayacak? | **PoA domain'de ana kayıt**, diğer domainlerde cross-domain doğrulama. |
| Standart | **W3C Verifiable Credentials + Budlum'a özgü `did:bud` metodu.** |
| Cross-domain taşınabilirlik | **CrossDomainMessage üzerinden commitment + Merkle proof.** |
| StateSnapshotV2 ilişkisi | **Yeni alt-ağaç, schema-version-4.** |
| Ham kimlik verisi zincir üstünde mi? | **Hayır.** Sadece commitment/hash. Bu, açık soru değil, sabit ilkedir. |

---

## 1. Katman: Registry Tabanlı Mimari

Kimlik sistemi, `VerifierRegistry`'nin yanına oturan yeni bir **`IdentityRegistry`** modülü olarak inşa edilir. Gerekçe:

- Budlum'da zaten kanıtlanmış bir registry pattern'i var (`VerifierRegistry`, `PermissionlessRegistry`). Yeni bir konsensüs mekanizması icat etmek yerine bu pattern'i genişletmek, standart kuralınıza (mevcut primitifleri önce ara) uyar.
- Konsensüs katmanına dokunmamak, şu anda ADIM11'de aktif olan liveness/slash çalışmasıyla çakışma riskini sıfıra indirir.
- Registry, credential *commitment*'larını (hash/kök) tutar — ham veriyi değil.

`IdentityRegistry` sorumlulukları:
- DID kaydı ve güncel durumu (`did:bud:<address>` → mevcut anahtar/credential kökü)
- Credential commitment'larının (issuance, revocation) kaydı
- Guardian/social recovery için kurtarma anahtarı işaretçileri (ham veri değil, sadece hash)

## 2. Domain Yerleşimi: PoA Ana, Diğerleri Doğrulayıcı

`IdentityRegistry`'nin **yazma otoritesi PoA domain'inde** yaşar. Gerekçe: PoA zaten kurumsal/izinli aktörler için var; KYC/uyumluluk gerektiren kimlik kayıtları doğal olarak bu domain'in sorumluluk alanına girer — ayrı bir izin modeli icat etmeye gerek yok.

PoW/PoS/BFT domainleri **yazmaz, sadece doğrular.** Bir uygulama bu domainlerden birinde bir credential'ı kontrol etmek istediğinde:

1. PoA domain'deki `IdentityRegistry` kökü, `DomainFinalityAdapter` aracılığıyla periyodik olarak diğer domainlere anchor edilir (StorageRoot'un `GlobalBlockHeader`'a anchor edilmesiyle aynı desen).
2. Talep eden domain, `CrossDomainMessage` ile commitment + Merkle proof alır, son bilinen finalize PoA kökü karşısında doğrular.

Bu, "evrensel" gereksinimini karşılar (her domain doğrulayabilir) ama kimlik verisinin yönetişimini tek bir yerde tutar (duplikasyon yok, tutarsızlık riski yok).

## 3. Standart: W3C VC + `did:bud`

Custom format yerine W3C Verifiable Credentials veri modeli kullanılır. Budlum'a özgü DID metodu: **`did:bud:<adres>`**. Gerekçe:
- Dış cüzdanlar/araçlar (Solidus dahil) ileride Budlum credential'larını doğrulayabilir — interoperabilite, "evrensel" iddiasının bir parçası.
- BBS+ selective disclosure benimsenir: kullanıcı sadece gereken alanı ifşa eder, tüm credential'ı değil.

BNS/.bud resolver ile ilişki: **ayrı katmanlar.** BNS insan-okunabilir isim → adres çözümlemesi yapar; `IdentityRegistry` adres → doğrulanabilir kimlik iddiaları tutar. Biri diğerinin ön koşulu değildir, ama aynı `.bud` isim alanı altında birlikte çalışabilirler (`isim.bud` bir DID'e işaret edebilir).

## 4. Cross-Domain Doğrulama Akışı

```
PoA domain: credential issue/revoke → IdentityRegistry kökü güncellenir
     ↓ (periyodik anchor, StorageRoot deseniyle aynı)
GlobalBlockHeader: PoA identity kökü de anchor edilir
     ↓ (CrossDomainMessage)
PoW/PoS/BFT domain: commitment + Merkle proof alır
     ↓
DomainFinalityAdapter: son finalize PoA kökü karşısında doğrular → geçerli/geçersiz
```

Yeni bir güven varsayımı eklenmez — mevcut cross-domain mesajlaşma güven modeli aynen kullanılır.

## 5. StateSnapshotV2 İlişkisi

`IdentityRegistry` kökü, StateSnapshotV2'ye yeni bir alt-ağaç olarak eklenir. Şema versiyonu **4**'e çıkar (versiyon 3, bilinen `StateSnapshotV2` düzeltmesiydi — bu, ayrı ve takip edilebilir bir versiyon artışıdır). Snapshot uyumluluk testleri bu adımda genişletilmelidir.

## 6. Gizlilik İlkesi

Ham kimlik verisi (isim, doğum tarihi, biyometrik veri vb.) zkvm ile B.U.D. 3.0 sistemi sayesinde korunur gizli kalır.

---

## Ek: Onay kapılı ifşa (kullanıcının sözlü eki, olduğu gibi)

bu yeni bir mimari > eğer bir sistem cüzdanda veriler için belli bir cveri ismi sorarsa ve cüzdan sahibi onaylarsa veri o kullanıcıya gönderilsin ama hangi veriyi yani o nftyi isteyeceği o ekranda gözüksün veri açılsın ama o isteyen kişinin cüzdanına açılsın sadece, ayrıca her bir bilgi bir nftdir

---

## Uygulama notu — altı sorunun cevabı ve ölçüm düzeltmeleri (kod yazılırken doğrulandı)

Doküman metni yukarıda değişmeden durur; burası ona düşülen notlardır, DENETIM §34 ile birlikte.

1. **StateSnapshotV2:** kod `CURRENT = 4`te zaten; "4'e çıkar" kararı **5'e bump** olarak güncellendi. Gerekçe `poa_onboarding`'in bump'sız girişinden farklı: identity alanı digest'e giriyor (`>= 5` kapısı), eski ikili 5'i versiyon reddiyle temiz karşılar; sessiz düşürme revivasyon demek. v4 pin digestleri byte byte aynı kalır (kilit testi var).
2. **DomainFinalityAdapter:** dosya yok ama mekanizma var ve isim ölçümü düzeltildi — `src/domain/finality_adapter.rs` içinde `PoAFinalityAdapter` (PoW/PoS zincir adapterlarıyla birlikte) mevcut. Çapraz-domain anchor `GlobalBlockHeader` kök desenine `identity_root` sahası eklenerek yapılacak (proto dönüşümleriyle ayrı dilim); state-root bağlaması bu dilimde hazır (`calculate_state_root → identity_v1`).
3. **BBS+:** düşmüyor; selective disclosure ispatı mevcut **zkVM/B.U.D 3.0 prover hattından** üretilecek — yeni imza bağımlılığı yok, supply-chain dosyası değişmedi.
4. **Onay akışı:** UX/UI yok; node tarafı kodun tamamı hazır olacak. Bind edilen yer: `src/storage/view_grant.rs` deseni (grantee-bound, digest-revoked, epoch-opened) — alan-taahhütleri "hangisi" sorusuna cevap olur, grant motoru kimlik için yeniden yazılmaz.
5. **NFT modeli:** "her bilgi bir NFT" kullanıcı kararı; sistem **klasör-NFT** sağlar: klasör, içerdiklerini açan bir NFT olarak davranır (NFT'ler sonradan klasörden taşınabilir — dosya gezgini semantiğinin zincir karşılığı), parça granülaritesini kullanıcı belirler. Onay gelince içerikler cüzdandan alınır ve X belgesinin istenen alanlarına doldurulur — sunum/doldurma motoru kimlik modülünün bir sonraki dilimi.
6. **Tx kapısı:** tam çevrim — Register/Issue/Revoke/Recover dördü birden, RPC ve mempool routing'le.

---

## Uygulama notu 7 — doğrulayıcı tarafın ölçtüğü boşluk (kod yazılmadan önce, ölçüme dayanarak)

Sıra: dört dilim kapandı (proto+executor, `identity_root` V5, sunum okuma yolu, vault kilitleri). KALAN tek kimlik işi "talep eden domain doğrular" yarım cümlesi — ve bu turda `IdentityRegistry::root()` tekrar okundu: kök **sıralı bir akümülatör** (`bud-identity-root-v1` etiketi üzerinde subjectler boyunca ilerleyen zincir-hash), ağaç DEĞİL. Sonuç matematikseldir, tembellik değil: tek bir subject'in kanıtı için zincirdeki ondan ÖNCEKI tüm yaprakların yeniden üretilmesi gerekir — yani kanıt = tüm registry. "Commitment + Merkle proof al, finalize edilmiş PoA kökü karşısında doğrula" cümlesi bu kökle **kurulamaz**; kök yeniden biçimlenmeden doğrulayıcı dilimi yazılırsa yazılan şey doğrulama değildir.

Seçenekler ve ölçülmüş halleriyle:

1. **Kökü iki katmanlı Merkle'a taşı** (önerilen): subject katmanı — yaprak = H(subject | methods-fold | credential_root | guardians-fold), mevcut akümülatör formülü YAPRAK olarak aynen korunur; + iptal katmanı — yapraklar `revoked` kümesinin sıralı elemanları. İspat = iki path (üyelik + komşu-yaprak ile hariçlik/exclusion). Aile içinde icat yok: `merkle_root` ve `disclosure_proof` zaten registry'nin kendi elemanları (credential alanları bu ağacı kullanıyor), tek yarpağın kendisiyle eşlenmesi kuralı ve `bud-vc-v1-node` etiketi dahil yeniden kullanılır. Maliyet: kök digest'i değişir — ve bu **pre-launch'da bedava**, tam `identity_root` V5'in aldığı gerekçeyle: "USL genesis'ten önce yayınlanmış state yok, bump'ın kendisi aktivasyondur". Snapshot kalıcılığı etkilenmez (kök digest'e girmez, `root()` çağrılır), state-root bağı aynı fonksiyona bağlanır, tek taze kilit gerekir: twin-node testi yeni kökle.
2. **Sınırsız replay**: doğrulayıcıya tüm (subject-listesi + credential-id'leri + iptaller) verilir, kök yerinde yeniden hesaplanır. Küçük registry'de "çalışır" — ve tam boyut kuralımızla ölür: birikimi sınırlamayan sınır yorumdur. Ret.
3. **Doğrulama köke hiç dokunmasın, sadece finalize header'ın kendisiyle sınırlı olsun** (isteyen domain `identity_root`'u görür, alan-kanıtını L1 RPC'sinden çeker — `bud_identityCredential` bugün tam bunu veriyor). Bu BUGÜN kurulu durumda doğrulayıcının güvenebileceği tek şey: kanıtın L1'in *anlık* görünümüne dayanması, finalize köke değil. Dürüst sunum: "anchor'a karşı doğrulama" değil "node'a güvenerek okuma". Cross-domain iddiası için yetmez; ama erteleme maliyeti sıfır olduğu için ara adım zaten mevcut.

Karar GÜNCELLENDİ 2026-09-12 - 1. seçenek düştü (`1680547`): kök artık `identity_anchor(records_root, revocations_root)`; yaprak mevcut formülü birebir taşıyor, `is_credential_valid`'ın altı kapısı yerinde, `verify_identity_witness` eklendi, yedi testle pinli. Kalan tüketici dilimi: settlement tarafının iki-path okuyucusu (RPC `bud_identityVerifyPresentation` üstüne bindirilecek); `is_credential_valid`'ın altı kapısı yerinde kalır, üstüne `verify_record_inclusion(anchor_root, subject, witness) -> Result` eklenir ve GlobalBlockHeader'ın `identity_root`'u o kökü taşır — sahanın verdiği söz ile kökün tuttuğu söz aynı yerde buluşur. İki inkâr edilemez ölçüm: `root()` formatı değişirse `identity_v1` state-root bağı DA aynı anda değişir (aynı dilim, iki yalan yarım düzeltmeden evladır); ve credential-iddiaları subject-köküne bağlanır, iptal seti KÜRESEL kalır — günün ispatı iki ağaçtan iki path demektir, tek path'lik sahte-bütünlükten iyidir.
