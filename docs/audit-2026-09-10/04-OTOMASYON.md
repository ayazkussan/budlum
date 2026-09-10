# 04 — Otomasyon (2026-09-10)

Kullanıcı kararı: tur planlaması yok; iş otomasyona bağlı, ekran-komut
beklenmez. Bu dosya döngünün nasıl döndüğünü tanımlar.

## İki katman

1. **Mekanik katman** (`.github/workflows/surekli-denetim.yml`): cron
   6 saatte bir + `workflow_dispatch` + bootstrap push tetikleyici.
   Sıfır üçüncü-taraf action (checkout dahil; `git clone` + sistem
   `python3`/`gh` kullanılır), toolchain gerektirmez:
   `wiring_check.py` çalıştırır, sonucu koşu özetine yazar, dalda açık
   PR varsa oraya yorum atar. Secret yok. Fork'ta issue'lar kapalı
   olduğundan issue günlüğü kullanılmaz; kalıcı kayıt = koşu özetleri
   + PR yorumları.
2. **Ajan katmanı** (bu oturum): kod okuma, bulgu, kodlama, commit+push.
   Her push, bootstrap tetikleyiciyle ölçümü ateşler (kanıt zinciri);
   main sonrası cron+dispatch devralır.

## Sınırlar (dürüst notlar, kanıtlı)

- GitHub'da cron VE `workflow_dispatch`, workflow tanımını **yalnızca
  varsayılan daldan** okur: dosya `main`'de yokken dispatch 404 verdi
  (2026-09-10, kanıtlandı). O zamana kadar tetik = session dalına
  push (bootstrap bloğu; merge'de kaldırılacak).
- Fork'larda 60 gün hareketsizlikte cron otomatik kapanır; repo aktif
  olduğu sürece sorun değil.
- Rust adımları (fmt/clippy/test) v1'de YOK: sandbox'ta toolchain yok,
  pin (`1.97.1`) ile gerçeklik arasındaki durum PR CI sonuçlarıyla
  netleşecek (kuyruk #8). Rust işleri mevcut 23 workflow'a emanet.

## Kesinti kuralı (ekran yok)

- R&D/mimari kararlar `ask_user` UI yerine PR yorumlarında + `03`'te
  birikir (madde + seçenekler + öneri).
- Geri-dönüşü olmayan karar (ekonomi, lisans, kapsam çatışması):
  ilgili akış `03`'te BLOKLU işaretlenir, **diğer akışlar sürer**.
- `ask_user`: yalnızca tüm akışlar bloklanırsa (tam tıkanma).

## Ajan turu devam protokolü

1. `03-SIRADAKI-IS.md` + PR yorumları + PR diff özeti.
2. Sıradaki kuyruk maddesi: kod → ölçüm → commit → push (PR birikir,
   push aynı anda ölçümü ateşler).
3. Bir sonraki turda koşu sonucunu kanıta işle.
