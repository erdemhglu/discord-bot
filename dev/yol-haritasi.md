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

## Etkin plan — Adım 8: Modal'lar + /zihin (PLANLANDI, henüz kodlanmadı)

Kararlar (kullanıcı onaylı): modal'lar slash komutlarla açılır, `!` mesaj komutları paralel
düz metin olarak kalır (ikisi birden) · zihin modalı herkese açık · 5 slot önerilen dağılımda.

Discord kısıtları (tasarımı belirler):
- Modal yalnız bir interaction'a (slash/buton) yanıt olarak gönderilebilir; mesaj komutuyla açılamaz.
- Modal en fazla 5 bileşen (her biri TextInput, değer ≤4000 karakter), başlık ≤45, etiket ≤45.

### Yapılacaklar
1. Yeni modül `src/modal.rs`:
   - `zihin_bolumleri(&Durum) -> Vec<(başlık, içerik)>` — 5 slot, her değer ≤4000 (taşanı kes + not):
     1) Bot özeti: evre/gün, sohbet+mesaj sayacı, model, token metriği, uyku, seyahat, düşünme kipi, kendim.md özeti
     2-3) Kişiler (iki slotta): ad, puan, etiketler, not, bildikler (`kisiler/<id>.md`, mtime sırası)
     4) Konular: adlar + son notlar
     5) Olaylar (bu ay) + son gündem girişleri
   - `modal_durum`, `modal_yardim` (tek slotluk dar modal'lar)
   - `modal_olustur(baslik, custom_id, bolumler)`: `CreateModal` + ≤5 `CreateActionRow` →
     `InputText(paragraph, required=false)`; boş bölüm "(henüz boş)".
2. Slash kayıt (`ready`): her sunucuya guild komutu `/durum`, `/yardim`, `/zihin`
   (guild komutu anında görünür; her ready'de idempotent; adlar ASCII).
3. `interaction_create` genişletmesi (main.rs):
   - `Interaction::Command` → ada göre modal (`CreateInteractionResponse::Modal`)
   - `Interaction::ModalSubmit` → kısa ephemeral onay (modal gösterimlik, girdi toplamıyoruz)
   - Mevcut Düşünce butonu akışına dokunulmaz.
4. `komut.rs`: yeni `!zihin` → INDEX.md düz metin + "ayrıntı için /zihin" yönlendirmesi
   (5×4000 karakteri kanala dökmek spam); YARDIM metnine slash notu.
5. Test: `zihin_bolumleri` slot ≤5 ve içerik ≤4000, kesme davranışı.
6. Docs: moduller (modal.rs), akislar (interaction akışı), README (komutlar + slash), kararlar.
7. Doğrulama + commit + push.

### Doğrulanmış API notları (serenity 0.12.5 kaynağından)
- `CreateModal::new(custom_id, title)` — ARGÜMAN SIRASI: önce custom_id, sonra title!
  (`src/builder/create_interaction_response.rs:441`), `.components(Vec<CreateActionRow>)`.
- `CreateInputText` aynı dosyada (`create_components.rs:357`); API'si teyit edilecek
  (new/style/value/required).
- Slash kayıt yöntemi teyit edilecek: `GuildId::set_guild_commands` ya da eşdeğeri
  (kayıtlı komut adları: durum/yardim/zihin, description zorunlu).

### Notlar / riskler
- Zihin büyürse 2 kişi slotu yetmeyebilir → taşan kırpılır; ileride sayfalama düşünülebilir.
- Modal canlı davranışı Discord'ta doğrulanmalı (birim testleri boyut mantığını korur).

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
