# discord-bot — ajanlar için giriş noktası

Bu dosya projeyi geliştirecek yapay zeka ajanlarının İLK okuyacağı yerdir. Kısa tutulur;
ayrıntı `docs/` altındadır. Kural: buraya ancak "her an geçerli" bilgi girer.

## Ne bu
Bir discord sunucusunda yıllardır takılan bir üye gibi davranan bot. Rust (serenity 0.12 +
tokio + reqwest), cevaplar OpenRouter (varsayılan `openai/gpt-4o-mini`) ya da Mistral
(`mistral-medium-latest`) üzerinden; ikisi de OpenAI uyumlu chat/completions, seçim `.env`'den.
`MODEL` ile OpenRouter üzerinden herhangi bir model seçilebilir (GLM, Grok, Gemini, Claude, ...);
sağlayıcıya özel tek fark `cache_control` (prompt cache), hedef adrese göre koşullu eklenir
(`onbellek_destekler`, src/main.rs — openrouter.ai'ye giden her istekte eklenir, karar openrouter'a
bırakılır; mistral native api'de ve özel `API_ADRES` router'larında eklenmez). Kişiliği kod değil,
arka planda çalışan ajanlar ve dosya tabanlı hafıza (`durum/`) belirler. Promptlar `promptlar/*.md`,
`include_str!` ile derlemeye gömülür.

## Hızlı komutlar
```
cargo build            # derle
cargo test             # 79 birim test (hafiza, gundem, seyahat, stream, isteklilik, hedef, onbellek, çıktı protokolü, sohbet_cli, zihin görseli, yanıt çözümü)
cargo clippy           # 0 uyarı beklenir
cargo fmt              # commit'ten önce
cargo run -- zihin     # discord'suz zihin panelini durum/zihin.png'ye yazar (tasarimi gormek/test icin)
cargo run --release    # .env: DISCORD_TOKEN + (OPENROUTER_KEY ya da MISTRAL_KEY); MODEL, SAGLAYICI, API_ADRES, FIRECRAWL_KEY, HABER_KANALI, GUILD_ID, KANALLAR, DEBUG_KANALI isteğe bağlı
cargo run -- sohbet    # discord'suz terminal sohbet tezgâhı (token istemez, yalnız model anahtarı); çıktı protokolünü denemek için
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
| Türkçe tanımlayıcıların İngilizce karşılığı | docs/sozluk.md |
| Gelişim evreleri ve isim seçme | docs/moduller.md (gelisim), docs/akislar.md |

## Değişmez kurallar (kodda da böyle)
1. **Kilit await üstünde tutulmaz.** `Bot::durum()` `std::sync::MutexGuard` döner; her zaman
   `{ let d = bot.durum(); ... }` bloğunda alınır, `.await` görmeden bırakılır.
2. **Model çıktısı sınırlanır, koda güvenilir.** Puanlar `clamp`, dosya boyları sabit, mesaj
   başına 1900 karakter (aşan cevap kırpılmaz, yeni mesaja bölünür). Cevap satır bazlı bir
   protokoldür (`cevap_parcala`): her satır ayrı mesaj, **tur başına en çok 4 satır**
   (`PATLAMA_SINIRI`) — normalde 4 mesaj; 1900'ü aşan satır ayrıca bölünür, düşünme "göster"
   kipinde düşünce mesajları da eklenir. Tek başına `-` susma, `tepki: 💀` yazı yerine emoji
   tepkisi (yalnız bilinen emoji blokları kabul edilir).
   Sohbet cevabı bütçesi
   `cevap_butcesi!()` makrosunda: debug `Some(2000)`, release `Some(CEVAP_TAVANI=4096)` — ikisinde
   de üst sınır var, sıradan cevap altında kalır, yalnız tekrar/döngü gibi kaçak durumları keser;
   diğer çağrılarda max_tokens sabit. Model ne derse desin favori +10.
3. **Mention'lar kapalı gider** (`CreateAllowedMentions::new()`), yalnız hoş geldin pingler.
4. **Botlara, webhook'lara, DM'lere cevap yok.** Uyurken yazmaz ama dinler: mesajlar zihne
   işlenir, uyanınca gece yazılanlar değerlendirilir (etiket varsa kesin dönüş).
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
10. **Oturum hafızası `dev/` klasöründedir.** Context compact edilirse ya da yeni oturumda
    önce `dev/ilerleme.md` ve `dev/yol-haritasi.md` okunur; her anlamlı adımda (commit
    ölçeğinde) `dev/ilerleme.md`'ye kronolojik not düşülür, plan değişirse `yol-haritasi.md`
    güncellenir.

## Durum klasörü (çalışma zamanı, git'e girmez)
`durum/INDEX.md` işaretçi · `kisiler/` `konular/` `olaylar/` içerik · `arsiv/` taşan ·
`huy.md profil.md duzeltmeler.md kendim.md gundem.md` ajan çıktıları. Bkz docs/durum-dosyalari.md.

## Bilinen açıklar / doğrulanmamış
- Canlı Discord akışı hiç test edilmedi (token yok). Serenity çağrıları derleyiciden geçti.
- **`!zihin` görseli canlı Discord'da görülmedi.** PNG yerelde üretildi ve göze bakıldı
  (`cargo run -- zihin`), ama ek olarak gönderilmesi, Discord'un koyu temasındaki görünümü
  ve telefonda okunurluğu doğrulanmadı. Metin genişliği Inter'de harf/em oranıyla tahmin
  ediliyor (gerçek glif ölçümü değil); alışılmadık metinlerde sarma erken/geç olabilir.
- Stream + thinking yalnızca birim testleriyle doğrulandı (sahte SSE sunucusu); canlı edit
  temposu (1,2 sn) Discord'ta ayrıca görülmedi.
- Thinking yalnız model üretirse görünür (`reasoning` / `reasoning_content`); gpt-4o-mini
  üretmez, o modelde bugünkü davranış aynen sürer.
- gpt-4o-mini görsel yorumu (resimci) canlıda görülmedi; başarısızsa metin yedeğine düşer.
- Kişi dosyaları id bazlı (`kisiler/<id>.md`); isim→id çevrilemeyen kayıt o tur atlanır
  (`Durum.ad_id`). Eski slug dosyaları okunmaz.
- İsteklilik/hedef/uyanış mini çağrıları yalnız birim testleriyle doğrulandı; canlı davranış
  eşikleri (ISTEK_ESIGI=6, ilgi≥5) ayarlanmak isteyebilir.
- Anahtar kelime eşleme düz alt-dize; kök bulma yok.
- Bayram tarihleri 2026-2027 için elle yazılı (`src/seyahat.rs`), sonraki yıllar eklenmeli.
- Takma ad değiştirme (isim seçme) botun sunucuda CHANGE_NICKNAME iznine bağlı; yoksa log'a düşer, isim yine kullanılır.
- Mistral'de görsel yorumu modele bağlı (`mistral-medium-latest` görsel destekler); desteklemezse metin yedeği.
- `onbellek_destekler` hedef adrese bakar (yalnız `openrouter.ai`); openrouter'ın cache_control'ü
  desteklemeyen modelde gerçekten sessizce yok saydığı varsayımı canlıda doğrulanmadı.
- GUILD_ID/KANALLAR filtreleri ve reply-to'nun koşullu hale gelmesi (`son_etiketlendi`) canlıda
  hiç görülmedi, yalnız derleyici+testlerle doğrulandı.
- Emoji tepkisi (`create_reaction`), satır patlaması (satır = ayrı mesaj) ve susma (`-`) canlıda
  görülmedi; yalnız birim testleriyle doğrulandı. Tepki hız sınırı davranışı (Discord emoji
  route'ları ayrı kotaya tabi) canlıda ölçülmedi.
- `gonder_satirlar` satır arası gecikme sabitleri (300 ms + 15 ms/karakter, tavan 1500 ms)
  ölçülmedi, kabaca seçildi; canlıda ayarlanmak isteyebilir.
- CLI sohbet modu (`cargo run -- sohbet`) gerçek model anahtarıyla denenmedi (bu makinede
  anahtar yok): anahtarsız hata yolu ve birim testleri dışında **doğrulanmadı**.
- Reasoning zorunlu model (glm-5.3-flash) için ajan dayanıklılığı (bütçe ×2, effort=low, düşünceden JSON) canlıda
  doğrulanmadı; `!zihin test` ile denenir. Debug modu ve ayar paneli butonları canlı Discord'da görülmedi.
