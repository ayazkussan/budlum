# 08 — Büyük Patch Triyajı (2026-09-10)

Kaynak: fork `main` HEAD `d07225d` ("Add files via upload"):
`01a08674-... (2).patch` — 49.974 satır / 421 dosya / 2,3MB.
Önceki session (01a08674) kümülatif ihracı.

## Yöntem

- `/tmp/apply` kopyasında `git apply --reject`: 26 dosya +1077/−2
  uygulandı, 364 `.rej`.
- İçerik-blok doğrulaması (her `+` bloğu canlı ağaçta arandı): 24/26
  dosya MEVCUT (fuzz ile çift-uygulanmış gürültü), 2 dosya karışık.
- Ara "zehir" alarmı (duplicate impl'ler) GERİ ÇEKİLDİ: tamamı
  canlıda mevcut içeriğin fuzz artefaktıydı. Kural pekişti: el-delta
  ile hüküm verilmez, içerik aranır.

## Gerçekten yeni olan (tek kalem)

- `RelayCompletion` varyantı (`transaction.rs`) + executor kolu
  (fee/nonce yarısı). StateUpdate kolu canlıda birebir mevcut
  (C3 sürümü, `executor.rs:2263`).

## Soğurulmadı (eksik set)

E-7 zincir-tamamlama için eksikler:

1. `Blockchain::apply_relay_completions` hiçbir yerde yok (kol
   yalnızca fee alır; soğurulsa kullanıcı boşuna öder).
2. `proto_conversions.rs` kolları yok (StateUpdate 5 yerde eşleşiyor;
   varyant ağa serileşemez).
3. `signing_hash`/`verify` varyant-agnostik görünüyor (eşleşme yok) —
   teyit kuyrukta.

→ `03` #13: E-7 tamamlama (tasarım + proto + testler; ekonomi R&D).

## Sonuç

Patch'in %99,9'u ağaçta mevcut. Yeni olan tek kalem eksik-set
olduğundan bu tur soğurma YOK; kör `git am` reddedildi (gerekçeli).
