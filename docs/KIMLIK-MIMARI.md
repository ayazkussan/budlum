# Kimlik ve Kimlik Bilgisi (Identity/Credential) Alt Sistemi Mimarisi

## Durum

Bu doküman mimariyi **nihai** hale getirmez. Açık sorular §33'te (DENETIM raporu) ve yanıtlandıkça buraya işlenecek.

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
