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

## Varyantlar (yedek iddialar)

- Koltuk sayısı 3→N (maliyet anchor başına lineer ~2,3 µJ/tick başına; pinler 3'te donmuş).
- Panel turnuvası yerine ağırlıklı lig (çok-digest skor tablosu).
- ~~BSE-2 zarf varyantı~~ **SEVK EDİLDİ**: fork kartı zarfa girdi (204 B, C11–C13).
- zk-ona indirgeme: C1–C7 kısıtları (`air.rs`) zincir-dışı kanıtla zarfı 32 B'a bastırır (yol haritası, iddia edilmiyor).

## Dürüst-negatifler (icattan düşülmeyenler)

- Bump-wander kilitlemiyor (oturmuş attractor yok) — DRAM forensics kanalına çevrildi.
- APL lezyonu probing v1'de davranışsal olarak sıfır — v2 gerekir.
- "MDN@10" senaryosu verdict'i çevirmedi — metrik izin verilen senaryoyu SINIRLANDIRIYOR, abartmıyoruz.
- Verdict semantiği "sinek karar verdi" iddiası taşımaz; taşınan şey anchor'dır.
