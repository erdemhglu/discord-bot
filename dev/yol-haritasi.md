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
### Adım 6 · Uyku modu — TAMAMLANDI
Gece gözlemi (2 saat), stok haber + sabah haberi, uyanış değerlendirmesi (`uyanis.md`),
etiket listesi hata kaybına karşı geri konur, üniversite haber önceliği.
### Adım 7 · Final — TAMAMLANDI
Tüm adımlar bitti; docs + doğrulama + push tamam. Açık kalanlar aşağıdaki "Bekleyen" listesinde.

## Adım 8: Modal'lar + /zihin — TAMAMLANDI
Yeni `src/modal.rs`: `zihin_bolumleri` 5 slot (özet / kişiler iki yarıda / konular /
olaylar+gündem), `sigdir` 4000 sınırında son satır/boşluk hizasında keser + not,
`modal_zihin/durum/yardim`, `komutlari_kayit` (guild komutları, ready'de idempotent).
`interaction_create` yeniden yazıldı: `Command` → modal, `Modal` → ephemeral onay,
`Component` → `dusunce_dugmesi` (ayrı impl). `!zihin` dizin dökümü + `/zihin` yönlendirmesi;
`!durum` artık `modal::durum_metni` ortak metni. 4 yeni test.

Doğrulanmış serenity 0.12.5 API notları:
- `CreateModal::new(custom_id, title)` — sıra: önce custom_id, sonra title.
- `CreateInputText::new(style, label, custom_id)` + `.value().required(false)`.
- `GuildId::set_commands(http, Vec<CreateCommand>)`; `CreateCommand::new(ad).description(...)`.
- Interaction varyantı `Interaction::Modal` (ModalSubmit değil).

Kalan risk: modal canlı davranışı Discord'ta görülecek (birim testleri boyut mantığını korur).

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
