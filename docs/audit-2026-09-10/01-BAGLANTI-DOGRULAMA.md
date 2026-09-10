# 01 — Bağlantı Doğrulama (2026-09-10, HEAD `88970e3`)

Charter §2 iddialarının canlı-ağaç kanıtı. Her satır dosya:satır ile
desteklenir; özet makine-kontrollü karşılığı:
`tools/audit/wiring_check.py`.

## B.U.D. (erasure / placement / repair)

| yol | durum | kanıt |
|---|---|---|
| encode (`encode_object`, `encode_parity`) | **UNWIRED** | Üretim çağrısı yok (tek `encode_object` kullanımı `src/storage/manifest.rs:1143` testinde). |
| reconstruct (`reconstruct_object`, `reconstruct`) | **UNWIRED** | Üretim çağrısı yok. |
| `to_manifest` | **UNWIRED** | Üretim çağrısı yok (`EncodedObject` üretimde hiç inşa edilmiyor — tutarlı). |
| coding audit (`for_scheme` + `parity_coefficient` + `column_is_correctly_encoded`) | **BAĞLI** | `src/domain/storage_deal.rs:2177-2189` (`verify_coding_audit`, `pub`). |
| verify RPC (`verify_object_encoding`) | **BAĞLI** | `src/rpc/server.rs:2293` (`verify_encoding` handler). |
| placement (`assign_shard`) | **BAĞLI** | `src/domain/storage_deal.rs:3077`. |
| placement (`assign_object`) | **UNWIRED** | Re-export dışında çağrı yok. |
| repair trigger | **EKSİK** | Üretim implementasyonu/çağrısı bulunamadı; tek eşleşme `src/storage/manifest.rs:1302` test adı. |

Sonuç: charter'daki "UNWIRED" hükmü **kısmi-doğrulandı ve düzeltildi**:
kodlama+onarım yolu bağlı değil; denetim/doğrulama yolu bağlı
(`verify_coding_audit` + `verify_encoding` RPC). Yani B.U.D. "hiç bağlı
değil" değil — "yazma/onarım bağlı değil, okuma/doğrulama bağlı".
Checker (`tools/audit/wiring_check.py`) bu tabloyu makine-ölçümü olarak
kilitler: encode tarafında yeni çağrı FAIL, verify tarafında çağrı kaybı
FAIL. Not: ilk el-taraması `verify_coding_audit` ve `verify_encoding`
bağlantılarını kaçırmıştı; checker yakaladı, tablo düzeltildi (ölçüm
disiplini çalışıyor). Kuyruk: `validate_untrusted` üretim akışı
(`03-SIRADAKI-IS.md` #2), R&D: encode yolu tasarım kararı.

## BudZKVM — VerifyMerkle

**GATED-BY-DESIGN (güvenli).** İşlem yolu (`execute_bytecode`,
`src/execution/zkvm.rs:79-86`) her zaman gate'li decode kullanır
(`execute_bytecode_inner(..., mainnet=true)`); gate'siz çalıştırma
yalnızca `#[cfg(test)]` (`execute_bytecode_ungated`). Gerekçe kodda
yazılı (`zkvm.rs:60-73`): gate ağ özelliği değil, "doğrulama bitmedi"
işaretidir ve hiçbir ağda bitmiş değildir.

Dokümantasyon tutarlı:

- `budzero/docs/BudL_SPEC.md:410-412` — "VerifyMerkle soundness",
  `gated off on mainnet`.
- `src/storage/README.md:25` — "production gate (closed)".
- `zkvm.rs:181,412-485` — gate testleri (`verify_merkle_enabled=false`
  assertion'ı dahil).

Açık nokta (harici): dış opcode denetimi + STARK/AIR tarafının bağımsız
doğrulanması. Turun web araştırması bu noktaya bağlandı: Sigma Prime'ın
zkVM denetçi kılavuzu, Merkle-içerme kanıtının tek başına yetmediğini,
kanıtın bağlandığı kökün (örn. blok hash) **doğrulayıcı tarafta harici
olarak** tazelik/geçerlilik kontrolünden geçmesi gerektiğini vurgular
[2](https://blog.sigmaprime.io/sp1-zkvm-security-guide.html); genel
zkVM inşa/doğrulama çerçevesi için bkz.
[1](https://eprint.iacr.org/2023/1032.pdf). VerifyMerkle prod'a
alınırken verifier-tarafı bağlam kontrolü şart koşulmalı (R&D maddesi).

## Lubot (çekirdek tarafı)

Budlum çekirdeğinde `lubot` referansı **0 dosya**
(`src`+`crates`+`bud`+`budzero` taraması). `src/lubot` yok — önceki
oturum logundaki "src/lubot sapması" bu HEAD'de de geçerli. Lubot
çalışması ayrı repo aynasından yürür (`repo-lubot/`).

## AiRegistry / AccessGrant

Mevcut: `src/ai/registry.rs`, `src/ai_inference/{executor,inference,mod}.rs`,
`src/chain/chain_actor.rs`. 4 çağrı zincirinin bu HEAD'de yeniden
doğrulanması kuyrukta (`03-SIRADAKI-IS.md` #3).

## Kanıt standardı notu

"Tamamlandı" iddiası yok; yukarıdakiler ham bulgu + kod kanıtıdır.
CI koşu numarası bu dalın PR'ı açılınca eklenecek.
