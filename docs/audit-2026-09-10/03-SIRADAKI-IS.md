# 03 — Sıradaki İş Kuyruğu (2026-09-10)

Öncelik sırasına göre. `ask_user` = mimari/ekonomik/kapsam kararı
gerektirir (toplu sorulur, ekran yoksa PR yorumlarında birikir);
diğerleri act-and-log.

1. **R&D (`ask_user`): B.U.D. encode yolu tasarım kararı.** Placement
   (`assign_shard`) bağlı; encode/repair değil. Seçenekler: (a) deal
   yazma yoluna `encode_object` bağlama, (b) arka-plan kodlama işçisi,
   (c) kapsam-dışı bırakıp charter bitiş kriterini revize etme.
2. `to_manifest` / `validate_untrusted` üretim akışı doğrulaması
   (manifest → deal → placement zinciri).
3. AiRegistry → Pollen AccessGrant → BNS → B.U.D. zincirleri.
   → Hüküm `05-ZINCIRLER.md` + DÜZELTME: L1 BAĞLI (apply-time),
   L2 BAĞLI, L3 zayıf (tip-seviyesi), L4 BAĞLI. Kalan mikro-iz:
   `ai_registry` kurulum yeri.
4. Deneme tabakalı örneklemesi → dilim 1 tamam (`06`): #493/#464,
   0 review. Sonraki: her parçadan 1 PR meta+review+CI.
5. **R&D (`ask_user`): headroom/arcbox vendoring stratejisi.**
   `headroom@4e1f7769` ≈35MB metin (Apache-2.0),
   `arcbox@55b384b9` ≈13MB metin (MIT+Apache). Tam-vendor patch
   boyutu yönetilemez; seçenekler: (a) katmanlı vendor
   (belgeler+bildirimler+çekirdek önce), (b) desen-çıkarım notu +
   seçili dosya aynası. Lisans dosyaları her durumda korunur.
6. Lubot: rebase (191 test / 38 gate iddiası; 38. kapı `check.py`
   koşusunda OK) + 0006 gate-runner izolasyonu (cargo'suz koşuda
   abort→FAIL+devam; kanıtlı). **R&D (`ask_user`): lubot reposunda
   `.github` (CI) yok** — CI ekleme veya `gates/check.py` + ölçüm
   disipliniyle devam.
7. Seed "bulgu-dump" taraması (denetçi bulgusunun kodsuz gömülmesi).
8. Bu dalın PR'ında fork CI koşusu (fmt + clippy `-D warnings` + test)
   ve sonuçların kanıt standardına işlenmesi.
9. VerifyMerkle prod-açılış önkoşulları: dış opcode denetimi +
   verifier-tarafı bağlam kontrolü (01'daki araştırma notu).
10. Post-merge otomasyon v2: upstream'da issue'lar açık (şu an boş);
    yazma yetkisi denenmedi. Cron main'e girince issue-günlüğü adayı.
11. **R&D (`ask_user`): AccessGrant↔ViewGrant köprüsü.** İki yönde de
    çapraz referans YOK (kanıtlı). AI-okuma Pollen grant'iyle, storage
    okuma ViewGrant'le korunuyor; birleşik "grant-kapılı bayt" akışı
    için köprü tasarımı gerekli. Seçenekler: (a) tek sisteme
    indirgeme, (b) çift-kontrol adaptörü, (c) kapsam-dışı + charter
    revizyonu.
12. HPKE: kodda adı geçen 5 dosyanın taraması (gerçek kullanım mı
    işaret mi) + "HPKE zorunlu uygulaması" R&D'sine girdi (`07`).
13. **E-7 tamamlama (kısmen R&D):** `RelayCompletion` varyant+kol
    patch'te hazır ama eksik-set (geçiş fn'ı yok, proto kolları yok).
    Tasarım + proto + testler; mint/credit ekonomisi R&D kararı.
    Kaynak: `08`.
