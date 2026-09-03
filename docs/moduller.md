# Modüller ve fonksiyonlar

Her satır: imza · ne yapar · kim çağırır · kilit/await notu. Satır numaraları yaklaşıktır,
`grep -n "fn ad"` ile bul.

## src/main.rs (+ src/bot/<grup>/*.rs, `include!` ile aynı modülde)

### Tipler
- `ChatMessage { role: &'static str, content: String, image: Option<String> }` — OpenRouter'a giden mesaj (serenity'nin kendi `Message` tipiyle karışmasın diye `Mesaj` yerine bilerek `ChatMessage`). `user(..)`, `user_with_image(metin, url)`, `assistant(..)` kurucular. `image` `#[serde(skip)]`: istek gövdesi elle kurulduğu için serileştirmeye girmez, `message_json` okur. Yalnız en son kullanıcı mesajında dolu kalır (`Handler::message` yeni satır eklerken eskilerinkini `None` yapar) — discord cdn linki ömürlü, eski görseli her turda yollamak token yakar.
- `message_json(&ChatMessage) -> Value` — mesajı openai uyumlu bloğa çevirir: resim yoksa `{role, content: "…"}`, varsa `content` = `[{type:text,text},{type:image_url,image_url:{url}}]` (agents.rs `image_commenter` gövdesiyle aynı biçim). `ask_split` ve `ask_raw_stream` ikisi de bunu kullanır, yani görsel hem stream'li hem stream'siz yolda gider.
- `Reply { lines: Vec<String>, reaction: Option<String>, silent: bool }` — model cevabının çözülmüş hâli (çıktı protokolü, bkz akislar.md). `Reply::is_empty()` ne söz ne tepki ne susma kararı var mı; `Reply::protocol_text()` geçmişe/kanal notuna giren biçim (satırlar `\n` ile + varsa `tepki: 💀`), model bir sonraki turda kendi biçimini görsün diye.
- `Chat { history: Vec<ChatMessage>, counter: u32, hacked: u32, last_message, last_was_tagged: bool, incoming: u32, recent_arrivals, mood: String }` —
  bir kanaldaki açık sohbet. `counter` botun yazdığı mesaj sayısı; `hacked` hack şakasında kalan cevap
  sayısı; `last_was_tagged` reply-to kararı için (bkz `reply`); `mood` `determine_mood`'un
  son sonucu, "durum (yoğunluk)" biçiminde, boşsa nötr.
- `State` — tek paylaşılan durum (bkz mimari.md). `State::load()` diskten profil/huy/duzeltmeler/kendim/gundem/taranan okur, dizini yeniler.
- `Bot { state: Mutex<State>, http: reqwest::Client, key, news_channel, firecrawl, guild_id: Option<GuildId>, allowed_channels: Option<HashSet<ChannelId>> }`.
- `Bot::state() -> MutexGuard<State>` — zehirli kilidi de açar. **Await üstünde tutma.**
- `Handler { bot: Arc<Bot>, started: AtomicBool }` — serenity `EventHandler`.
- `BotError = Box<dyn Error + Send + Sync>`.

### Yardımcılar
- `now_unix() -> i64` — şu an, unix saniye.
- `display_name(&User) -> String` — görünen ad (`global_name`), yoksa kullanıcı adı. Hafıza ve kişi dosyaları bu adla.
- `channel_note(&mut State, kanal, satir)` — kanal geçmişine (bellek 60 + `durum/kanallar/<id>.md`) ekler; kullanıcı satırları `message`'dan, bot satırları `send`'den. `channel_notes(&mut State, kanal, satirlar)` aynı işi birden çok satır için TEK dosya yazımıyla yapar (`channel_note` onun tek elemanlı hâli); `send_stream` çok satırlı cevabı bununla yazar, yoksa satır başına bütün geçmiş baştan yazılıyordu.
- `remember(&mut State, isim, metin)` — ham hafızaya "isim: metin" ekler, 2000'i aşarsa baştan atar.
- `recent_messages(&State, n) -> String` — ham hafızanın son n satırı, `\n` ile.
- `transcript(&[ChatMessage], bot_adi) -> String` — sohbeti "isim: metin" satırlarına çevirir. Bot cevabı çok satırlı olabildiği için (protokol metni) **her** satırına `bot_adi:` öneki konur — tepki satırı dahil; yoksa eleştirmen/günlükçü/hoca alt satırları gruptaki insanlara sayar.
- `strip_name(&str, bot_adi) -> &str` — model çıktısı: baştaki `bot_adi:` kalıbı ve dış tırnak atılır. Dilim döner, klonlamaz.
- `clean(String, bot_adi) -> String` — `strip_name` + 1900 karakterde keser. `generate`'in çıkışında, yani stream'siz yolda cevabın TAMAMINA uygulanır (protokol satırlara ayrılmadan önce): 4 satırlık bir cevabın toplamı 1900'ü aşarsa son satır(lar) sessizce kırpılır. Stream yolunda kırpma yok, her satır ayrı ayrı `split` ile bölünür.
- `parse_reply(metin) -> Reply` — **çıktı protokolünü çözen tek yer.** `strip_name` uygulanmış metinde çalışır (yeniden soymaz): `\n` ile böler, trim, boşları atar; `silence_marker` satırı → `silent`; `reaction_body` + `extract_emoji` → `reaction` (ilk kazanır, satır mesaj olarak gitmez); `'` ile başlayan kırıntı satır atılır; `clean_slop` uygulanır; **gerçek liste** ise (≥2 satırda numara öneki) `number_prefix` ile `1. `/`2) ` önekleri de silinir — tek satırdaki "3. sınıftayım" sıra sayısıdır, dokunulmaz; aynı turda birebir tekrar eden satır ikinci kez alınmaz; en çok `BURST_LIMIT` (4) satır kalır (fazlası debug log ile düşer); her satır `split(satir, MESSAGE_LIMIT)` ile düzleştirilir. **Kısa satır elenmez** ("he", "yok", "la" doğal tepkidir). Çağıranlar: `send_stream`, `send_lines`, `stream_view`, `run_prank`, `chat_cli`.
- `reaction_body(satir) -> Option<&str>` — satır `tepki:` ile mi başlıyor (büyük/küçük harf ve "tepki :" boşluğu tolere edilir); iki noktadan sonrası döner. `too_many_questions` da bunu tepki satırlarını saymamak için kullanır.
- `extract_emoji(metin) -> Option<String>` — ilk emoji dizisi: `emoji_start` (bilinen emoji blokları: U+2600–27BF, U+2B00–2BFF, U+1F000–1FAFF ve ©/®/™ gibi tekiller) ile başlar, `emoji_continues` (aynılar + VS15/VS16, ZWJ, keycap) ile en çok 8 char sürer. Tanım bilerek dar: "harf değilse emojidir" demek `—`, `…`, `→`, tipografik tırnak gibi işaretleri de emoji sayıyordu ve Discord isteği 400 ile dönüyordu. `:kekw:` gibi özel emoji biçiminde ve emoji hiç yoksa `None`.
- `silence_marker(satir) -> bool` — satır tek başına `-`, `"-"`, `'-'`, `[sus]` ya da `(sus)` mu.
- `clean_slop(satir) -> String` — "yapay zeka yazmış" izlerini siler: baştaki `- `/`* `/`• ` madde öneki, `**` ve `__` markdown işaretleri. Backtick hem kendisi hem İÇİ korunur (satır `` ` `` ile bölünür, tek indeksli parçalara dokunulmaz) — `` `__init__` `` bozulmasın. Numara öneki burada değil `parse_reply`'de (`number_prefix`) elenir, çünkü "gerçek liste mi Türkçe sıra sayısı mı" ancak cevabın tamamına bakınca ayırt edilir.
- `number_prefix(satir) -> Option<&str>` — `1. ` / `2) ` önekinden sonrası. Tek başına uygulanmaz; `parse_reply` yalnız cevapta ≥2 numaralı satır varsa çağırır.
- `too_many_questions(&State, kanal) -> bool` — kanal geçmişindeki son 4 bot satırından (`tepki:` satırları sayılmaz) ≥2'si `?` ile bitiyor mu. `reply` ve `chat_cli` talimata "Bu sefer soru sorma; düz laf et ya da sus." ekler; kesme yok. Kod ölçer, uygulamayı model yapar.
- `split(metin, sinir) -> Vec<String>` — metni en çok `sinir` karakterlik parçalara böler: önce cümle sınırı, sonra boşluk, o da yoksa sert keser; hiçbir şey atılmaz. Cevap 1900'ü aşınca ve uzun thinking'te kullanılır.
- `cut_point(metin, sinir) -> usize` — `split`'in kesim yeri; sınırın dörtte birinden önceki cümle/boşluk sayılmaz.
- `spoiler(metin) -> String` — `||...||`; içindeki `|` kaçırılır.
- `stream_view(kip, dusunce, cevap, bitti) -> Vec<String>` — kipe göre ekran: düşünme sürerken (cevap boş, düşünce var) göster kipinde "Düşünüyorum...", gizle kipinde `thought_counter` (canlı kelime sayısı), sessiz/kapalı kipinde hiçbir şey; cevap başlayınca göster kipinde `single_line(dusunce)` hem spoiler hem `code_blocks` + cevap satırları, gizle/sessiz/kapalı kiplerde yalnız satırlar. Cevap artık `split` ile değil `parse_reply(...).lines` ile mesajlara ayrılır: model yeni satıra geçince yeni mesaj açılır, önceki mesaj değişmez. `single_line(metin)` thinking'i tek akıcı satıra indirger (her düşüncede newline atılmaz).
- `stream_slice(cevap, bitti) -> &str` — akış sürerken cevabın gösterilebilir kısmı: tamamlanmış satırlar (ardında `\n` olan) + son yarım satır ancak `HALF_LINE_THRESHOLD` (12) karakteri geçtiyse. Gerekçe: "tep" yarım hâlde `tepki: 💀` ya da `-` olabilir, mesaj olarak açılıp bir sonraki edit'te silinmesin; kısa satır için boşuna edit atılmasın. `bitti=true` ise metnin tamamı.
- `stream_layout(kip, dusunce, satirlar) -> Vec<String>` — düşünce blokları (yalnız göster kipinde) + satır mesajları. Satırlar dışarıdan gelir: `send_stream` final yerleşimde tekrar elemesinden geçmiş hâllerini verir.
- `thought_counter(dusunce)` — "Düşünüyorum... Şu ana kadar N kelime düşündüm." `code_blocks(metin)` — thinking'in kod blokları (1900'e bölünmüş). `thought_display(metin)` — butonun ephemeral yanıtı: tek mesaja sığan kod bloğu, uzunsa kısaltma notu.
- `State::link_thought(mesaj, dusunce)` — gizle kipinde butonun bulması için düşünceyi son mesaj id'sine bağlar (`thought_store` 50 giriş, `thought_order` ile eskiden düşer). `Handler::interaction_create` — `THOUGHT_BUTTON` tıklanınca depodan alır, yalnız tıklayana görünen ephemeral kod bloğu gönderir.
- `ThinkingMode { Show, Hide, Silent, Off }` (bot/types/types_chat_state.rs) — düşünme kipi; `from_arg` komut argümanı çözer, `read`/`file_value` `durum/dusunme.md`, `label` ekran adı. Silent kipte reasoning normal istenir (kapatılmaz), yalnız `send_stream` düşünceyi hiç toplamaz/göstermez (placeholder/sayaç/buton yok) — göster/gizle'nin ekranda gösterdiği şeyi arka planda bırakır. Off kip `Bot::disable_reasoning` ile isteğe `reasoning.enabled=false` + `enable_thinking=false` ekler (yalnız stream/sohbet yolunda kipe bakılır; `ask_raw`'ın stream olmayan yolu — arka plan ajanları — kipten bağımsız her zaman kapatır, çünkü o yol reasoning'i zaten hiç okumaz).
- `reply_budget!()` — makro; sohbet cevabı token bütçesi derleme durumuna göre: release `Some(REPLY_CAP=4096)` (sıradan cevap altında kalır, yalnız tekrar/döngü gibi kaçak durumları keser), debug `Some(2000)` (maliyet koruması).
- `extract_json(&str) -> &str` — ilk `{` ile son `}` arası (kod bloğu süsünü atar).

### OpenRouter (impl Bot)
- `ask_raw(Value, kategori) -> Result<String>` — POST `/chat/completions`, `choices[0].message.content`; boşsa hata. Tek HTTP noktası. Zaman aşımı: bağlantı 15 sn, iki veri arası 120 sn, toplam sınır yok (uzun düşünme kesilmez). Ağ hatası / 429 / 5xx'te (`status_retryable`) 2+4 sn geri çekilip `AI_RETRIES` kez yeniden dener; reasoning zorunlu bir modelde 400 dönerse (`reasoning_mandatory_error`) alanları kaldırıp yeniden dener — bu durumda `apply_budget_floor` ile `max_tokens` (varsa) `REASONING_MANDATORY_BASE`'e (500) çıkarılır, yoksa küçük bütçeli mini-çağrılarda (20-80 token) reasoning bütçenin tamamını yiyip `content: null` bırakır. Aynı sebeple 200 dönüp içerik boş gelirse de (reasoning bütçeyi yemiş olabilir) hata hemen dönmez: bütçe tabana çıkarılıp bir kez daha denenir, taban da yetmezse `AI_RETRIES` sonunda pes edilir. Başarılı yanıtın `usage`'ı `kategori` ile `add_metric`'e gider (`!durum` kırılımı). `ask_raw_stream` de aynı mantıkta (yalnız akış açılmadan önce; boş-içerik-sonrası yeniden deneme yalnız `ask_raw`'da var, stream tarafı `send_stream`'de ayrıca ele alınır).
- `disable_reasoning(govde, force) -> bool` — kip Off ise ya da `force=true` ise sağlayıcıya göre düğme: openrouter `reasoning.enabled`, mistral'e bir şey gitmez, diğerleri (qwen tarzı router) `enable_thinking:false`. `ask_raw` (stream olmayan) her zaman `true` geçer — o yol `reasoning_content`'i zaten okumaz, kullanıcı kipi ne olursa olsun kapatır; `ask_raw_stream` `false` geçer, yalnız kip Off ise kapatır. Alanları gerçekten eklediyse `true` döner (mandatory-reasoning yeniden denemesi için).
- `BusyGuard` — `reply` kanalın meşgul bayrağını RAII ile bırakır: normal/erken dönüş ve panikte Drop çalışır; yeni tur için `drop(_busy_guard)` + üstte yeniden insert.
- `strip_name(metin, bot_adi)` — ad öneki + tırnak soyma; char güvenli (bayt dilimi yok), `casefold` İ→i̇ birleşik noktasını atarak karşılaştırır.
- `ask(sistem, gecmis, max_tokens, kategori)` — `system` + geçmiş → `ask_split` (bütçe `Some`).
- `generate(gecmis, talimat, butce: Option<u32>, kategori)` — **kişilikle konuşan tek yol.** `chat_system` ile sistem mesajını kurar → `ask_split` → `clean`. Bütçe `None` ise max_tokens gitmez (yalnız kimi tekil çağrılarda; sohbet cevabı `reply_budget!()` ile hep `Some`). Çağıranlar: stream yedeği/tekrar denemesi, poke, prank, haber tanıtma, hoş geldin, uyandım, gezgin notu, image_commenter yedeği, isim.
- `chat_system(gecmis, talimat) -> (sabit, degisken, bot_adi)` — geçmişteki `user` mesajlarından katılımcı adlarını (`"isim: "` öneki) ve metinleri çıkarır → `memory::keywords` → kilit altında `memory::retrieve` + `system_text`. `generate` ve `generate_stream` ortak kullanır.
- `ask_raw_stream(sabit, degisken, gecmis, butce, kategori) -> Result<StreamReader>` — `stream:true` POST; hata kontrolü `ask_raw` ile aynı. `Chunk{text,thought}` döndüren `StreamReader::next()` SSE satırlarını çözer (`extract_sse`; reasoning `reasoning` ya da `reasoning_content` alanından), utf-8 chunk ortasında bölünse de tamponda bekletir.
- `memory_cycle(bot)` — 10 dakikada bir, uyku kontrolüne takılmaz: uykudaysa 2 saatte bir gece gözlemini kuyruğa koyar; sonra `memory_queue`'yu işler (`diarist`, biten sohbette + `critic`).
- `generate_stream(gecmis, talimat, butce, kategori) -> Result<(StreamReader, bot_adi)>` — sohbet cevabını akış olarak açar. Çağıran: yalnız `reply`.
- `send_stream(ctx, kanal, reader, StreamContext) -> Result<StreamResult>` — parçaları biriktirir (kapalı kipte reasoning biriktirilmez), `STREAM_EDIT_INTERVAL` aralıkla `stream_view(..., bitti=false)` + `write_stream`; bitince `parse_reply(strip_name(...))`:
  **silent** (satır yok VE tepki yok) → açılan geçici mesajlar `delete_messages` ile silinir, `StreamResult::Silent`; (`-` ile `tepki: 💀` birlikte gelirse susma değil: emoji yine düşer)
  **boş** (ne satır ne tepki ne sus) → aynı temizlik, `StreamResult::Empty`;
  **tekrar** artık satır bazlı: son 5 bot satırıyla aynı olan satırlar düşer, hiç satır kalmaz ve tepki de yoksa bir kez `generate` ile yeniden üretim, o da tekrarsa sil + Empty;
  final `stream_layout` + `write_stream`; tepki varsa `context.reaction_target` mesajına `ctx.http.create_reaction(..., ReactionType::Unicode(emoji))` (hata yalnız warn log, akış durmaz — yalnız tepki de geçerli bir cevaptır); kayıt: her görünen satır ayrı ayrı `own_messages` + `channel_note`, tepki `"{bot}: tepki: 💀"` satırı olarak (tohum tutarlılığı), thinking hiç girmez. Döndürdüğü `Sent(String)` içeriği `Reply::protocol_text()`.
  `StreamResult::{Sent(String), Empty, Silent}`; `StreamContext{bot_name, reply_to, reaction_target, history, instruction, budget}` argüman yığını yerine tek yapı. `reaction_target` `reply_to`'dan ayrı bir alandır çünkü `reply_to` koşulludur (yalnız etiket/kalabalık durumunda dolu), tepkinin ise her zaman düşeceği bir mesaj gerekir. `reaction_target` **her zaman** sohbetin `last_message`'ıdır; `reply_to` ise `pick_target` bir kişi seçtiğinde o kişinin mesajına kayar — yani ikisi ayrışabilir: cevap erdem'e reply olarak bağlanırken emoji son yazana düşebilir. Bilinçli: hedef seçimi cevabın muhatabını değiştirir, tepki hâlâ "az önceki mesaja" düşer.
- `send_lines(ctx, kanal, ham, reply_to, reaction_target, ping) -> Option<String>` — **stream OLMAYAN yolların ortak göndericisi.** `strip_name` + `parse_reply` yapıp `send_reply`'e devreder. `send_reply(ctx, kanal, Reply, reply_to, reaction_target, ping)` gövdedir: elinde zaten çözülmüş/tekrar elenmiş `Reply` olan yollar (reply'nin yedek dalı) metne geri dönmeden bunu çağırır. Satırlar sırayla ayrı mesaj (`send`); aralarına `LINE_DELAY_BASE + LINE_DELAY_PER_CHAR × karakter` (tavan `LINE_DELAY_CAP`) bekleme ve `broadcast_typing` girer — stream'in kendi temposu burada yok, satırlar aynı anda düşmesin. `reply_to` yalnız ilk satıra takılır; `ping` de öyle ama **protokol çözüldükten sonra**, gönderim anında ilk satırın başına `<@id> ` olarak eklenir (metne baştan yapıştırılınca "`<@id> -`" susma işareti, "`<@id> tepki: 💀`" de tepki satırı sayılmıyordu). `reaction_target` yoksa tepki düşürülür (kanalda görünmeyecek bir tepki "gönderildi" sayılmasın). `silent` ya da gidecek hiçbir şey kalmayan cevapta hiçbir şey gitmez, `None` döner. Döndürdüğü `protocol_text` çağıranda sohbet açılış metni olur. Çağıranlar: `reply`'nin yedek `generate` dalı, `post_problem`, `send_news` (tanıtım), `poke_cycle` (OUT_OF_THE_BLUE/ON_THE_WAY/LEAVING), `guild_member_addition` (hoş geldin, ping'li), `sleep_transition` (WOKE_UP), `evaluate_waking`, `pick_name` (duyuru). `None` dönerse o açılış atlanır ve sohbet açılmaz (debug log).
- `write_stream(ctx, kanal, &mut Vec<Message>, yerlesim, reply_to)` — serbest fonksiyon. Yerleşimi açık mesajlarla uzlaştırır: değişeni `EditMessage` ile düzenler, eksiği açar (yalnız ilk mesaj yanıt/mention taşır), fazlasını siler; typing burada atılmaz (edit döngüsünde tekrarlanırdı, discord hız sınırına takılıyordu — `reply` model çağrısından önce bir kez atar). `delete_messages(ctx, Vec<Message>)` açılanları geri alır.
- `analyze(metin, talimat, max_tokens, kategori)` — **kişiliksiz tek yol.** Sistem = `ANALYST`; kullanıcı mesajı = `metin + "---" + talimat`. Çağıranlar: profiler, diarist, coach, critic, summarizer, news_agent seçim, wanderer seçim, waking değerlendirme.
- `willingness() -> Option<(u8, String)>` — "bu konuşmaya katılayım mı?" mini değerlendirmesi: profil+dizin sabit blokta (cache_control), son 12 mesaj değişken → `ask_split(..., 80, "isteklilik")` → `parse_willingness` JSON'dan 0-10. Çağıran: `Handler::message` (kanal başına en sık 2 dk, `last_evaluation`, yalnız farklı biri yazdıysa ya da sohbet yoksa). Hata/bozukta `None` → yedek zar.
- `pick_target(bekleyenler) -> Option<String>` — 2+ farklı kişi yazınca kime dönüleceğini seçer: son 12 mesaj + bekleyen isimler → sabit blok TARGET_PICK{ad}, değişken bekleyenler → `ask_split(..., 40)` → `extract_target` (JSON ya da düz metin, bilinen adlarla eşleştirilir). Çağıran: `reply`; seçilen kişinin mesajı `reply_to` olur, talimata not girer.
- `determine_mood(gecmis) -> Option<String>` — bu sohbetin ruh halini belirler: ANALYST sabit, MOOD{ad} değişken, sohbetin kendi geçmişi mesaj listesi olarak gider (görseller bu kopyada `None`'lanır: 40 token'lık analize resim yükü yollanmaz, vision'suz route hataya düşmesin) → `ask_split(..., 40, "ruh_hali")` → `extract_mood` (yoğunluk <3 ise None, nötr sayılır). Çağıran: `reply`, yalnız sohbet açılırken (`counter==0`) ve her 4 turda bir; sonuç `Chat.mood`'a yazılır ve talimata "ŞU ANKİ RUH HALİN" satırı olarak eklenir.
- `send(ctx, kanal, metin, ping, dosya, reply_to: Option<MessageId>)` — `reply_to` verilirse discord yanıtı (`reference_message`) olur ve yanıtlanan kişi pinglenir (`replied_user`).  mention'lar kapalı (`CreateAllowedMentions::new()`, yalnız `ping` açılır), isteğe bağlı ek dosya; başarılıysa `own_messages`'a (50) ekler. Kilit gönderimden SONRA alınır.
- `Bot::ask_split(sabit, degisken, gecmis, butce: Option<u32>)` — sistem mesajını `system_json` ile iki metin bloğu olarak gönderir, ilki `cache_control: ephemeral`; bütçe `None` ise max_tokens yok.
- `system_json(sabit, degisken) -> Value` — değişken boşsa düz system, değilse iki blok. Serbest fonksiyon.
- `Bot::is_repeat(kanal, cevap)` — kanal geçmişindeki son 5 bot satırıyla aynı mı. `Bot::research(metin) -> Option<String>` — link/haber/araştır tetiklerine göre sayfa, RSS ya da Firecrawl arama sonucu.
- `system_text(&State, talimat, getirilen) -> (String, String)` — (sabit, değişken); bölümleri sırayla ekler (mimari.md listesi). Serbest fonksiyon, kilit çağıranda.

### Sohbet motoru
- `Bot::post_problem(ctx, kanal)` — `generate(PROBLEM, 160)` ile uydurma kod derdi, gönder, sohbet aç. Poke döngüsü (%25) ve `/sorun`.
- `start_chat(&mut State, kanal, acilis: Option<String>) -> &mut Chat` — kanal geçmişinin son 10 satırıyla tohumlar (bot satırları assistant). Açılış zaten gönderilip geçmişe SATIR SATIR düşmüş olur: tohumun sonundaki bot bloğu taranır ve açılışın satırlarıyla eşleşenler atılır (araya haber linki gibi başka bir bot mesajı girmiş olabilir), böylece açılış modele iki kez görünmez;  varsa mevcut sohbeti döner (`entry().or_insert`), yoksa yeni; açılış varsa `assistant` mesajı + `counter=1`.
- `end_chat(&mut State, kanal) -> Option<Chat>` — haber bekleme silinir, sohbet çıkarılıp döner; kanal yasağı yok, kapatma `close_timed_out`'tan gelir.
- `Bot::close_timed_out(ctx)` — dakika tikinde: `CHAT_TIMEOUT` (30 dk) sessiz kalan sohbetleri meşgul değilse kapatır, dökümü `diarist`+`critic`'e verir, `growth.chats++`; ayrıca süresi dolan haber sohbetlerini temizler (yorum penceresi geçmiş + o pencerede aktivite yoksa sessizce kapanır, `awaiting_comment` haritası şişmez).
- Komutlar → `src/command.rs` içinde (aşağıda, slash komut yöneticisi).
- `Bot::post_news(ctx, kanal) -> bool` — news_agent → link → tanıtım → gönder → sohbet + 2 saat yorum bekleme. `news_cycle` ve `/haber` çağırır.
- `Bot::run_prank(ctx, kanal, hack)` — görsel seç, hack ise `HACK_ENTER`, değilse `image_commenter`; metin `parse_reply`'den geçer ve yalnız **ilk satır** alınır (görsel tek mesajda gider, satır patlaması burada anlamsız); model sustuysa şaka atlanır; gönder; sohbet (`hacked=3`). `prank_cycle` ve `/saka`/`/hack` çağırır.
- `Bot::reply(ctx, kanal)` — döngü: (1) kilit: meşgulse çık; sohbet yoksa çık; talimat seç ve meşgul işaretle. (2) 0,15-0,35 sn mesaj biriktirme payı; güncel geçmiş, hedef mesaj, `incoming`; ruh hali (4 turda bir), `research` bulgusu ve hedef kişi notu göreve eklenir; `too_many_questions` ise "bu sefer soru sorma" talimatı; `broadcast_typing`. (3) `generate_stream` (bütçe `reply_budget!()`) ile stream açılır. (4) `send_stream` (`reaction_target = last_message`): her satır ayrı mesaj, thinking kipe göre, tekrar eden satırlar düşer. (5) `Silent` → geçmişe/sayaca/`last_activity`'ye **hiçbir şey yazılmaz**, yedek `generate` çağrılmaz; yeni mesaj varsa bir tur daha, yoksa çık. (6) `Empty` → yeni mesaj varsa bir tur daha, yoksa `generate` + `send_lines` ile stream'siz yedek (o da susarsa çık). (7) meşgul kaldır, asistan satırı (`protocol_text`) ekle, sayaçları ilerlet, `last_activity` tazele. Yeni mesaj geldiyse başa dön. Kapatma yok; sessiz kalan sohbeti zaman aşımı kapatır.

### Hafıza (discord tarafı)
- `read_history(bot, ctx, guild)` — botun üyeliğini çeker, izinli (`VIEW_CHANNEL|READ_MESSAGE_HISTORY`) metin kanallarını pozisyon sırasıyla gezer, `GetMessages` 100'lük sayfalarla 14 gün geriye okur, bot/boş mesajları atlar, `content_safe` ile mention'ları ada çevirir, zamana göre sıralar; favori id görürse `favorite_name` yazar, `name_to_id` yalnız boşsa dolar (canlı eşleme öncelikli). Tarih hafızanın ÖNÜNE eklenir (tarama sürerken gelen canlı mesajlar arkada kalır, kronoloji bozulmaz, boca canlıları ezmez).
- `default_channel(bot, ctx) -> Option<ChannelId>` — `news_channel` → sunucu sistem kanalı → en üst metin kanalı. Önbellekten, await yok.
- `idle_channel(bot) -> Option<(ChannelId, String)>` — son konuşulan kanal; sohbet açık değil, yasaklı değil, profil var → (kanal, son 40 satır). Poke ve prank bunu kullanır.

### Döngüler (`run_cycle`, `ready`'de bir kez)
`run_cycle(ad, kur)` — her döngüyü bekçiyle başlatır: panikte log + 5 sn sonra yeniden,
temiz dönüşte de yeniden (döngüler sonsuzdur). `SHUTTING_DOWN` (AtomicBool) kapanış sinyali:
döngüler tik başında bakar ve döner, bekçi yeniden başlatmaz; `main`'in kapanış görevi kurar.
- `news_cycle(bot, ctx)` — 6 saatte bir: **uykudaysa** haber seçer ama atmaz, `stashed_news`'e koyar (bir kez); seyahatteyse profiler+coach, geç; uyanıkken: profiler → gözlem kuyruğa → coach → varsayılan kanalda sohbet açıksa geç → stok varsa `send_news(stok)`, yoksa `post_news`.
- `poke_cycle(bot, ctx)` — saatte bir: uyanık değilse geç; seyahatteyse günde bir kez %25 ile `ON_THE_WAY`; yarın seyahat başlıyorsa bir kez `LEAVING`; değilse %30 ile `OUT_OF_THE_BLUE`; `idle_channel` → `generate(son 40 satır)` → gönder → sohbet başlat.
- `prank_cycle(bot, ctx)` — 3 saatte bir: uyanık değilse/seyahatteyse geç; %10; `idle_channel`; `random_image` yoksa geç; %30 hack: `generate(HACK_ENTER)`, değilse `image_commenter(resim)`; görselle gönder; sohbet başlat, hack ise `hacked = 3`.
- `wanderer_cycle(bot)` — ilk 10 dk sonra, sonra 4 saatte bir, uyanıksa `wander`.
- `Bot::sleep_transition(ctx)` — uyudu/uyandı geçişini loglar; uyurken `sleep_start`+`sleep_start_memory_len` işaretlenir. Uyanışta: bekleyen etiket varsa `WOKE_UP` ile dönüş (hata durumunda liste geri konur); yoksa `evaluate_waking` gece mesajlarını değerlendirir. Döngü ve `/uyan`/`/uyu` çağırır.
- `Bot::evaluate_waking(ctx, gece)` — `analyze(WAKING{ad}, 100)` → `{"ilgi","konu"}`; ilgi ≥5 ise `generate(WAKING_REPLY{ad,konu}, 250)` ile son konuşulan kanala sabah sözü + sohbet.
- `sleep_cycle(bot, ctx)` — dakikada bir: `sleep::update`, uyandı/uyudu geçişini loglar; uyanınca `pending_mentions` varsa son etiketin kanalına `generate(WOKE_UP)` ile döner, sohbet başlatır.

### Discord olayları (Handler)
- `ready` — bot adını yazar; her gelişte sunuculara slash komutları kaydeder (`modal::register_commands`, idempotent); `started` ilk kez ise beş döngüyü başlatır.
- `interaction_create` — `Command` → `command::definitions()` tablosunda adına göre bulunur ve çalıştırılır (her komut kendi embed yanıtını üretir); `Modal` → kısa ephemeral onay (girdi toplanmaz); `Component` → düşünce butonu (`thought_button`) ya da zihin detay katmanı (`MIND_TOPICS/EVENTS/SUMMARY` butonları bölüm modalı, `MIND_PERSON_PICK` menüsü kişi modalı açar).
- `guild_create` — `scanned`'a ilk kez giriyorsa arka planda `read_history → profiler → coach (huy boşsa)`.
- `guild_member_addition` — kanal: sunucu sistem kanalı → varsayılan; favori ise adını kaydet; sohbet açık/yasaklıysa çık; `generate(WELCOME)` → mention'lı gönder (ping açık) → sohbet başlat.
- `message` — bot/webhook/DM ise çık; `GUILD_ID`/`CHANNELS` dışıysa çık; `content_safe`; **ek görsel:** `attachments` içinde `content_type`'ı `image/` ile başlayan ilk ekin URL'i alınır, erken çıkış "metin de ek de boş" hâline gelir (sırf resim atılmış mesaj işlenir); metin `[resim] <metin>` ya da `[resim attı]` olarak işaretlenir ve bu işaretli metin hafızaya, kanal notuna ve sohbet satırına aynen gider — URL yalnız sohbet geçmişindeki `ChatMessage.image`'a konur ve yeni kullanıcı satırı eklenirken önceki girdilerin `image`'i `None` yapılır (yalnız en son görsel modele gider). **1. faz (kilit):** etiketlendi mi (mention listesi, yanıtlanan mesaj botun mu, metinde bot adı geçiyor mu) → `remember`, `name_to_id`/`usernames`, `last_channel`, favori adı; haber bekleme süresi dolduysa sohbeti kapat; **uyuyorsa**: etiketlendiyse `pending_mentions`'a (20) ekle, çık; `ongoing_dialog` — sohbet açık VE sohbetteki son user mesajının sahibi bu mesajı atanla aynı isimse (gerçekten kendisiyle konuşuyor) → doğrudan cevaplanır, isteklilik değerlendirmesi atlanır. Etiket de aynı şekilde doğrudan cevaplanır. İkisi de değilse (kanalda başka biri yazdı, ya da sohbet yok) isteklilik değerlendirmesi gerekir (kanal başına en sık 2 dk). **2. faz (kilitsiz):** gerekiyorsa `willingness()`; puan ≥ eşik (evre ±1, seyahat +2) ise katılır; çağrı yoksa yedek zar (`CHANCE`). **3. faz (kilit):** katılıyorsa `start_chat`, kullanıcı satırını geçmişe ekle (20'de tut), `channel_note`. Kilit dışı: `reply`. Not: bu, bir kez açılan sohbette kanaldaki HERKESE otomatik cevap verme davranışını (eski tasarım) kaldırır — yalnız gerçek muhatabına.

### Başlangıç
- `setting(isim)` — boş olmayan env değişkeni ya da açık hata.
- `wait_for_shutdown()` — ctrl-c veya SIGTERM.
- `Bot::setup() -> Result<Arc<Bot>, BotError>` — sağlayıcı seçimi (`PROVIDER`/anahtarlar/`MODEL`/`API_URL`), `NEWS_CHANNEL`/`GUILD_ID`/`CHANNELS`, `durum/{kisiler,konular,olaylar,arsiv,kanallar}` + `resimler/` klasörleri, `State::load` + `sleep::update` + `durum/model.md`, reqwest istemcisi. **Discord'a bağlanmaz, DISCORD_TOKEN istemez**: hem `main`'in bot yolu hem `cargo run -- chat` buradan geçer (ikisi aynı kurulumu görsün diye tek fonksiyona çıkarıldı).
- `main` — `.env`, loglama, panic hook; ilk argüman `chat` ise `Bot::setup()` + `Bot::chat_cli()` (kurulum hatasında tek satır mesaj + çıkış kodu 1) ve döner. Değilse `DISCORD_TOKEN` + `Bot::setup()`, intents `GUILDS|GUILD_MESSAGES|GUILD_MEMBERS|MESSAGE_CONTENT`, kapanışta `shard_manager.shutdown_all`.

- `version_text()` — `v{CARGO_PKG_VERSION} ({VERSION_COMMIT}, {VERSION_DATE})`; iki env'i `build.rs` derlemede git'ten doldurur (git/date yoksa `?`). `modal::status_message` ve `guild_create` sürüm duyurusu kullanır.

## src/command.rs (+ src/command/*.rs) — slash komut yöneticisi
Bot yalnız slash (`/`) komutlarla yönetilir; `!`/metin komut yok, `Handler::message` artık komut
ayrıştırmaz (mesajlar doğrudan sohbet/hafıza akışına girer). Tek kayıt tablosu `definitions()`:
her `CommandDefinition` ad + açıklama + Discord seçenekleri (`CreateCommandOption`) + çalıştırıcı
(`define_command!` makrosuyla `fn(&Bot,&Context,&CommandInteraction) -> Pin<Box<dyn Future<...>+Send>>`
kaydeder) taşır. `modal::register_commands` bu tablodan Discord'a kayıt listesi çıkarır,
`interaction_create` (main.rs) `Interaction::Command`'ı isme göre tabloda bulup çalıştırır — komut
adı iki yerde elle tutulmaz. (Slash komut ADLARI — `durum`, `zihin`, vb. — Türkçe kalır; Discord'a
çıkan yüzey, bkz AGENTS.md madde 8.)
- Komutlar: durum · yardim · zihin(`test`) · ayarlar · sifirla(`hepsi`) · haber · sorun · gez ·
  saka · hack · ajanlar · uyan · uyu(`saat`) · dusunme(`kip`) · model(`id`) · debug(`durum`).
- Yanıt: her komut embed döner, düz metin yok. Yerel/hızlı komutlar (durum/yardim/ayarlar/zihin
  varsayılan görünüm/sifirla/dusunme/model-sorgu/debug) doğrudan `CreateInteractionResponse::Message`
  ile yanıtlar (`send_response`/`reply_info`, embed `modal::info_embed`). Ağ/model çağrısı yapan
  komutlar (haber/sorun/gez/saka/hack/ajanlar/uyan/uyu/zihin `test`/model id değişimi) Discord'un
  3 sn'lik ilk yanıt sınırını aşabileceği için önce `defer` (`Defer`) ile anında onay verir, işi
  bitirince `report_result` (`edit_response`) ile kısa bir sonuç embed'i yazar — asıl içerik
  (haber/şaka/vb.) zaten kendi `Bot::send` çağrısıyla kanala gidiyor, bu yalnız bir "tamam" notu.
- `zihin` `test:true` seçeneği eski `!zihin test` teşhisini yapar: kanalın son 30 satırını hemen
  günlükçüye verir (zihin zinciri 40 dk beklemeden görülsün); seçenek yoksa `modal::mind_message`
  (kişi menüsü + bölüm butonları → detay modalları).
- `dusunme` seçenek değerleri (`goster/gizle/sessiz/kapat`) `ThinkingMode::from_arg`'ın tanıdığı
  dizgelerle birebir aynı tutulur (test: `thinking_mode_options_match_from_arg`); argümansız çağrı
  mevcut kipi söyler.
- `model id`: yalnız FAVORITE değiştirebilir; `model_exists(id)` OpenRouter `/models` listesinde arar,
  liste çekilemezse engel olmaz.
- `Bot::wake/put_to_sleep/set_debug` (durum yazan yardımcılar) `impl Bot` içinde kalır, komutlar
  bunları çağırır.

## src/chat_cli.rs (impl Bot)
Terminal sohbet tezgâhı: discord'a bağlanmadan çıktı protokolünü (satır = mesaj, `tepki:`, `-`)
denemek için. `cargo run -- chat` ile açılır (`main`).
- `CLI_CHANNEL = 1` — sahte kanal id'si; gerçek bir discord kanalı değil, yalnız sohbet durumunun anahtarı.
- `append_history(&mut State, kanal, satir)` — kanal geçmişine **yalnız bellekte** ekler (`CHANNEL_HISTORY` sınırıyla). `channel_note` aynı işi yapıp diske de yazdığı için burada kullanılmaz: tezgâh gerçek `durum/kanallar/*.md` dosyalarını kirletmemeli. (`remember` zaten bellek içi, o çağrılır.)
- `parse_line(satir) -> (isim, metin)` — `"isim: metin"`; iki nokta yoksa ya da bir yanı boşsa yazan `misafir`, satırın tamamı metin.
- `Bot::chat_cli(&self)` — `bot_name` boşsa (`ready` hiç gelmiyor) `growth.name`, o da yoksa `"bot"`; `start_chat` gerçek `durum/` dosyalarından tohumlar (kişilik gerçekçi kalsın). stdin bloklayarak satır satır okunur (bu kipte arka plan döngüsü yok, ayrı okuyucuya değmez); `!quit` ya da EOF çıkar. Her tur: `remember` + bellek geçmişi + sohbet geçmişine `user` satırı → `too_many_questions` talimatı → `generate` (**stream yok**: burada akış temposu değil çıkan protokol ölçülüyor) → `strip_name` → `parse_reply` → her satır `"bot_name: satır"`, tepki `[reaction 💀]`, susma `(silent)`, hiçbir şey `(empty)`, model hatası `(error: …)` (döngü sürer) → geçmişe `protocol_text` iter, `counter++`.
- Testler: `line_parses`, `history_limited_in_memory`.

## src/modal.rs
Komut arayüzü: slash komutlar ephemeral **embed kart** döndürür (web sayfası gibi bölümlü),
detaylar **etiketli modal alanlarına** dağıtılır — tek metin kutusuna her şey boca edilmez.
Discord sınırları: embed field value ≤1024, modal ≤5 bileşen × value ≤4000, başlık/etiket ≤45,
select menü ≤25 seçenek (etiket ≤45, açıklama ≤100).
- `info_embed(baslik, aciklama)` — komutların kısa onay/durum yanıtı (ör. "haber bulamadım", "tamam, {model}"); command.rs'teki `reply_info`/`report_result` bunu sarar, düz metin gitmez. `token_breakdown(m)` — kategoriler toplam tokene göre sıralı; `status_message` ve `summary_modal` kullanır.
- `mind_embeds(state)` — tek kart: description evre/gün/model/kip; üç satır içi alan: Kişiler (ilk 8: ad+puan+etiket), Konular (ilk 8: ad+son not), Olaylar (son 5, en yeni aydan kronolojik); footer bot adı + tarih. Boş alan "—".
- `mind_components()` — 1. satır: kişi select menüsü (`MIND_PERSON_PICK`, ≤25 kişi, değer=id, açıklama etiket+not); 2. satır: Konular/Olaylar/Bot özeti butonları.
- `mind_message(state)` / `status_message(state)` / `help_message()` — ephemeral `CreateInteractionResponseMessage` (embed + bileşen).
- Detay modalları (`person_modal(id)` / `topics_modal()` / `events_modal()` / `summary_modal(state)`) — her konu kendi etiketli alanında: kişi = Kimlik/İzlenim/Etiketler/Bildikleri(son 8)/Son olaylar(son 5); konular = Son değişenler(15)+Diğer; olaylar = ay başına alan (son 3 ay, her ayın son 10 kaydı, başlık "Eylül 2026"); özet = Durum/Token/Kendim/Gündem. Boş bölümler atlanır, hepsi boşsa tek "(henüz boş)" alanı.
- `fit_to_limit(metin, sinir)` — sınır aşımı son satır/boşluk hizasında kesilir + not. `month_name("2026-09")` → "Eylül 2026".
- `register_commands(http, guild)` — sunucu komutları `command::definitions()` tablosundan çıkarılır (ad/açıklama/seçenekler tek kaynak); her ready'de idempotent.
- `memory.rs` yardımcıları: `person_summaries` (mtime sırası `Person` listesi), `topic_summaries` (ad + son not), `event_months(n)` (son n ayın "- " satırları, en yeni ay başta).

## src/agents.rs (impl Bot)
- `profiler()` — son 600 satır → `analyze(PROFILE_EXTRACT, 1200)` → `profil.md` + `State.profile`.
- `diarist(dokum, kaynak, kanal)` — `analyze(DIARIST{ad,kaynak,favori}, 1200)` → JSON `Record{olay, kisiler[{isim,puan_degisimi,not,bilgiler,etiketler}], konular[{ad,not}], kendim}` (alan adları model çıktısıyla eşleşsin diye Türkçe bırakıldı, bkz promptlar/gunlukcu.md); JSON çözülemezse ham çıktı `arsiv/gunlukcu-<kaynak>.md`'ye kurtarılır (emek kaybolmaz). → olay satırı (`memory::add_event`, saniyeli), her kişi: isim `State.name_to_id` ile id'ye çevrilir (çözülemeyen atlanır, loglanır), `kisiler/<id>.md` oku, ad değiştiyse eskisi `previous_names`'e, puan += clamp(-3..3) sonra clamp(-10..10), not/bilgiler/etiketler, favori ise +10 ve sabit not, `memory::write_person`; konular `memory::add_topic`; kendim → `kendim.md`; dizin yenile; sonra `summarizer`.
- `summarizer()` — `memory::over_limit()` için: kişi → `analyze(SUMMARIZER_PERSON{sinir=1000}, 700)`, konu → `SUMMARIZER_TOPIC{800}`, olay → eski %60 satır `SUMMARIZER_EVENTS` ile 3-5 satıra, yeni %40 kalır. Sonuç boş değil ve eskisinden kısaysa: kişi/konu için eski dosya arşive, yeni yazılır; olayda taşınan satırlar arşive. Küçülmediyse dokunmaz. Dizin yenile.
- `send_news(ctx, kanal, item) -> bool` — seçilmiş haberi paylaşır (tur haberi ya da uyku stoku); tanıtım `generate(NEWS_INTRO)`, sohbet açılır, 2 saat yorum beklenir.
- `coach()` — profil + dizin + gündem + kendim + mevcut huy + son 200 satır + botun son mesajları → `analyze(COACH{ad}, 800)` → `huy.md`.
- `critic(dokum)` — `analyze(CRITIC{ad,mevcut}, 400)` → `duzeltmeler.md`.
- `news_agent() -> Result<News>` — HN ilk 12 (atılmamış) + Sözcü RSS ilk 12 (atılmamış, kimlik = link hash) → liste "n. [hn|gündem] başlık" → `analyze(NEWS_PICK{profil}, 10)` → numara → `News{id,title,url,score,source}`.
- `image_commenter(&PathBuf) -> Result<String>` — görseli base64 `image_url` olarak sistem=`system_text(IMAGE_POST)` ile `ask_raw`; hata olursa `generate` ile körlemesine. `clean`.
- `random_image() -> Option<PathBuf>` — `resimler/` içinden png/jpg/jpeg/gif/webp.
- `News` — serde; `source` `#[serde(skip)]`, `score` HN dışı 0.

## src/memory.rs
- Sabitler: `PERSON_LIMIT 1800 / PERSON_TARGET 1000 / TOPIC_LIMIT 1500 / TOPIC_TARGET 800 / EVENT_LIMIT 6000 / CONTEXT_BUDGET 6000 / INDEX_PEOPLE 40 / FAVORITE_NOTE`.
- `path(parca)`, `read(parca)`, `write(parca, icerik)` — `WRITE_LOCK` (static Mutex) ile tek sıradan ve atomik (geçici + rename); `append(parca, satir)` — gerçek append (OpenOptions), aynı kilit; `archive(parca, icerik)` (`arsiv/parca`'ya tarihli başlıkla ekler).
- `person_summaries` / `topic_summaries` / `event_months` — modal gösterimi için mtime sıralı dökümler.
- `slug(isim)` — küçük harf, Türkçe harf sadeleştirme, alfanümerik dışı `-`, boşsa "bilinmeyen".
- `date()`, `date_from_unix(unix)` (Hinnant civil-from-days), `month()` "YYYY-AA".
- `Person { id, name, username, previous_names, score, tags, note, facts, events }` — `parse(id, metin)` dosyadan, `text()` dosyaya; dosya `kisiler/<id>.md`. Biçim (dosya alanları Türkçe, bkz AGENTS.md madde 8): `# İsim` / `id:` / `kullanici_adi:` / `eski_adlar:` / `puan: +3` / `etiket: a, b` / `not: ...` / `## Bildiklerin` `- ...` / `## Son olaylar` `- tarih saat: ...`.
- `read_person(id)`, `write_person(&Person)` — `kisiler/<id>.md`.
- `add_topic(ad, not)` — `konular/<slug>.md`, yoksa başlık+etiket satırı, sonra `- tarih: not`.
- `add_event(kanal, olay)` — `olaylar/YYYY-AA.md`'ye `- tarih #kanal: olay`.
- `files(klasor)` — `.md`'ler, son değişen önce. `first_line(p)`.
- `refresh_index() -> String` — `## Kişiler` (≤40: `- ad (+p) · etiketler · not`), `## Konular` (≤30: `- ad · son: tarih`), `## Olaylar` (≤3 ay: `- YYYY-AA · n kayıt`); `INDEX.md`'ye yazar.
- `STOPWORDS` — elenen sık kelimeler. `keywords(&[String])` — 4+ harf, durak değil, tekrarsız, ≤40.
- `score_matches(metin, anahtar)` — kaç anahtar geçiyor. `trim(metin, sinir)` — karakter sınırı + `…`.
- `retrieve(katilimcilar, name_to_id, anahtar, hafiza, exclude_recent) -> String` — sırayla: katılımcıların kişi dosyaları (`name_to_id` ile id'ye çevrilir, ≤4, her biri ≤1200), en çok eşleşen 2 konu dosyası (≤800), ayın son 8 olayı, ham hafızadan (son `exclude_recent` hariç) ≥2 anahtarla eşleşen en fazla 12 satır (puan sonra yenilik sırası, sonra kronolojik). Bütçe 6000 karakter; sığmayan bölüm ve sonrası atlanır.
- `over_limit() -> Vec<(tür, yol)>` — boyutu aşan kişi/konu dosyaları ve bu ayın olay dosyası.

## src/agenda.rs
- Sabitler: `RSS_URL`, `AGENDA_ENTRIES 12`, `PAGE_LIMIT 3500`.
- `clean_html(ham)` — CDATA, script/style blokları, etiketler atılır; temel entity'ler; boşluk toplanır.
- `tag_content(parca, etiket)` — `<etiket>`/`<etiket ` … `</etiket>` içi, temizlenmiş.
- `rss(http) -> Result<Vec<RssNews{title,link,summary}>>` — `<item` bölerek; başlık ve http link şart.
- `link_id(link) -> u64` — DefaultHasher; atılan haber takibi için.
- `entries(metin) -> Vec<String>` — `gundem.md`'yi `## ` başlıklı girişlere böler. `latest_agenda(metin)` son 3 giriş.
- `Bot::read_page(url)` — firecrawl anahtarı varsa `POST api.firecrawl.dev/v1/scrape {url, formats:[markdown], onlyMainContent}` → `data.markdown`; yoksa `GET` + `clean_html`. 3500 karakter.
- `Bot::firecrawl_search(sorgu) -> Result<String>` — `POST /v1/search` limit 5; başlık, açıklama, adres satırları.
- `Bot::wander()` — rss ilk 20 → `analyze(WANDERER_PICK{ad,huy,profil}, 20)` → ≤3 numara → her biri `read_page` (hata: özet) → `generate(WANDERER_NOTE, 350)` (kişilikle, kendi günlüğü) → `gundem.md`'ye `## tarih saat` girişi; 12'yi aşan en eski giriş arşive; `State.agenda` = son 3.

## src/sleep.rs
- Sabitler: `TIMEZONE_OFFSET +3h`, `INSOMNIA_CHANCE 0.07`, `INSOMNIA_TENSE 0.20`.
- `Plan { day, insomnia_start: Option<i64>, start, end }` — bir gecenin planı (unix saniye).
- `local_time(unix) -> (gün no, gün içi saniye)`, `time()` "SS:DD", `time_text()` "YYYY-AA-GG günadı SS:DD".
- `jitter()` ±45 dk. `is_tense(&State)` — `myself`+`temperament` içinde kırgın/sinir/gergin/takıntı/uyku/kafayı/bunalt geçiyor mu.
- `build_plan(gun, tense)` — normal: 01:00±45 → 09:00±45; uykusuz: 01:00 ayakta, 06:00±45 → 13:00±45.
- `update(&mut State)` — dün ve bugün için plan yoksa kurar, biteni atar. `is_awake`, `status_text` ("ŞU AN" satırı).

## src/travel.rs
- `Trip { place, reason, start, end }` (yerel gün no). `Event` tablosu `EVENTS` (yıllık + yıla özel bayramlar 2026-2027).
- `day_number(y,m,d)` — Hinnant days-from-civil. `year_of(gun)`.
- `on_day(gun) -> Option<Trip>` — bu yıl ve geçen yıl (yılbaşı sarkması) için tabloyu tarar; yer = `(y + ay*31 + gun) % yerler.len()` ile sabit.
- `today()`, `now()`, `tomorrow()` (yarın başlayan, bugün olmayan). `status_text()` — "Şu an X'desin (...); n gündür, m gün sonra dönüyorsun" / "Yarın X'ye gidiyorsun" / boş.

## src/growth.rs
- `Stage { name, min_days, min_chats, confidence, poke, description }`, `STAGES` (4 evre), `NAME_STAGE = 2`.
- `Growth { birth, chats, messages, stage, name }` — `load()` `durum/gelisim.md`'den (yoksa doğum = şimdi), `save(&Growth)` `anahtar: değer` satırları.
- `days(&Growth)` doğumdan bu yana gün. `earned_stage(&Growth)` gün ve sohbet eşiklerini geçen en yüksek evre. `stage(&Growth)` mevcut evre. `stage_text` "GELİŞİM EVREN" bölümü.
- `clean_name(&str) -> Option<String>` — ilk kelime, alfanümerik, 2..20 karakter.
- `Bot::check_growth(ctx)` (main.rs) — her biten sohbet ve 6 saatlik turda: hak edilen evre > mevcut ise atlar, kaydeder; evre ≥ NAME_STAGE ve isim yoksa `pick_name`.
- `Bot::pick_name(ctx)` (main.rs) — `generate(NAME_PICK, 12)` → `clean_name` → her sunucuda `edit_nickname` → `growth.name`, `bot_name` → varsayılan kanala `generate(NAME_ANNOUNCE{isim})` + sohbet. Etiket algısı hem seçilen adı hem kullanıcı adını tanır.

## src/prompts.rs
Yalnız `pub const X: &str = include_str!("../promptlar/x.md");` satırları. Bkz docs/promptlar.md.

## Bugün eklenenler (2026-09-02, sürüm/debug/ayarlar/reasoning)
- `response_content(&Content, kategori)`, `thought_length`, `JSON_CATEGORIES`, `Bot::grow_budget`,
  `Bot::reasoning_low_effort` — reasoning zorunlu modelde ajan çağrısı dayanıklılığı (`ask_raw`).
- `agents::DiaristSummary`; `diarist` sonuç döner; `/zihin test:true` (command.rs).
- `State.debug`, `Bot.debug_channel`, `Bot::debug_note`, `Bot::debug_trace`, `Bot::set_debug`,
  `parse_willingness` (puan+sebep; `willingness` artık `Option<(u8, String)>`).
- `modal::settings_embed/settings_components/settings_message`, `SETTING_*` kimlikleri, `Handler::setting_button`,
  `Bot::wake/put_to_sleep` (komutlarla paylaşılan uyku yolları).
- **Aynı gün geri alındı**: panel görseli (`zihin_gorsel.rs`, `resvg`, gömülü fontlar, `Bot::gonder_ekli`,
  `cargo run -- zihin`) tamamen kaldırıldı, `/zihin` embed+buton+modal tek yol oldu; `!`/metin komutlar
  (`Bot::komut`) kaldırılıp yerine `command::definitions()` slash kayıt tablosu kondu (bkz docs/kararlar.md).

## 2026-09-03: kod İngilizceye çevrildi
Tanımlayıcılar, yorumlar, dosya/dizin adları İngilizceye çevrildi (bkz AGENTS.md madde 8 ve
dev/ilerleme.md). Bu dosyadaki tüm kod referansları yeni adlarla güncellendi. Türkçe kalanlar
değişmedi: `promptlar/*.md` (dizin+dosya adı+içerik), `durum/` dosya biçimleri (alan adları,
dosya adları — `Person`/`Record` gibi tiplerin JSON/dosya alanları model promptlarıyla eşleşsin
diye Türkçe bırakıldı), Discord'a çıkan her şey (slash komut adları, embed metni, buton/menü
etiketleri).
