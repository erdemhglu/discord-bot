# Sabitler

## src/bot/types/types_settings.rs (main.rs'in include! ettiği modülde)
| Sabit | Değer | Anlam |
|---|---|---|
| VERSION / VERSION_COMMIT / VERSION_DATE | Cargo.toml sürümü / build.rs'in git'ten aldığı commit (+ = derlemede commit'lenmemiş değişiklik vardı) / derleme tarihi | `version_text()`: /durum ve açılış duyurusu |
| OPENROUTER_URL / OPENROUTER_MODEL | …/api/v1/chat/completions / openai/gpt-4o-mini | varsayılan sağlayıcı |
| MISTRAL_URL / MISTRAL_MODEL | api.mistral.ai/v1/chat/completions / mistral-medium-latest | MISTRAL_KEY varsa ya da PROVIDER=mistral |
| CHANCE | 0.35 | yedek zar: isteklilik çağrısı başarısızsa araya girme olasılığı |
| WILLINGNESS_THRESHOLD / EVALUATION_INTERVAL | 6 / 2 dk | isteklilik puan eşiği / kanal başına en sık değerlendirme |
| CHAT_TIMEOUT | 30 dk | bu kadar sessiz kalan sohbet vedasız kapanır |
| COMMENT_WINDOW | 2 saat | haber attıktan sonra yorum bekleme |
| NEWS_INTERVAL | 6 saat | haber turu ve 6 saatlik ajanlar |
| POKE_INTERVAL / POKE_CHANCE | 1 saat / 0.3 | kendiliğinden laf atma |
| PRANK_INTERVAL / PRANK_CHANCE / HACK_SHARE / HACK_MESSAGES | 3 saat / 0.1 / 0.3 / 3 | görsel ve hack şakası |
| PROBLEM_SHARE | 0.25 | laf atma turlarının kod derdi olma payı |
| CHANNEL_HISTORY / CHAT_SEED | 60 / 10 | kanal başına saklanan satır / yeni sohbete tohum |
| HISTORY_DAYS | 14 | açılış taramasının derinliği |
| MEMORY_SIZE | 2000 | ham hafıza satırı |
| CHAT_SIZE | 20 | modele giden sohbet geçmişi |
| MESSAGE_LIMIT | 1900 | Discord 2000 sınırına pay |
| STREAM_EDIT_INTERVAL | 1200 ms | stream'de iki düzenleme arası asgari süre (Discord edit sınırı) |
| BURST_LIMIT | 4 | bir turda en çok kaç satır (= ayrı mesaj) gider; fazlası düşer. Ölçüye dayanıyor: gerçek IM'de bir kişinin peş peşe mesaj dizisi ortalama 1.7 mesaj, dizilerin %42'si çok-mesajlı (Baron 2010) — "her cevabı üçe böl" yanlış olur, 4 tavandır, hedef değil |
| HALF_LINE_THRESHOLD | 12 | akış sürerken son (henüz `\n` görmemiş) satır bu kadar karakteri geçmediyse gösterilmez; "tep" yarım hâlde mesaj olup bir sonraki edit'te silinmesin |
| LINE_DELAY_BASE / _PER_CHAR / _CAP | 300 ms / 15 ms per karakter / 1500 ms | `send_lines` (stream OLMAYAN yollar) satırlar arası bekleme + typing. Stream yolunda gecikme YOKTUR (akışın kendi temposu yeter). **Bu üç değer ölçülmedi**, insan yazma hızından kabaca seçildi; canlıda ayarlanmak isteyebilir |
| CONNECT_TIMEOUT / READ_TIMEOUT | 15 sn / 120 sn | http: el sıkışma / iki veri arası (ilk tokeni kapsar). Toplam süre sınırı yok, uzun düşünme akışı kesilmez |
| AI_RETRIES | 2 | ağ hatası / 429 / 5xx'te ek deneme sayısı (toplam bu + 1) |
| `reply_budget!()` (makro) / REPLY_CAP | debug `Some(2000)` / release `Some(4096)` | sohbet cevabı token bütçesi; ikisinde de üst sınır var, release'de yalnız tekrar/döngü gibi kaçak durumları keser |
| REASONING_BUDGET_BASE | 1500 | stream'siz ajan çağrısında reasoning kapatılamayınca yeniden deneme bütçesi: max(2×mevcut, bu) |
| FAVORITE | 259669117248864257 | her zaman sevilen kullanıcı id |
| WANDERER_INTERVAL | 4 saat | gündem gezintisi |
| IMAGE_DIR / STATE_DIR | resimler / durum | klasörler (çalışma dizinine göre; değerler bilerek Türkçe — gerçek disk yolları) |

## src/memory.rs
PERSON_LIMIT 1800 · PERSON_TARGET 1000 · TOPIC_LIMIT 1500 · TOPIC_TARGET 800 · EVENT_LIMIT 6000 ·
CONTEXT_BUDGET 6000 · INDEX_PEOPLE 40 · FAVORITE_NOTE · STOPWORDS (elenen kelimeler)

## src/agenda.rs
RSS_URL (Sözcü) · AGENDA_ENTRIES 12 · PAGE_LIMIT 3500

## src/sleep.rs
TIMEZONE_OFFSET +3 saat (TR, yaz saati yok) · INSOMNIA_CHANCE 0.07 · INSOMNIA_TENSE 0.20 ·
normal uyku 01:00→09:00 ±45 dk · uykusuz gece 01:00 ayakta, 06:00→13:00 ±45 dk

## src/travel.rs
EVENTS tablosu (yılbaşı 30 Ara 4g, sömestr 24 Oca 7g, ramazan
bayramı 2026: 19 Mar 4g / 2027: 8 Mar, 23 Nisan 3g, 19 Mayıs 3g, kurban 2026: 26 May 5g / 2027:
15 May, yaz 14 Tem 6g, zeytinli rock 21 Ağu 4g, 30 Ağustos 3g, 29 Ekim 3g)

## Kodda gömülü sayılar (sabit olmayan)
`send` kendi mesaj tamponu 50 · `pending_mentions` 20 · `retrieve` kişi ≤4/1200, konu ≤2/800,
olay 8, ham satır 12/200, anahtar ≤40, ≥2 eşleşme · `read_history` sayfa 100 · news_agent HN 12 +
RSS 12 · wander rss 20, sayfa ≤3 · yoldan mesaj günde 1, %25 · coach son 200 satır · profiler 600 ·
gözlem 300 · hack giriş max_tokens 150

## `durum/taranan.md`
`read_history` (14 günlük geçmiş taraması) daha önce taranmış sunucu id'lerini burada tutar;
her yeniden başlangıçta `State::load` okur, `guild_create`'te güncellenir. Yoksa her süreç
yeniden başlayışında her sunucunun tüm kanalları baştan taranırdı (API'ye ve zamana yazık).

## Ortam değişkenleri (.env)
DISCORD_TOKEN (discord'a bağlanmak için zorunlu; `cargo run -- chat` istemez) ·
OPENROUTER_KEY veya MISTRAL_KEY (biri zorunlu, CLI sohbet modunda da; ikisi de varsa openrouter) ·
PROVIDER=mistral (zorlama) · MODEL (model kimliği, sağlayıcının varsayılanını ezer) ·
API_URL (openai uyumlu chat/completions adresi; seçilen sağlayıcının adresini ezer) ·
FIRECRAWL_KEY (yoksa düz indirme) · NEWS_CHANNEL (kanal id; yoksa sistem kanalı / ilk metin kanalı) ·
GUILD_ID (tek sunucu id; ayarlıysa bot yalnız bu sunucuda çalışır) ·
CHANNELS (virgüllü kanal id listesi; ayarlıysa bot yalnız bu kanallarda çalışır) ·
DEBUG_CHANNEL (kanal id; `/debug` açıkken karar izleri buraya, yoksa mesajın kanalına) ·
IMAGE_ANALYSIS (varsayılan açık; `kapali/off/hayir/0` ekli fotoğrafları okumayı kapatır — yalnız
açılışta okunur, hiçbir slash komutla çalışırken değiştirilemez, `Bot.image_analysis`) ·
LOG_LEVEL (error/warn/info/debug/trace, varsayılan info) · LOG_COLOR (on/off; varsayılan: terminalde açık, dosyada kapalı)

**Not (2026-09-03):** bu değişken adları eskiden Türkçeydi (`SAGLAYICI`, `HABER_KANALI`, `KANALLAR`,
`DEBUG_KANALI`, `RESIM_ANALIZI`, `API_ADRES`, `LOG_SEVIYE`, `LOG_RENK`); kod İngilizceye
çevrilirken bunlar da çevrildi. Yerel `.env` dosyaları elle güncellenmeli, geriye dönük
uyumluluk shim'i yok.

## src/growth.rs
NAME_STAGE 2 (yerlesik) · STAGES: yeni (0 gün, 0 sohbet, confidence×0.7, poke×0.4) · isinma (3g, 8s, ×0.8, ×0.7) ·
yerlesik (10g, 25s, ×1, ×1) · eski-toprak (30g, 80s, ×1, ×1.2)
