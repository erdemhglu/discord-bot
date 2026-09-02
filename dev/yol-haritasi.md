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

## Adım 8: Modal'lar + /zihin — TAMAMLANDI (arayüz 2026-09-02 yeniden tasarlandı)
İlk sürüm 5 slotlu zihin modalıydı; canlı şikayet üzerine (içerik boş/kötü, tek kutuya boca)
**embed kart + detay modalı** düzenine geçildi: `/durum` `/yardim` `/zihin` ephemeral embed kart,
`/zihin`'de kişi select menüsü + bölüm butonları, her detay kendi etiketli modal alanlarında.
Ayrıntı `dev/ilerleme.md`'nin ilgili kaydında ve `docs/moduller.md` `src/modal.rs` bölümünde.
Eski 5 slot (`modal_zihin`/`bolumler`) kaldırıldı.

Doğrulanmış serenity 0.12.5 API notları:
- `CreateModal::new(custom_id, title)` — sıra: önce custom_id, sonra title.
- `CreateInputText::new(style, label, custom_id)` + `.value().required(false)`.
- `CreateSelectMenu::new(custom_id, CreateSelectMenuKind::String{options})` + `.placeholder()`;
  `CreateSelectMenuOption::new(label, value)` + `.description()`; `CreateActionRow::SelectMenu`.
- `CreateEmbed::new().title/color/description/field/ad/footer`; embed field value ≤1024.
- `CreateInteractionResponseMessage::new().ephemeral().embeds().components()`.
- `GuildId::set_commands(http, Vec<CreateCommand>)`; `CreateCommand::new(ad).description(...)`.
- Interaction varyantı `Interaction::Modal` (ModalSubmit değil); select menü seçimi
  `ComponentInteractionData.kind`'da `ComponentInteractionDataKind::StringSelect{values}`.

Kalan risk: modal canlı davranışı Discord'ta görülecek (birim testleri boyut mantığını korur).

## Token optimizasyonu + prod-hazırlık (2026-09-02) — TAMAMLANDI
İsteklilik/hedef_sec cache'li sabit bloğa taşındı · sohbet cevabına release'de de token tavanı
(CEVAP_TAVANI=3000) · çağrı-tipi bazlı token metriği + `!durum` kırılımı + önbellek isabet sayacı ·
`cache_control` model adına göre koşullu (GLM/GPT/Grok kırılmasın) · reply-to koşullu hale geldi
(`son_etiketlendi`) · `durum/taranan.md` kalıcı (her başlangıçta 14 günlük tarama tekrarlanmıyor) ·
GUILD_ID/KANALLAR ile kapsam daraltma · HTTP client timeout ayrıldı (P0 kapandı) · `mesgul` RAII
guard (`MesgulKilit`). Ayrıntı + gerekçe: docs/kararlar.md.

## Ruh hali + ikinci dayanıklılık turu (2026-09-02) — TAMAMLANDI
`ruh_hali_belirle` (RUH_HALI prompt, disküsyon sırasında insan ruh hali taklidi) · `soy` artık
bayt değil karakter say (Türkçe İ gibi harflerde panik riski kapandı) · `hafiza::yaz` atomik
(geçici dosya + rename, süreç kill olsa bile yarım dosya görünmez) · arka plan döngüleri
`dongu_bekci` ile sarmalandı (paniklerse loglayıp 5 sn sonra yeniden başlar, sessiz ölüm yok) ·
`durum/huy.md`'de "uykulu/uyudum amk/uyandırılmaktan bıktım" gibi gerçek uyku sistemiyle
karışan kalıntı satırlar temizlendi + `hoca.md`'ye bunu bir daha üretmeme kuralı eklendi
(kaynağı: hoca test sırasındaki sık `!uyan` muhabbetini kişilik sanmış).

## Zihin panel görseli (2026-09-02) — TAMAMLANDI
`!zihin` embed kart yerine PNG panel atıyor (`src/zihin_gorsel.rs`, SVG → resvg → PNG).
Gerekçe ve alternatifler (headless Chrome neden değil): docs/kararlar.md.
Bekleyen uçlar:
- **Kişi detay görseli.** `/zihin` menüsünden seçilen kişi için tek kişilik panel (bildikleri,
  son olaylar, puan geçmişi). Şu an o detay modalda metin olarak duruyor.
- **Açık tema.** Palet `zihin_gorsel.rs` başındaki `C_*` sabitlerinde tek yerde; açık tema
  ikinci bir palet + `!zihin acik` argümanıyla eklenebilir.
- **Gerçek glif ölçümü.** Şimdilik harf/em oranı tahmini. `fontdb`/`ttf-parser` ile gerçek
  advance okunabilir; tahmin yukarı yuvarladığı için taşma değil, boşluk israfı riski var.
- Görsel canlı Discord'da doğrulanmalı (ek gönderimi, koyu temada görünüm, telefonda okunurluk).

## Bekleyen / düşük öncelikli (5 ajan raporundan kalanlar)
- **Ajan 5 (döngüler):** uyanış kanal bazlı.
- Tamamlanıp düşenler: hata sınıflandırma+retry, typing edit dışı, ajan yazımları tek sıra,
  günlükçü JSON kurtarma, arsivle append, zarif kapanış (`KAPANIYOR`), süresi dolan haber
  sohbeti temizliği, tarama sırası (önüne ekleme) — yerel dalda yapıldı, PR merge'inde korundu.

## Bilinen riskler
- İsteklilik/hedef mini çağrılarının token maliyeti → rate limitlerle sınırlı.
- Bellek kuyruğu bellek içinde; süreç çökerse işlenmemiş kuyruk kaybolur (kabul).
- Uyanış ajanı yanlış kişiyi seçebilir → fallback: son mesaj / etiketli.
- `.env`, `durum/`, `bot.log` git dışı (kişisel veri). `resimler/` yalnız `.gitkeep`.
