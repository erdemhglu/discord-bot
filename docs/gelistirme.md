# Geliştirme rehberi

## Başlamadan
1. `AGENTS.md` oku. 2. İşin dokunduğu modül için `docs/moduller.md` bölümünü oku.
3. `cargo test && cargo clippy` yeşil mi bak. 4. Metin eşleştirmeli yama yapacaksan dosyanın
güncel halini önce oku (`cargo fmt` satırları akıtır, hizalı yorumları kaydırır).

## Bitti demeden
`cargo fmt` → `cargo clippy` (0 uyarı) → `cargo test` → `cargo build --release` → ilgili
`docs/` dosyasını güncelle (moduller/akislar/sabitler/promptlar) → `docs/kararlar.md`'ye
gerekçe → commit (Türkçe, ne+neden) → push.

## Tarifler

### Yeni prompt
1. `promptlar/<ad>.md` yaz; ilk satır `# Başlık`; yer tutucular `{x}`. (Bu dosya Türkçe kalır.)
2. `src/prompts.rs`'e `pub const AD: &str = include_str!("../promptlar/<ad>.md");` (sabit adı İngilizce).
3. Kullanan yerde `.replace("{x}", ..)`.
4. `docs/promptlar.md` tablosuna satır.

### Yeni ajan (arka plan değerlendirme)
1. `src/agents.rs` içinde `impl Bot { pub async fn <ad>(&self, ...) }`. Girdi `State`'den kilit
   altında klonlanır, kilit bırakılır, `self.analyze(metin, talimat, max_tokens).await`,
   sonuç kilit altında `State`'e ve `memory::write` ile dosyaya.
2. Sınır koy: `max_tokens`, sayıysa `clamp`, dosya ise `memory::over_limit` mantığı.
3. Takvime bağla: 6 saatlik tur `news_cycle` içinde ya da `reply` sonu.
4. `system_text`'e bölüm ekle (gerekiyorsa) ve `docs/mimari.md` sistem mesajı listesi.
5. `State::load` ile açılışta oku.

### Yeni döngü
`async fn <ad>_cycle(bot: Arc<Bot>, ctx: Context)`; `loop { sleep(..).await; if
!sleep::is_awake(&bot.state()) { continue; } ... }`; `ready` içinde `tokio::spawn` (yalnız
`started` ilk kez). Seyahat etkisini düşün (`travel::now()`).

### Yeni durum dosyası
`memory::read/write` kullan (yol `durum/` altına göre). Sınır ve arşiv kuralı belirle;
`docs/durum-dosyalari.md` tablosuna satır.

### Kişilik davranışını değiştirmek
Sabit kural → `promptlar/kisilik.md`. Zamanla değişmesi gereken şey → `hoca.md`'nin karar
alanına ekle (kod değişmez). Sohbet sonrası düzeltme → `elestirmen.md`.

### Sabit değiştirmek
`src/bot/types/types_settings.rs` başı; `docs/sabitler.md` güncelle.

## Tuzaklar
- **MutexGuard await üstünde:** derleyici `Send` hatası verir ya da kilitlenme olur. Kilit
  bloğunu `{ }` ile kapat, `.await` dışarıda.
- **`state.chats.get_mut` sonra `end_chat(&mut state)`:** ödünç çakışır. Önce bool hesapla,
  sonra çağır (`reply` örneği).
- **`content_safe`** mention'ları `@ad`'a çevirir; ham `msg.content` kullanma.
- **Serenity `Guild::member`** `&ctx` alır (CacheHttp). `ctx.cache.current_user()` bir guard
  döner; id'yi kopyala, guard'ı await'e taşıma.
- **Ready birden fazla kez gelir** (yeniden bağlanma). Döngüleri `started` korur.
- **`guild_create` her bağlanmada gelir.** `scanned` seti tekrar taramayı önler.
- **`include_str!` yolu `src/`'ye göredir:** `"../promptlar/x.md"`.
- **Tanımlayıcı İngilizce ve ASCII olmalı** (kod tarafı, bkz AGENTS.md madde 8); rustc
  uncommon_codepoints uyarır. `promptlar/*.md` ve `durum/` dosya alanları bu kuralın dışında,
  Türkçe kalır — model JSON çıktısıyla/mevcut disk verisiyle eşleşmeleri gerekiyor.
- **`cargo fmt` uzun `let … else { continue };`** satırlarını çok satıra böler; yama
  eşleştirmesi kırılır.
- **Firecrawl yanıtı** `data.markdown` yoksa hata; wander özete düşer.
- **RSS'te `<link>`** bazen `<atom:link/>` ile karışır; `tag_content` `<link>` ve `<link ` arar.
- **Seyahat tablosu**: bayramlar yıla özel; her yıl sonunda gelecek yılı ekle.

## Test etme
- Birim: `cargo test` (memory: tarih, slug, kişi biçimi, anahtar, ham çekme; agenda: rss,
  html, girişler; travel: gün no, yıl sarkması, bayram, yer sabitliği; çıktı protokolü:
  satır bölme, `tepki:`, sus işareti, slop önekleri, patlama sınırı, soru tavanı, `message_json`;
  chat_cli: satır çözme, bellek geçmişi sınırı).
- Protokol tezgâhı: `cargo run -- chat` (discord'suz, yalnız model anahtarı) — satır patlaması,
  `tepki:` ve `-` davranışını canlı modelde denemek için.
- Canlı: `.env` doldur, `cargo run --release`, log'da "logged in", "<sunucu>: N mesaj okundu"
  (`read history`), "profiler", "coach". `durum/INDEX.md` oluşmalı. Bir sohbet bitince
  `mind: diarist ... written` ve `kisiler/` altında dosya.
- Uyku testi: `TIMEZONE_OFFSET` veya `build_plan` saatlerini geçici değiştir; ya da `sleep::update`
  sonrası `plans` yazdır.
- Seyahat testi: `travel::on_day(day_number(y,m,d))` birim testleriyle.

## Yapılmamış / fikirler
- `reaction_add` olayı yok: bot tepkiyi yalnız verir, kendi mesajına gelen tepkiyi görmez
  (tepkiye tepki, "kim neye güldü" bilgisi kayıp).
- Özel (sunucuya yüklü) emoji tepkisi desteklenmiyor: `extract_emoji` `:kekw:` biçimini eler,
  yalnız Unicode emoji atılır. Gerekirse `ReactionType::Custom` + sunucu emoji listesinden
  doğrulama gerekir.
- ILGI/keyword kancası: belirli kelime geçince (kendi takıntı konuları) isteklilik çağrısını
  atlayıp doğrudan girme yolu yok; şimdilik her şey isteklilik puanından geçiyor.
- Kişi anahtarı için kullanıcı id (görünen ad çakışması).
- Anahtar kelime için basit kök kesme (Türkçe ekler).
- `plans` ve `posted_news` diske yazılmıyor; yeniden başlatınca sıfırlanır.
- Sesli kanal / tepki (reaction) olayları yok.
- Metrik: kaç çağrı, kaç token (OpenRouter yanıtındaki `usage` alanı okunmuyor).
