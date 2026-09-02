# Yol haritası

Açık plan. Adımlar tamamlandıkça `ilerleme.md`'ye taşınır, buradan düşer.

## Etkin plan — davranış yeniden tasarımı (7 adım)

Kullanıcının bildirdiği 6 kök problemi çözer. Her adım: commit + push + bu dosyanın güncellenmesi.

### Adım 0 · dev/ klasörü — TAMAMLANDI
### Adım 1 · Log sadeleştirme — TAMAMLANDI
### Adım 2 · 12 mesaj sınırı kalksın — TAMAMLANDI
`SOHBET_ZAMAN_ASIMI` 30 dk, `zaman_asimi_kapat` uyku tikinde, kanal yasağı yok.
### Adım 3 · Zihin id bazlı + zaman damgası + bellek döngüsü — TAMAMLANDI
`kisiler/<id>.md`, `ad_id` çözümlemesi, `tarih_saat()`, `bellek_dongusu` kuyruk işleme.
### Adım 4 · Cevap istekliliği — TAMAMLANDI
Mini model çağrısı (`isteklilik.md`), eşik 6 (evre ±1, seyahat +2), 2 dk rate limit, yedek zar.
### Adım 5 · Hedef kişi seçimi + Eski sil-baştan kalktı — TAMAMLANDI
`son_gelenler` + `hedef_sec`; akış artık yeni mesajda silinmiyor, sıradaki turda ele alınıyor.
### Adım 3 · Zihin id bazlı + zaman damgası + bellek döngüsü
- `kisiler/<id>.md`: id, kullanici_adi, gorunen_ad, eski_adlar + mevcut alanlar.
  Temiz başlangıç (eski slug dosyalarına dokunulmaz).
- `Durum.ad_id` eşlemesi; günlükçü dökümüne `KATILIMCILAR: ad=id` bloğu; JSON isim→id kodda çevrilir.
- `tarih_saat()` (YYYY-AA-GG SS:DD:SS) tüm kayıtlarda.
- `bellek_dongusu` (10 dk): sohbet sonu ajanları inline değil, kuyruktan işlenir
  (günlükçü → özetleyici → eleştirmen). Uyku kontrolüne takılmaz.
### Adım 4 · Cevap istekliliği
Etiket/yanıt/ad her zaman. Diğerleri: mini model çağrısı (~50 token, `isteklilik.md`)
→ `{"puan":0-10,"sebep"}`; eşik üstüyse girer. Kanal başına en sık 2 dk'da bir; açık sohbette çağrı yok.
### Adım 5 · Hedef kişi seçimi + Eski kalksın
- Sil-baştan (Eski) kaldırılır; akış tamamlanır, yeni mesaj sıradaki turda.
- Bot sustuğundan beri 2+ kişi yazdıysa mini hedef çağrısı (`hedef-sec.md`) kime dönüleceğini seçer.
- Sisteme "doğru kişiye adıyla dön" kuralı.
### Adım 6 · Uyku modu
- Bellek döngüsü geceleri 2 saatte bir gece gözlemi yapar (zihne işler).
- Uyanışta `uyanis.md` ajanı gece mesajlarından ilgili olanları seçer, en çok 2 kanala cevap.
- Uyurken gezgin/haberci çalışır ama stoklar; uyanınca "sabah haberi".
- Haber seçimine "Nişantaşı Üniversitesi ile ilgili konu öncelikli" kuralı.
### Adım 7 · Final
docs/ + AGENTS.md güncelle, tam doğrulama, push.

## Bekleyen / düşük öncelikli (5 ajan raporundan kalanlar)
- **Ajan 2 (HTTP):** global `.timeout(60sn)` stream'i kesebilir → `connect_timeout`+`read_timeout`+ilk-token sınırı (P0); hata sınıflandırma+retry; `reasoning_kapat` sağlayıcıya göre koşullu.
- **Ajan 1 (mekanik):** `mesgul` RAII guard (panik sızıntısı); typing'i edit döngüsünden çıkar; `soy` byte-dilimi char-güvenli.
- **Ajan 4 (hafıza):** `hafiza::yaz` atomik (geçici+rename); ajan yazımları tek sıra; günlükçü JSON hatasında ham döküm kurtarma; `arsivle` append.
- **Ajan 5 (döngüler):** döngü panik bekçisi (log+yeniden başlat); zarif kapanış (watch); uyanış kanal bazlı; süresi dolan haber sohbeti temizliği; tarama sırası (canlı mesajlar tarama boca'sıyla ezilmesin).

## Bilinen riskler
- İsteklilik/hedef mini çağrılarının token maliyeti → rate limitlerle sınırlı.
- Bellek kuyruğu bellek içinde; süreç çökerse işlenmemiş kuyruk kaybolur (kabul).
- Uyanış ajanı yanlış kişiyi seçebilir → fallback: son mesaj / etiketli.
- `.env`, `durum/`, `bot.log` git dışı (kişisel veri). `resimler/` yalnız `.gitkeep`.
