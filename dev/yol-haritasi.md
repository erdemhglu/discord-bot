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

## "Normal insan gibi tepki" turu (2026-09-02) — TAMAMLANDI
Emin'in isteği üzerine kişilikteki mekanik yazma sınırları kaldırıldı, yerine satır bazlı bir
çıktı protokolü geldi. Dört lane paralel koştu: L1 protokol+stream (`Cevap`, `cevap_parcala`,
`slop_temizle`, `AkisSonuc::Sus`, `AkisBaglam.tepki_hedefi`, `soru_fazla_mi`, `gonder_satirlar`),
L2 promptlar (`kisilik.md` NASIL YAZARSIN yeniden, `elestirmen.md` denetim maddeleri),
L3 resim + CLI (`Mesaj.resim`, `mesaj_json`, `kullanici_resimli`, `Bot::kur`, `src/sohbet_cli.rs`),
L4 dokümantasyon. Dokunulmayan bölümler (sunucu kuralları, kandırılmazsın vb.) byte byte aynı.
Doğrulama: fmt temiz · clippy 0 uyarı · 65 test yeşil (önceki 51) · release build.
Ayrıntı + gerekçe (araştırma URL'leriyle): docs/kararlar.md 2026-09-02 girdileri.

**İnceleme turu (aynı gün) — TAMAMLANDI.** Kapı + üç incelemecinin yüksek/orta bulgularının
hepsi uygulandı: `gonder_cevap` ayrıldı (tepki hedefi yoksa tepki düşer, gidecek bir şey yoksa
`None`), hoş geldin ping'i protokol çözümünden sonra takılıyor, `-`+`tepki:` birleşimi susma
sayılmıyor, `sohbet_baslat` dedup'ı ve yedek `uret` yolundaki tekrar elemesi satır bazlı,
komut algılaması ham metinde, `dokum` her bot satırına önek koyuyor, `emoji_ayikla` gerçek
emoji bloklarıyla sınırlı, numara öneki yalnız gerçek listede siliniyor, `kanal_not_coklu` ile
tur başına tek dosya yazımı. 70 test yeşil. Uygulanmayanlar (gerekçeli): YASAK KALIPLAR'daki
"Aynı mesajda" ifadesi (kabul çıtası o bölümü dondurmuş) ve `SOHBET_TOHUM`/`KANAL_GECMIS`'in
satır enflasyonuna göre büyütülmesi (sabit ayarı, canlı ölçüm ister).

Kalan risk (canlıda görülecek): emoji tepkisi rate limit davranışı, satır patlamasının gerçek
kanaldaki temposu, `-` susmasının sıklığı (model fazla susarsa prompt ayarlanır),
`gonder_satirlar` gecikme sabitleri (ölçülmedi).

## Zihin panel görseli (2026-09-02) — TERK EDİLDİ, `zihin_gorsel.rs` silindi
`!zihin` bir süre embed yerine PNG panel atıyordu (`src/zihin_gorsel.rs`, SVG → resvg → PNG).
Aynı gün kullanıcı geri döndü ("kötü duruyor, embed düzgün olsun") — panel tamamen kaldırıldı,
`/zihin`'in embed+buton+select+modal yapısı tek yol oldu. Aşağıdaki bekleyen uçlar artık
**geçersiz** (kod yok): kişi detay görseli, açık tema, gerçek glif ölçümü. Gerekçe: docs/kararlar.md
("Panel görseli terk edildi" kaydı).

## Komutlar slash'a taşındı (2026-09-02) — TAMAMLANDI (Faz 1+2), Faz 3 açık
Kullanıcı: "bir komut yöneticisi hazırlayıp tüm komutları onun altına taşı ve tüm komutlar düz
text yerine embed çıktısı versin", sonra "ünlem komutlarını tamamen devre dışı bırak sadece slash
commands ile çalışsın bot". Plan: `dev/ilerleme.md`'nin "Panel görseli terk edildi, bot tamamen
slash komutlara geçti" kaydında ayrıntılı.
- **Faz 1 — TAMAMLANDI**: panel görseli kaldırıldı (yukarı bak).
- **Faz 2 — TAMAMLANDI**: `Bot::komut` + `!`/metin yakalama bloğu kalktı; `komut::KomutTanimi`
  kayıt tablosu (`src/komut.rs`) kondu, 12 eski `!` komutu slash'a taşındı, hepsi embed döner.
- **Faz 3 — TAMAMLANDI** (gerçek `mod` değil `include!` ile — bkz docs/kararlar.md): `main.rs`
  (4695 satır) ve sonra `komut.rs` (578 satır) `src/bot/` ve `src/komut/` altında ~50 küçük
  dosyaya (çoğu <200 satır) bölündü. `pub(crate)` gerekmedi, diğer 6 kardeş dosyaya hiç
  dokunulmadı — `include!` görünürlüğü/`use super::*`'ı hiç değiştirmiyor.

## Bekleyen / düşük öncelikli (5 ajan raporundan kalanlar)
- **`reaction_add` olayı yok:** bot tepki verir ama kendi mesajına gelen tepkiyi görmez
  (tepkiye tepki, "kim neye güldü" bilgisi kayıp).
- **Özel emoji tepkisi:** `extract_emoji` `:kekw:` biçimini eler, yalnız Unicode emoji atılıyor.
  `ReactionType::Custom` + sunucu emoji listesinden doğrulama gerekir. Aynı iş emoji whitelist'i
  de getirir (ra-muhendislik §10 öneriyordu, bilerek ertelendi — bkz kararlar.md).
- **Tohum/geçmiş pencereleri satır cinsinden:** `CHAT_SEED=10` ve `CHANNEL_HISTORY=60` artık
  "tur" değil "satır" sayıyor; çok satırlı turlarda modelin gördüğü geçmiş kısalıyor. Canlıda
  ölçülüp büyütülmesi gerekebilir (şimdilik dokunulmadı).
- **ILGI/keyword kancası:** takıntı konusu geçtiğinde isteklilik çağrısını atlayıp doğrudan
  girme yolu yok; her şey isteklilik puanından geçiyor.
- **Ajan 5 (döngüler):** uyanış kanal bazlı.
- Tamamlanıp düşenler: hata sınıflandırma+retry, typing edit dışı, ajan yazımları tek sıra,
  günlükçü JSON kurtarma, arsivle append, zarif kapanış (`SHUTTING_DOWN`), süresi dolan haber
  sohbeti temizliği, tarama sırası (önüne ekleme) — yerel dalda yapıldı, PR merge'inde korundu.

## Bilinen riskler
- İsteklilik/hedef mini çağrılarının token maliyeti → rate limitlerle sınırlı.
- Bellek kuyruğu bellek içinde; süreç çökerse işlenmemiş kuyruk kaybolur (kabul).
- Uyanış ajanı yanlış kişiyi seçebilir → fallback: son mesaj / etiketli.
- `.env`, `durum/`, `bot.log` git dışı (kişisel veri). `resimler/` yalnız `.gitkeep`.
- **Reasoning:** glm-5.3-flash ile canlı doğrulama (`/zihin test:true`), effort=low'un gerçekten düşünceyi kısıp kısmadığı.
- **Slash komutlar:** hiçbiri (yeni 12 dahil) canlı Discord'da hiç görülmedi; seçenek/choice görünümü, `defer`+`report_result` akışı doğrulanmalı.

## 2026-09-03 · Kod İngilizceye çevrildi — TAMAMLANDI
`src/**/*.rs` + README.md İngilizceye çevrildi (tanımlayıcılar, yorumlar, dosya/dizin adları,
`.env` değişken adları); botun Türkçe çalışma şekli (promptlar/, durum/, Discord çıktısı)
değişmedi. Yukarıdaki eski kayıtlardaki kod referansları (tarih öncesi) o zamanki Türkçe
adlarla bırakıldı — geçmiş doğru olsun diye elle değiştirilmedi; yalnız hâlâ açık olan
"Bekleyen"/"Bilinen riskler" maddelerindeki isimler güncel koda uysun diye yukarıda
güncellendi. Ayrıntı: dev/ilerleme.md, docs/kararlar.md, AGENTS.md madde 8.
