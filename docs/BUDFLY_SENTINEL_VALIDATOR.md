# BudFly × Budlum: Lubot uyumu, mimari çapraz-eşleme, oylamasız validator

> Durum raporu + tasarım teklifi. Kod değişikliği `crates/budfly` ile sınırlı;
> bu dosya mimari analizdir ve neyin **var olduğunu** neyin **teklif olduğunu**
> açıkça ayırır. (Repo dürüstlük kültürü: `docs/AI_VERIFICATION_STATUS.md`
> modeli.)

## 1. Yapısal envanter (önce rakam)

`find` ile ölçüldü: repo toplam **329.356 satır Rust** (`src/` 192.165).
En büyük modüller (satır): `src/tests` 32.4k, `src/storage` 30.4k,
`src/chain` 19.5k, `src/domain` 11.9k, `src/core` 10.9k, `src/ai` 10.6k,
`src/cross_domain` 9.3k, `src/network` 9.2k, `src/registry` 8.9k,
`src/rpc` 8.8k, `src/consensus` 5.3k, `src/execution` 4.1k,
`budzero/` ayrı workspace, `crates/` altında ayrı paketler.
BudFly: +2.317 satır, `crates/budfly` altında tamamen izole. 300k+ satırın
tamamının satır-satır denetimi bu belgenin iddiası değildir; yapılan yapısal
geçiştir (modül sınırları, rol mekanikleri, kapı konvansiyonları).

## 2. Mimari çapraz-eşleme: BudFly bugün nerede durur, yarın nereye bağlanır

| Budlum parçası | BudFly ilişkisi | Durum |
|---|---|---|
| `budzero/verifier-registry` | SENTINEL rolünün stake/slash/evidence primitifi **hazır ve genel** | teklif (bölüm 4) |
| `src/execution/` (executor) | Anchor kayıtları transaksiyon verisi olarak taşınabilir (fails-closed: anchor yoksa kayıt yok) | teklif, kod yok |
| `budzero/bud-vm` + `bud-proof` | C1–C7 AIR taslağı STARK'a düşürülecek hedef tanım; 1-tick gecikme kararı bu yüzden | taslağı hazır, aritmetizasyon yok |
| `src/registry` (8.9k) | Rol kayıt yüzeyi; açık `RoleId` sayesinde değişiklik gerektirmez | hazır |
| `src/ai_inference` kapısı | BudFly'ın duruşu bu kapının kopyası: "anlam deneysel, anchor gerçek" ≈ "unverified inference fails closed" | uyum, bağımlılık yok |
| `src/consensus` | **Temas yok.** Bilinçli: verdict asla konsensüs koşulu değil | — |
| `ci.yml` / determinizim kapıları | `budfly.yml` workflow'u aynı konvansiyonla eklendi | yapıldı |

## 3. "Lubot'ta iş görür mü?" — dürüst cevap

Bu repoda Lubot bir **çalışma zamanı değil**: kanonik prompt
(`docs/LUBOT-SYSTEM-PROMPT.md`) + yetenek izolasyonu, denetim/audit
crate'leri, kanıt kilidi, mühür ve ratchet konulu patch serileri
(`repo-lubot/patches/0001..0019`, `repo-workspace/`). Yani BudFly "Lubot'un
içinde çalışan kod" olamaz — burada koşan bir Lubot yok. Ama üç yerde
doğrudan iş görür:

1. **Referans disiplin.** Lubot patch serisinin konusu neyse BudFly onun
   çalışan örneği: donmuş jeneratör = *ratchet*; golden anchor'lar = *mühür*
   (mühür sonradan düzeltilirse iz bırakır — patch 0012'nin kuralı); AIR
   denetçisi = *kanıt kapsama kilidi* (patch 0008). Lubot araç zinciri bir
   "nasıl yapılır" ararsa, cevap bu crate'tir.
2. **Kanıt taşıyan iş yükü.** Agent yüzeyleri (Lubot Code tarzı) için ideal
   görev tipi: çıktısı üçüncü tarafça ms maliyetle doğrulanabilen çalıştırma
   makbuzları üretmek. BudFly anchor zinciri tam olarak bu makbuz formatıdır.
3. **Determinizim armatürü.** `repo-lubot/tools/rebuild_series.py` gibi patch
   yeniden-kurma araçlarının ihtiyaç duyduğu şey deterministik yeniden-üretim;
   BudFly'ın Python↔Rust bit-özdeş golden'ları bu alışkanlığın test
   fikstürüdür.

## 4. Oylamasız validator: SENTINEL rolü teklifi

**Mekanik hazır.** `budzero/verifier-registry` dokümanında açıkça yazar:
"*`RoleId` is a u32 newtype, not an enum. New roles can be introduced by
Callers without modifying this crate*" — ve `VALIDATOR_ROLES.md` üçüncü bir
rolü (`AI_OPERATOR`) "asla blok üretmeden" kaydedilebilir diye tanımlar.
Bilinen rol ID'leri 1–9 dolu; `Content validator` RoleId(9)'dır.

**Teklif:** `SENTINEL = RoleId(10)` — uygulama katmanında `RoleId::new(10)`
ile başlar, registry'ye dokunmadan kullanılır; ileride well-known sabitler
listesine terfi etmesi ayrı bir yönetişim kararıdır (bu PR'da **yapılmadı**).

**Rolün görev çevrimi:**

```
digest girişleri (domain finality payload hash'leri, AI outcome commitment'ları…)
   │
   ▼  sentinel_verdict(): 48 tick anchor'lı transkript
SENTINEL yeniden yürütür ──► anchor eşleşiyor mu?
   │ evet: sessiz onay (periyodik liveness kanıtı)
   │ hayır: SlashingProof::Other { tag: "budfly-anchor-mismatch", … }
   │        ProofProvenance::Unverified olarak girer; registry kuralına uygun:
   │        doğrulanmadan slash yok. Doğrulama = trace'in deterministik
   │        yeniden yürütülmesi (milisaniye maliyet).
```

**Neden bu rol "yapısı gereği slash-lanabilir":** deterministic execution
olmadan "yanlış bildirdi" kanıtlanamaz; BudFly'ın bit-özdeşliği (golden
anchor'lar) "anchor ya uyar ya uymaz" önermesini nesnel bir olguya çevirir.
Konsensüs stake'iyle hiçbir ilişkisi yok: kendi bond'u, kendi koşulları
(`LivenessFault` sessizlikte, `MaliciousBehaviour` asılsız ihbarda) —
`VALIDATOR_ROLES.md`'nin ayrı stake gerekçesiyle birebir aynı mantık.

**Ne değildir:** blok üretmez, oy vermez, finality'yi etkilemez, verdict
anlambilimini onaylamaz. Konsensüse dokunmaz — repo kapılarının da kuralıdır
(admin path yok, pause hook yok).

## 5. Bu PR'da eklenenler

- `crates/budfly` (önceki iş): jeneratör, sim, AIR, fabric, oracle, golden'lar.
- `.github/workflows/budfly.yml`: fmt + clippy (-D warnings) + testler +
  örnek çalıştırma + Python referans doğrulaması + soğuk-runner determinizm
  duman testi. Amaç: CI, bu crate'i her PR'da derleyip altın anchor'ları
  üretir — derleme ve doğrulama artık projenin kendi altyapısında kanıtlanır.
- Bu belge. Gelecek aday işler (PR dışı): SENTINEL kayıt/ücret
  parametreleri, `SlashingProof::Other(tag="budfly-anchor-mismatch")`'i
  registry rapor yoluna bağlayan veri şeması, BudZero AIR düşürme.
