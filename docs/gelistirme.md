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
1. `promptlar/<ad>.md` yaz; ilk satır `# Başlık`; yer tutucular `{x}`.
2. `src/promptlar.rs`'e `pub const AD: &str = include_str!("../promptlar/<ad>.md");`.
3. Kullanan yerde `.replace("{x}", ..)`.
4. `docs/promptlar.md` tablosuna satır.

### Yeni ajan (arka plan değerlendirme)
1. `src/ajanlar.rs` içinde `impl Bot { pub async fn <ad>(&self, ...) }`. Girdi `Durum`'dan kilit
   altında klonlanır, kilit bırakılır, `self.analiz(metin, talimat, max_tokens).await`,
   sonuç kilit altında `Durum`'a ve `hafiza::yaz` ile dosyaya.
2. Sınır koy: `max_tokens`, sayıysa `clamp`, dosya ise `hafiza::sinir_asanlar` mantığı.
3. Takvime bağla: 6 saatlik tur `haber_dongusu` içinde ya da `cevapla` sonu.
4. `sistem_metni`'ne bölüm ekle (gerekiyorsa) ve `docs/mimari.md` sistem mesajı listesi.
5. `Durum::yukle` ile açılışta oku.

### Yeni döngü
`async fn <ad>_dongusu(bot: Arc<Bot>, ctx: Context)`; `loop { sleep(..).await; if
!uyku::uyanik_mi(&bot.durum()) { continue; } ... }`; `ready` içinde `tokio::spawn` (yalnız
`baslatildi` ilk kez). Seyahat etkisini düşün (`seyahat::simdi()`).

### Yeni durum dosyası
`hafiza::oku/yaz` kullan (yol `durum/` altına göre). Sınır ve arşiv kuralı belirle;
`docs/durum-dosyalari.md` tablosuna satır.

### Kişilik davranışını değiştirmek
Sabit kural → `promptlar/kisilik.md`. Zamanla değişmesi gereken şey → `hoca.md`'nin karar
alanına ekle (kod değişmez). Sohbet sonrası düzeltme → `elestirmen.md`.

### Sabit değiştirmek
`src/main.rs` başı; `docs/sabitler.md` güncelle.

## Tuzaklar
- **MutexGuard await üstünde:** derleyici `Send` hatası verir ya da kilitlenme olur. Kilit
  bloğunu `{ }` ile kapat, `.await` dışarıda.
- **`d.sohbetler.get_mut` sonra `sohbet_bitir(&mut d)`:** ödünç çakışır. Önce bool hesapla,
  sonra çağır (`cevapla` örneği).
- **`content_safe`** mention'ları `@ad`'a çevirir; ham `msg.content` kullanma.
- **Serenity `Guild::member`** `&ctx` alır (CacheHttp). `ctx.cache.current_user()` bir guard
  döner; id'yi kopyala, guard'ı await'e taşıma.
- **Ready birden fazla kez gelir** (yeniden bağlanma). Döngüleri `baslatildi` korur.
- **`guild_create` her bağlanmada gelir.** `taranan` seti tekrar taramayı önler.
- **`include_str!` yolu `src/`'ye göredir:** `"../promptlar/x.md"`.
- **Türkçe tanımlayıcı:** ASCII kullan (`durt`, `dürt` değil), rustc uncommon_codepoints uyarır.
- **`cargo fmt` uzun `let … else { continue };`** satırlarını çok satıra böler; yama
  eşleştirmesi kırılır.
- **Firecrawl yanıtı** `data.markdown` yoksa hata; gezgin özete düşer.
- **RSS'te `<link>`** bazen `<atom:link/>` ile karışır; `etiket_ici` `<link>` ve `<link ` arar.
- **Seyahat tablosu**: bayramlar yıla özel; her yıl sonunda gelecek yılı ekle.

## Test etme
- Birim: `cargo test` (hafiza: tarih, slug, kişi biçimi, anahtar, ham çekme; gundem: rss,
  html, girişler; seyahat: gün no, yıl sarkması, bayram, yer sabitliği).
- Canlı: `.env` doldur, `cargo run --release`, log'da "giriş yapıldı", "<sunucu>: N mesaj okundu",
  "profilci", "hoca". `durum/INDEX.md` oluşmalı. Bir sohbet bitince `gunlukcu: biten sohbet
  kaydedildi` ve `kisiler/` altında dosya.
- Uyku testi: `SAAT_FARKI` veya `plan_kur` saatlerini geçici değiştir; ya da `uyku::guncelle`
  sonrası `planlar` yazdır.
- Seyahat testi: `seyahat::gunde(gun_no(y,m,d))` birim testleriyle.

## Yapılmamış / fikirler
- Kişi anahtarı için kullanıcı id (görünen ad çakışması).
- Anahtar kelime için basit kök kesme (Türkçe ekler).
- `planlar` ve `atilan_haberler` diske yazılmıyor; yeniden başlatınca sıfırlanır.
- Sesli kanal / tepki (reaction) olayları yok.
- Metrik: kaç çağrı, kaç token (OpenRouter yanıtındaki `usage` alanı okunmuyor).
