# 07 — Web Araştırma Günlüğü (2026-09-10+)

Charter §5: her kaynak bir açık noktaya bağlanır. (İlk kayıt olan
Sigma Prime zkVM kılavuzu 01'dedir.)

## 2026-09-10 — HPKE (RFC 9180) → Pollen HPKE açık noktası

Kod durumu (önce okundu): HPKE adı kodda geçiyor
(`src/pollen/data_rights.rs`, `src/pollen/offers.rs`,
`src/pollen/README.md`, `src/ai/README.md`,
`src/core/governance.rs`) — gerçek kullanım mı işaret mi, taranacak
(kuyruk). Mevcut yük şifreleme: `payload_crypt.rs`
(`seal_payload_csprng`/`open_payload`, anahtar `PayloadKey::derive`).

Kaynaklar:

- `rozbb/rust-hpke`: RFC 9180 uygulaması; RESMİ DENETİMSİZ
  (Cloudflare 0.8 iç-gözden geçirmesi temiz)
  [1](https://github.com/rozbb/rust-hpke). PQ/hibrit test vektörleri
  var; kripto-esneklik (agility) yok.
- `hpke-dispatch`: base-mode tek-atımlık zarf için çalışma-anı
  dağıtımlı sarmalayıcı
  [2](https://docs.rs/hpke-dispatch/latest/hpke_dispatch/).
- HPKE açıklayıcısı: akış (streaming) formu nonce-yeniden-kullanım
  tuzağı; sarmalayıcılar RFC limitlerine göre denetlenmeli
  [3](https://havenmessenger.com/blog/posts/hpke-hybrid-public-key-encryption-explained/).

Değerlendirme (ham bulgu, karar değil): "HPKE zorunlu uygulaması"
için tek-atımlık (single-shot) zarf + `rust-hpke` adayı; akış formu
YASAKLANMALI (nonce riski); denetimsizlik R&D risk kaydı. Karar
R&D'ye taşınacak (seçenekler + öneriyle).
