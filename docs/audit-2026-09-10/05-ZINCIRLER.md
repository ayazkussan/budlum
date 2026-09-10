# 05 — AiRegistry/Pollen/BNS/B.U.D. Zincir Hükümleri (2026-09-10, HEAD `88970e3`)

Önceki log "4 zincir BAĞLI" diyordu; bu HEAD'de yeniden doğrulandı.
Sonuç: 1 bağlı, 1 kısmi, 1 zayıf, 1 bağlı-değil.

## L1 — inference executor → Pollen grant: KISMİ (verify yok)

- `build_ai_transaction(..., grant: &AccessGrant, ...)`
  (`src/ai_inference/executor.rs:56-77`) grant'i alıp
  `inference::build_ai_request`'e iletiyor (:77); ikinci kurucu
  `AccessGrant::new_unsigned` üretiyor (:100-122).
- executor.rs + inference.rs prod bölgesinde (1-122) grant doğrulama
  çağrısı YOK (`verify`/`expir`/`revoke` izi yok).
- **Doc-code gap**: executor modül dokümanı "(1) Pollen grant
  verification" diyor (`executor.rs:42-46`); kodda karşılığı yok.
- Zincir-uygulama tarafında grant mekanizması genel olarak kontrollü
  (`src/execution/executor.rs:1674` owner-check + 1679/1690 create/revoke).
- Kuyruk: AI-tx apply yolunda grant kontrolü (execution →
  AiInferenceRequest izi) + AccessGrant↔ViewGrant köprüsü (aşağıda).

## L2 — AiRegistry zincir kullanımı: BAĞLI DEĞİL (UNWIRED)

- `AiRegistry` API'si tam (`src/ai/registry.rs`: register/submit/dispute...).
- `registry.rs` dışındaki TÜM kullanımlar testlerde: `src/ai/mod.rs`
  (testler :33'ten başlar), `src/ai_inference/mod.rs` (:343'ten),
  `src/ai_inference/inference.rs` (:123'ten). `src/ai/` dışında prod
  `AiRegistry::new`/`submit_*` çağrısı YOK.
- Yani registry kodlanmış + testli ama zincir/executor/RPC hiçbir
  yerden çağırmıyor. Önceki "BAĞLI" hükmü bu HEAD'de GEÇERSİZ.

## L3 — BNS okuma yolunda: ZAYIF (tip-seviyesi)

- `src/gateway/passport.rs`: `BnsResolved` tipini taşır (:18, :287),
  alanlarını okur (`owner`, `content_id`, `storage_root`, `is_expired`;
  :320-363), bir yerde kendisi inşa eder (:442).
- BNS çözümleme çağrısı YOK (`bns::` fonksiyon çağrısı yok).
- Passport, gateway'den ihraç ediliyor (`gateway/mod.rs:10-13`); RPC
  katmanında passport paket izleri var (`rpc/api.rs:719,726`).
- Kuyruk: passport çağrı zinciri (RPC → gateway) + gerçek
  çözümleyicinin nerede çağrıldığı.

## L4 — grant yaşam döngüsü + storage: BAĞLI

- `chain_actor`: `issue_view_grant` (:1029, çağrı :3896),
  `revoke_view_grant` (:1055, çağrı :3914), `view_grants_for` +
  `live_view_grant_count` (:3941-42).
- `storage_deal`: `ViewGrantRegistry` durum alanı (:725), state-hash'e
  girer (:1146-47), issue/revoke gerçeklemeleri (:1543-1604).
- Grant servis tarafı gerçek durumu okur: `PollenGetAccessGrants`
  handler'ı `marketplace.access_grants`'ı döner
  (`chain_actor.rs:4530-38`).

## Keskin soru (kuyruk)

İki grant sistemi paralel görünüyor: Pollen `AccessGrant` (AI yolu) ve
storage `ViewGrant` (depolama yolu). AI-okuma için grant-kapılı bayt
akışı bu ikisi arasında köprü ister; köprü araması:
`AccessGrant` ↔ `ViewGrant` çapraz referansları. Bulunamazsa bu bir
mimari R&D maddesidir (`03`'e eklenecek).

Ölçüm v2 adayı: L2/L4 ratchet'leri `wiring_check.py`'ye taşınacak.

## DÜZELTME (2026-09-10, aynı gün)

L1/L2 hükümleri el-grep hatasıyla verilmişti (BRE `\(` tuzağı:
`\.submit_request\(` kalıbı çağrıları tutmadı). Apply-yolu okumasıyla
düzeltildi:

- **L2 → BAĞLI**: `state.ai_registry.submit_request(...)`
  (`src/execution/executor.rs:1379-81`) + prod admission gate'i
  `admit_inference_request` (`src/ai_inference/mod.rs:112`, çağrı
  :1373). Registry state alanı olarak taşınıyor
  (`blockchain.rs:259`, `chain_actor.rs:2999+` okumaları); kurulum
  izi (`Default`/literal?) kuyrukta (tek-satırlık grep).
- **L1 → BAĞLI (apply-time)**: `marketplace.validate_ai_read_ref` +
  `consume_ai_read_grant` (:1375-86), fail-closed
  (`ai_data_access_denied`). Executor dokümanındaki "(1) Pollen grant
  verification" ifadesi sistem seviyesinde DOĞRU; yaptırım
  build-anında değil apply-anında (katman farkı, gap değil).
- Metodoloji kuralı (bu turdan itibaren): el-grep ile "yok" hükmü
  verilmez; yok-hükmü checker'a yazılır ya da apply-yolu okunur.
