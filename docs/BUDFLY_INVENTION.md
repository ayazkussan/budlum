# BULUŞ BİLDİRİMİ — "Mahkeme Omurgası" (Fly Court Spine)

> Tarih: 2026-09-17 · Depo: ayazkussan/budlum, dal `arena/01a0aa03-budlum`
> Bu belge bir patent başvurusu değil, icat envanteridir: her iddia satırının
> altında CI'da donmuş (golden-pinned) bir test yatar. İddia boş çıkarsa
> alarm çalar — bildirim ile ürün aynı komuttan beslenir.

## Tek cümlelik icat

Donmuş (golden-pinned) bir sinek-beyni bağlantı haritasını **kanıta dayalı
yargı makinesi** olarak kullanmak: aynı donmuş dinamikler hem meydan-okuma
cevabı (bütünlük kalp atışı), hem jüri (tuzlu probelar), hem de tahkim
hakemi (biseksiyon) görevi görür; mahkemenin kararı **136 baytlık tek
zarfta** mühürlenir.

## Neden yeni (önce-sanat ayrımı)

| Var olan | Bizim farkımız |
|---|---|
| FlyCoder: sinek konektomunu "div ortalamaya" koşturur | Biz konektomu **oy kullanmayan karar mekanizması** yapıyoruz: ürün tahmin değil, mühürlü transkript |
| Klasik fraud-proof: aynı programı iki kez koştur, ilk sapmayı böl | Aynısı + zincire künyelenmiş **yapay-sinir devresi tanığı**: tanık sabit, jüri tuzlanmış, karar oybirliği-şartıyla kapanıyor |
| Firmware canary: uzun self-test, aylar | 1 tick ≈ tek SHA-fold = **32 bayt/tick** kalp atışı; ucuz uç pahalı sözleşmeye (sentinel golden'a) birebir çıkıyor |
| Quorum imza: kriptografik eşik | **Davranışsal eşik**: üç ameliyat-edilmiş probe aynı olgu üzerinde anlaşmazsa kayıt `Abstain` — gürültü değil, karar kuralı |

## İddialar (claim'ler) ve donmuş kanıtları

| # | İddia | Kanıt (test/pin) |
|---|---|---|
| C1 | Donmuş spike dinamiği her koşuda aynı 48-başlı zinciri üretir | `[tamper]`, sentinel golden `398e126a…`, cross-language (Rust=Python) |
| C2 | 1-tick canary challenge→response çipi kendi sözleşmesine oturur | `canary.rs`, `canary.self_consistent=True` |
| C3 | Koltuk-tuzlu jüri (`sha256(digest‖seat)`) RNG çekim disiplinini bozmadan üç bağımsız probe üretir | `divan.seat{0,1,2}_anchor` pin'leri |
| C4 | Oybirliği-olmazsa-kapan kuralı gerçek veride tetiklenir | `[Abstain, Affirm, Abstain]` → council=Abstain (pinli) |
| C5 | Tembel jüri ihraç edilir; tahkim yeniden mühürler | divergence=23, honest_final == seat2 anchor |
| C6 | Kontrgerçek dallar prefix-bağlı mühürlenir | `replay.prefix_bound=True`, "MDN@10" dalı `62cfbd0f…` |
| C7 | Erken-veto dayanıklılığı ölçülebilir | verdict Affirm değişmedi (sayaç 7,6,5) — dürüst-negatif çıktı |
| C8 | Donanım seçimi kararlılık metriğiyle turnuvalanır | panel 8, A=7 B=5 abstain → kazanan B |
| C9 | Mahkeme kararı 136 B zarfa sığar; ucuz tutarlılık denetimi sıfır nöral yürütme | `BSE1…00010000` hex, `cheap_consistent=True`, sahtesi reddedilir |
| C10 | Bütünü sıfır-bağımlılık, clippy-unwrap yok, fails-closed | CI gate, API yüzeyi |
| C11 | Karar ve kontrgerçek kartı tek zarfta (204 B, "BSE2") | `envelope2.len=204`, frozen hex, BSE-1 = fork==0 alt-durumu |
| C12 | Ucuz replay denetimi tek 32 B byte-compare (yayınlanmış baş ile) | `cheap_replay_consistent` forged-head reddi (pinli) |
| C13 | Forksuz kart byte'ları sıfır; kartlı fork'suz kabul edilmez | `decode2` garbaje `None`, pin `envelope2.nocard_tail` |
| C14 | Tek-tick dolandırıcılık kanıtı connectome'suz yargılanır (ACT-1 bant, ~24,7 KB) | `tape.rs`, pinli byte-uzunluk 24708 + sha256 |
| C15 | Çift bağımsız kilit: kısıt katili (C1–C7, 192 ihlal) + zincir katili (fold uyumsuz) | iki pin, ikisi de bağımsız ateşler |
| C16 | Genesis tick-anahtarlıdır; trace PENCERELERİ genesis'a takılmaz | `air.rs` C7 tick==0 refaktörü, mevcut testler değişmeden yeşil |
| C17 | (peer, epoch, nonce)-bağlı davranışsal build-parmakizi el sıkışması | `attest.rs`, frozen challenge + L1/L2 pin'leri |
| C18 | Merdiven iç tutarlılığı: L1 her zaman L2 zincirinin ilk başı | `attest.l1_prefix_of_l2=True`, `ladder_consistent` |
| C19 | Epoch-freshness: eski epoch cevabı taze challenge'da geçmez | `attest.epoch_moves_l2=True` |
| C20 | Sezonluk sıralama tablosu: ayak-bağlama kuralı dahil hakem imzası taşır | `league.rs`, standing pinleri (çaylak şampiyon 0xB0DF1A) |
| C21 | Turnuva şampiyonu sezonda tahtını koruyamaz; metrik tutarlılığı sürer | 0xB0DF18 saltanatı, tie-break'te B0DF19 kırılıyor (pinli) |
| C22 | Yürüyen kanıtlayıcı protokolü (erasure): root-bağlı chunk challenge'ları | `erasure.rs`, K=8 cevap pinleri |
| C23 | Tek-bit avalansı: 1 chunk bit'i => root kayar => 8/8 cevap değişir | `erasure.bitflip_moves_all=True` |
| C24 | Dürüst-negatif sabitlenmiş iddia sınırı: kanıt posesyon DEĞİLDİR (v1) | `erasure.proves_possession=False` (geri alınamaz pin) |
| C25 | Devre test-vektörleri kanonik: ikili ACT-1 bant + yeniden-üretim kapısı | fixtures/ + `dump_fixtures.py`, sözleşme `budfly-fixtures-v1` |
| C26 | ACT-1 hasar taraması kapsamlı: 24.708 tek-bit mutant + tüm kesmeler; kaçış envanteri 396 offset'in tamamı "hayalet refraktör" sınıfında (`f16:rb0sp0`), offset listesi hash-mühürlü; spike/v_after/r-zinciri/fold %100 delici | `hardening.rs::tape_adversary_sweep`, `hard.tape.*` pinleri (8+24.300+4+396) |
| C27 | Erasure avalansı tam uzayda kanıtlı: 32/32 chunk pozisyonu K=8 cevabın tamamını kaydırır; epoch cevapları 8/8 hareket eder | `hard.erasure.chunk_avalanche=32`, `hard.erasure.epoch_moves=8` |
| C28 | Kimlik ve lig bağışıklığı: nonce avalansı 32/32, merdiven kendi kendini 32/32 yeniden başlatır, eş binding; lig replay/shuffle/tie/total-order invaryantları | `hard.attest.*`, `hard.league.*` pinleri (32 kurul, 20 abstain bütçesi) |

## Varyantlar (yedek iddialar)

- Koltuk sayısı 3→N (maliyet anchor başına lineer ~2,3 µJ/tick başına; pinler 3'te donmuş).
- Panel turnuvası yerine ağırlıklı lig (çok-digest skor tablosu).
- ~~BSE-2 zarf varyantı~~ **SEVK EDİLDİ**: fork kartı zarfa girdi (204 B, C11–C13).
- ~~zk-ona indirgeme~~ **İLK DİREK DİKİLDİ**: ACT-1 bant = STARK'sız çalışan sürüm (C14–C16); zk bunu 32 B'a bastırır (yol haritası, artık somut).

## Dürüst-negatifler (icattan düşülmeyenler)

- Bump-wander kilitlemiyor (oturmuş attractor yok) — DRAM forensics kanalına çevrildi.
- APL lezyonu probing v1'de davranışsal olarak sıfır — v2 gerekir.
- "MDN@10" senaryosu verdict'i çevirmedi — metrik izin verilen senaryoyu SINIRLANDIRIYOR, abartmıyoruz.
- Verdict semantiği "sinek karar verdi" iddiası taşımaz; taşınan şey anchor'dır.
