# discord-bot — ajanlar için giriş noktası

Bu dosya projeyi geliştirecek yapay zeka ajanlarının İLK okuyacağı yerdir. Kısa tutulur;
ayrıntı `docs/` altındadır. Kural: buraya ancak "her an geçerli" bilgi girer.

## Ne bu
Bir discord sunucusunda yıllardır takılan bir üye gibi davranan bot. Rust (serenity 0.12 +
tokio + reqwest), cevaplar OpenRouter (varsayılan `openai/gpt-4o-mini`) ya da Mistral
(`mistral-medium-latest`) üzerinden; ikisi de OpenAI uyumlu chat/completions, seçim `.env`'den.
`MODEL` ile OpenRouter üzerinden herhangi bir model seçilebilir (GLM, Grok, Gemini, Claude, ...);
sağlayıcıya özel tek fark `cache_control` (prompt cache), hedef adrese göre koşullu eklenir
(`supports_cache`, src/main.rs — openrouter.ai'ye giden her istekte eklenir, karar openrouter'a
bırakılır; mistral native api'de ve özel `API_URL` router'larında eklenmez). Kişiliği kod değil,
arka planda çalışan ajanlar ve dosya tabanlı hafıza (`durum/`) belirler. Promptlar `promptlar/*.md`,
`include_str!` ile derlemeye gömülür (bu dizin ve dosya adları bilerek Türkçe bırakıldı — botun
Türkçe çalışma şeklinin bir parçası; kod tarafı İngilizce'dir, bkz. madde 8).

## Hızlı komutlar
```
cargo build            # derle
cargo test             # 76 birim test (memory, agenda, travel, stream, willingness, target, cache, çıktı protokolü, chat_cli, komut tablosu, yanıt çözümü)
cargo clippy           # 0 uyarı beklenir
cargo fmt              # commit'ten önce
cargo run --release    # .env: DISCORD_TOKEN + (OPENROUTER_KEY ya da MISTRAL_KEY); MODEL, PROVIDER, API_URL, FIRECRAWL_KEY, NEWS_CHANNEL, GUILD_ID, CHANNELS, DEBUG_CHANNEL, IMAGE_ANALYSIS isteğe bağlı
cargo run -- chat      # discord'suz terminal sohbet tezgâhı (token istemez, yalnız model anahtarı); çıktı protokolünü denemek için
```

## Yön levhası
| İhtiyaç | Nereye bak |
|---|---|
| Oturum durumu: yapılanlar, açık plan (compact sonrası İLK buraya bak) | dev/ilerleme.md, dev/yol-haritasi.md |
| Genel resim, katmanlar, veri akışı | docs/mimari.md |
| Bir fonksiyonun ne yaptığı, kim çağırıyor, kilit kuralı | docs/moduller.md |
| Bir olay olunca sırayla ne oluyor (mesaj, sohbet, uyku, seyahat, şaka, haber) | docs/akislar.md |
| Çıktı protokolü (satır = mesaj, `-` susma, `tepki:` emoji, resim, CLI tezgâh) | docs/akislar.md ("Çıktı protokolü", "CLI sohbet") |
| `durum/` dosya biçimleri, sınırlar, özetleme | docs/durum-dosyalari.md |
| Hangi prompt nerede kullanılıyor, yer tutucular, max_tokens | docs/promptlar.md |
| Bütün sabitler ve anlamları | docs/sabitler.md |
| Neden böyle yapıldı (kararlar + gerekçe) | docs/kararlar.md |
| Yeni ajan/prompt/döngü/durum dosyası ekleme, tuzaklar, kontrol listesi | docs/gelistirme.md |
| Türkçe kalan çalışma zamanı kelime dağarcığı (promptlar, durum/ alanları, ajan adları) | docs/sozluk.md |
| Gelişim evreleri ve isim seçme | docs/moduller.md (growth), docs/akislar.md |

## Değişmez kurallar (kodda da böyle)
1. **Kilit await üstünde tutulmaz.** `Bot::state()` `std::sync::MutexGuard` döner; her zaman
   `{ let state = bot.state(); ... }` bloğunda alınır, `.await` görmeden bırakılır.
2. **Model çıktısı sınırlanır, koda güvenilir.** Puanlar `clamp`, dosya boyları sabit, mesaj
   başına 1900 karakter (aşan cevap kırpılmaz, yeni mesaja bölünür). Cevap satır bazlı bir
   protokoldür (`parse_reply`): her satır ayrı mesaj, **tur başına en çok 4 satır**
   (`BURST_LIMIT`) — normalde 4 mesaj; 1900'ü aşan satır ayrıca bölünür, düşünme "göster"
   kipinde düşünce mesajları da eklenir. Tek başına `-` susma, `tepki: 💀` yazı yerine emoji
   tepkisi (yalnız bilinen emoji blokları kabul edilir).
   Sohbet cevabı bütçesi
   `reply_budget!()` makrosunda: debug `Some(2000)`, release `Some(REPLY_CAP=4096)` — ikisinde
   de üst sınır var, sıradan cevap altında kalır, yalnız tekrar/döngü gibi kaçak durumları keser;
   diğer çağrılarda max_tokens sabit. Model ne derse desin favori +10.
3. **Mention'lar kapalı gider** (`CreateAllowedMentions::new()`), yalnız hoş geldin pingler.
4. **Botlara, webhook'lara, DM'lere cevap yok.** Uyurken yazmaz ama dinler: mesajlar zihne
   işlenir, uyanınca gece yazılanlar değerlendirilir (etiket varsa kesin dönüş).
5. **Hiçbir hafıza silinmez**: sınırı aşan dosya özetlenir, ham parça `durum/arsiv/`'e gider.
6. **Kişilikle konuşan tek yol `Bot::generate` / `Bot::generate_stream`**, analiz yapan tek
   fonksiyon `Bot::analyze`. Ajanlar kişiliksizdir. Yeni bir "konuşma" mutlaka `generate`'ten
   (sohbet cevabı stream'de `generate_stream`'ten), yeni bir "değerlendirme" mutlaka
   `analyze`'den geçer.
7. **Prompt metni Rust'a yazılmaz**, `promptlar/*.md`'ye yazılır ve `src/prompts.rs`'de
   `include_str!` ile bağlanır. Yer tutucular `{ad}` gibi süslü parantezli, `replace` ile dolar.
8. **Tanımlayıcılar İngilizce ve ASCII, yorumlar İngilizce** — ama botun Türkçe çalışma şekli
   koda dokunmaz: `promptlar/*.md` (dizin+dosya adı+içerik), `durum/` dosya biçimleri (alan
   adları, dosya adları), ve Discord'a çıkan her şey (slash komut adları/açıklamaları, embed
   metni, buton/menü etiketleri, model çıktısı) Türkçe kalır. Kod "yapay zeka yazmış" gibi
   durmamalı: kısa, düz, açıklamalar sebep söyler. Türkçe kalan çalışma zamanı terimleri için
   bkz. docs/sozluk.md.
9. `cargo fmt` satırları yeniden akıtır; metin eşleştirmeli yama yapacaksan önce dosyanın
   güncel halini oku (docs/gelistirme.md "tuzaklar").
10. **Oturum hafızası `dev/` klasöründedir.** Context compact edilirse ya da yeni oturumda
    önce `dev/ilerleme.md` ve `dev/yol-haritasi.md` okunur; her anlamlı adımda (commit
    ölçeğinde) `dev/ilerleme.md`'ye kronolojik not düşülür, plan değişirse `yol-haritasi.md`
    güncellenir.
11. **Bot yalnız slash (`/`) komutlarla yönetilir**, `!`/metin komut yok. Komutlar tek kayıt
    tablosunda (`command::definitions()`, src/command.rs): ad, açıklama, Discord seçenekleri ve
    çalıştırıcı bir arada; `modal::register_commands` bu tablodan Discord'a kayıt çıkarır,
    `interaction_create` (main.rs) `Interaction::Command`'ı isme göre tabloda bulup çalıştırır.
    Her komut embed döner (düz metin yok); 3 sn'yi aşabilecek komutlar (ağ/model çağrısı yapanlar)
    önce `defer` ile erteleyip `report_result` ile sonucu düzenler.

## Durum klasörü (çalışma zamanı, git'e girmez)
`durum/INDEX.md` işaretçi · `kisiler/` `konular/` `olaylar/` içerik · `arsiv/` taşan ·
`huy.md profil.md duzeltmeler.md kendim.md gundem.md` ajan çıktıları. Bkz docs/durum-dosyalari.md.

## Bilinen açıklar / doğrulanmamış
- Canlı Discord akışı hiç test edilmedi (token yok). Serenity çağrıları derleyiciden geçti.
- **Slash komut tablosu (`command::definitions()`) canlı Discord'da hiç görülmedi.** Kayıt
  (`register_commands`), seçenekler (choice/min/max), erteleme+düzenleme akışı (`defer` /
  `report_result`, 3 sn sınırı) ve embed çıktıları yalnız derleyici+birim testleriyle
  doğrulandı; gerçek Discord istemcisinde seçenek adları/görünümü kontrol edilmedi.
- Stream + thinking yalnızca birim testleriyle doğrulandı (sahte SSE sunucusu); canlı edit
  temposu (1,2 sn) Discord'ta ayrıca görülmedi.
- Thinking yalnız model üretirse görünür (`reasoning` / `reasoning_content`); gpt-4o-mini
  üretmez, o modelde bugünkü davranış aynen sürer.
- gpt-4o-mini görsel yorumu (image_commenter) canlıda görülmedi; başarısızsa metin yedeğine düşer.
- Kişi dosyaları id bazlı (`kisiler/<id>.md`); isim→id çevrilemeyen kayıt o tur atlanır
  (`State.name_to_id`). Eski slug dosyaları okunmaz.
- İsteklilik/hedef/uyanış mini çağrıları yalnız birim testleriyle doğrulandı; canlı davranış
  eşikleri (WILLINGNESS_THRESHOLD=6, ilgi≥5) ayarlanmak isteyebilir.
- Anahtar kelime eşleme düz alt-dize; kök bulma yok.
- Bayram tarihleri 2026-2027 için elle yazılı (`src/travel.rs`), sonraki yıllar eklenmeli.
- Takma ad değiştirme (isim seçme) botun sunucuda CHANGE_NICKNAME iznine bağlı; yoksa log'a düşer, isim yine kullanılır.
- Mistral'de görsel yorumu modele bağlı (`mistral-medium-latest` görsel destekler); desteklemezse metin yedeği.
- `supports_cache` hedef adrese bakar (yalnız `openrouter.ai`); openrouter'ın cache_control'ü
  desteklemeyen modelde gerçekten sessizce yok saydığı varsayımı canlıda doğrulanmadı.
- GUILD_ID/CHANNELS filtreleri ve reply-to'nun koşullu hale gelmesi (`last_was_tagged`) canlıda
  hiç görülmedi, yalnız derleyici+testlerle doğrulandı.
- Emoji tepkisi (`create_reaction`), satır patlaması (satır = ayrı mesaj) ve susma (`-`) canlıda
  görülmedi; yalnız birim testleriyle doğrulandı. Tepki hız sınırı davranışı (Discord emoji
  route'ları ayrı kotaya tabi) canlıda ölçülmedi.
- `send_lines` satır arası gecikme sabitleri (300 ms + 15 ms/karakter, tavan 1500 ms)
  ölçülmedi, kabaca seçildi; canlıda ayarlanmak isteyebilir.
- CLI sohbet modu (`cargo run -- chat`) gerçek model anahtarıyla denenmedi (bu makinede
  anahtar yok): anahtarsız hata yolu ve birim testleri dışında **doğrulanmadı**.
- Reasoning zorunlu model (glm-5.3-flash) için ajan dayanıklılığı (bütçe ×2, effort=low, düşünceden JSON) canlıda
  doğrulanmadı; `/zihin test:true` ile denenir. Debug modu ve ayar paneli butonları canlı Discord'da görülmedi.
- **2026-09-03: kod tabanı (src/**/*.rs, README.md) Türkçe'den İngilizceye çevrildi**
  (tanımlayıcılar, yorumlar, dosya/dizin adları, .env değişken adları). Botun çalışma şekli
  (promptlar/, durum/ dosya biçimleri, Discord'a çıkan her şey) bilerek Türkçe bırakıldı — bkz.
  madde 8. Bu çeviri canlı Discord'da doğrulanmadı; yalnız derleyici + 76 birim test + clippy
  ile kontrol edildi. AGENTS.md/docs/dev/ içindeki düzyazı Türkçe kaldı, yalnız kod
  referansları (fonksiyon/dosya/env değişken adları) güncellendi.
