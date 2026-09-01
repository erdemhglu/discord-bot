# discord-bot — ajanlar için giriş noktası

Bu dosya projeyi geliştirecek yapay zeka ajanlarının İLK okuyacağı yerdir. Kısa tutulur;
ayrıntı `docs/` altındadır. Kural: buraya ancak "her an geçerli" bilgi girer.

## Ne bu
Bir discord sunucusunda yıllardır takılan bir üye gibi davranan bot. Rust (serenity 0.12 +
tokio + reqwest), cevaplar OpenRouter (`openai/gpt-4o-mini`) ya da Mistral (`mistral-medium-latest`)
üzerinden; ikisi de OpenAI uyumlu chat/completions, seçim `.env`'den. Kişiliği kod değil,
arka planda çalışan ajanlar ve dosya tabanlı hafıza (`durum/`) belirler. Promptlar
`promptlar/*.md`, `include_str!` ile derlemeye gömülür.

## Hızlı komutlar
```
cargo build            # derle
cargo test             # 12 birim test (hafiza, gundem, seyahat)
cargo clippy           # 0 uyarı beklenir
cargo fmt              # commit'ten önce
cargo run --release    # .env: DISCORD_TOKEN + (OPENROUTER_KEY ya da MISTRAL_KEY); MODEL, SAGLAYICI, API_ADRES, FIRECRAWL_KEY, HABER_KANALI isteğe bağlı
```

## Yön levhası
| İhtiyaç | Nereye bak |
|---|---|
| Genel resim, katmanlar, veri akışı | docs/mimari.md |
| Bir fonksiyonun ne yaptığı, kim çağırıyor, kilit kuralı | docs/moduller.md |
| Bir olay olunca sırayla ne oluyor (mesaj, sohbet, uyku, seyahat, şaka, haber) | docs/akislar.md |
| `durum/` dosya biçimleri, sınırlar, özetleme | docs/durum-dosyalari.md |
| Hangi prompt nerede kullanılıyor, yer tutucular, max_tokens | docs/promptlar.md |
| Bütün sabitler ve anlamları | docs/sabitler.md |
| Neden böyle yapıldı (kararlar + gerekçe) | docs/kararlar.md |
| Yeni ajan/prompt/döngü/durum dosyası ekleme, tuzaklar, kontrol listesi | docs/gelistirme.md |
| Türkçe tanımlayıcıların İngilizce karşılığı | docs/sozluk.md |
| Gelişim evreleri ve isim seçme | docs/moduller.md (gelisim), docs/akislar.md |

## Değişmez kurallar (kodda da böyle)
1. **Kilit await üstünde tutulmaz.** `Bot::durum()` `std::sync::MutexGuard` döner; her zaman
   `{ let d = bot.durum(); ... }` bloğunda alınır, `.await` görmeden bırakılır.
2. **Model çıktısı sınırlanır, koda güvenilir.** Puanlar `clamp`, dosya boyları sabit, mesaj
   başına 1900 karakter (aşan cevap kırpılmaz, yeni mesaja bölünür). Sohbet cevabı bütçesi
   `cevap_butcesi!()` makrosunda: release'de max_tokens gitmez, debug'da kapak var; diğer
   çağrılarda max_tokens sabit. Model ne derse desin favori +10.
3. **Mention'lar kapalı gider** (`CreateAllowedMentions::new()`), yalnız hoş geldin pingler.
4. **Botlara, webhook'lara, DM'lere cevap yok.** Uyurken cevap yok (etiket bekletilir).
5. **Hiçbir hafıza silinmez**: sınırı aşan dosya özetlenir, ham parça `durum/arsiv/`'e gider.
6. **Kişilikle konuşan tek yol `Bot::uret` / `Bot::uret_akis`**, analiz yapan tek fonksiyon
   `Bot::analiz`. Ajanlar kişiliksizdir. Yeni bir "konuşma" mutlaka `uret`'ten (sohbet cevabı
   stream'de `uret_akis`'ten), yeni bir "değerlendirme" mutlaka `analiz`'den geçer.
7. **Prompt metni Rust'a yazılmaz**, `promptlar/*.md`'ye yazılır ve `src/promptlar.rs`'de
   `include_str!` ile bağlanır. Yer tutucular `{ad}` gibi süslü parantezli, `replace` ile dolar.
8. **Tanımlayıcılar Türkçe ve ASCII** (ü, ş yok). Yorumlar Türkçe. Kod "yapay zeka yazmış"
   gibi durmamalı: kısa, düz, açıklamalar sebep söyler.
9. `cargo fmt` satırları yeniden akıtır; metin eşleştirmeli yama yapacaksan önce dosyanın
   güncel halini oku (docs/gelistirme.md "tuzaklar").

## Durum klasörü (çalışma zamanı, git'e girmez)
`durum/INDEX.md` işaretçi · `kisiler/` `konular/` `olaylar/` içerik · `arsiv/` taşan ·
`huy.md profil.md duzeltmeler.md kendim.md gundem.md` ajan çıktıları. Bkz docs/durum-dosyalari.md.

## Bilinen açıklar / doğrulanmamış
- Canlı Discord akışı hiç test edilmedi (token yok). Serenity çağrıları derleyiciden geçti.
- Stream + thinking yalnızca birim testleriyle doğrulandı (sahte SSE sunucusu); canlı edit
  temposu (1,2 sn) Discord'ta ayrıca görülmedi.
- Thinking yalnız model üretirse görünür (`reasoning` / `reasoning_content`); gpt-4o-mini
  üretmez, o modelde bugünkü davranış aynen sürer.
- gpt-4o-mini görsel yorumu (resimci) canlıda görülmedi; başarısızsa metin yedeğine düşer.
- Kişi dosyaları görünen ada göre; aynı görünen adlı iki kişi çakışır.
- Anahtar kelime eşleme düz alt-dize; kök bulma yok.
- Bayram tarihleri 2026-2027 için elle yazılı (`src/seyahat.rs`), sonraki yıllar eklenmeli.
- Takma ad değiştirme (isim seçme) botun sunucuda CHANGE_NICKNAME iznine bağlı; yoksa log'a düşer, isim yine kullanılır.
- Mistral'de görsel yorumu modele bağlı (`mistral-medium-latest` görsel destekler); desteklemezse metin yedeği.
