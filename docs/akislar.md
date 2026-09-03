# Akışlar (olay → sırayla ne olur)

## Bir mesaj geldi
0. Her mesaj (bot dahil, `send` üstünden) kanalın geçmişine düşer: `channel_note` → bellek (60 satır) + `durum/kanallar/<id>.md`. Yeni sohbet açılırken son 10 satır tohum olur (`start_chat`), böylece sohbet bitmiş ya da bot yeniden başlamış olsa da bağlam kaybolmaz.
0. **Komutlar bu akışa hiç girmez.** Bot yalnız slash (`/`) komutlarla yönetilir; `Handler::message`
   artık metni komut diye ayrıştırmaz, her mesaj doğrudan aşağıdaki adımlara girer. Slash komutların
   akışı ayrı: Discord `Interaction::Command` gönderir, `interaction_create` (main.rs)
   `command::definitions()` tablosunda ada bakıp ilgili çalıştırıcıyı çağırır — bkz "Slash komutlar" bölümü.
1. `Handler::message`: bot/webhook/DM → çık; `GUILD_ID`/`CHANNELS` ayarlıysa dışarıdaki sunucu/kanal → çık. `content_safe` (mention'lar `@ad`, `@everyone` zararsız).
1b. **Resim eki:** `msg.attachments` içinde `content_type`'ı `image/` ile başlayan ilk ekin URL'i alınır. Erken çıkış artık "metin boş" değil "metin de ek de yok": sırf görsel atılmış mesaj da işlenir. Hafızaya/kanal notuna/sohbet satırına giden metin işaretlenir: metin varsa `[resim] <metin>`, yoksa `[resim attı]`. URL yalnız sohbet geçmişindeki `ChatMessage.image` alanına konur ve **yalnız en son kullanıcı mesajında** kalır (yeni satır eklenirken eskilerin `image`'i `None` olur: discord cdn linki ömürlü, eski görseli her turda yollamak token yakar). `message_json` bu alanı görürse istek gövdesinde `content` düz metin değil `[{text},{image_url}]` dizisi olur (agents.rs `image_commenter` ile aynı biçim).
2. Kilit içinde: etiketlendi mi? (mention listesi ∪ yanıtlanan mesaj botun ∪ metinde bot adı)
3. `remember` (ham hafıza), `last_channel`, favori adı.
4. Haber attıysa ve 2 saat dolduysa o sohbeti sessizce kapat (yasak yok).
5. **Uyuyorsa:** etiketlendiyse kuyruğa (≤20), cevap yok, çık.
6. Etiketlendiyse ya da **sürmekte olan diyalog**sa (sohbet açık VE sohbetteki son user
   mesajının sahibi bu mesajı atanla aynıysa — yani gerçekten kendisiyle konuşuyor) doğrudan
   cevap. Değilse (sohbet yok, ya da kanalda BAŞKA biri yazdı) **isteklilik değerlendirmesi**:
   kanal başına en sık 2 dakikada bir mini model çağrısı (`isteklilik.md`, ~80 token) son 12
   mesaj + profil + dizin üzerinden `{"puan":0-10}` üretir; eşik (`WILLINGNESS_THRESHOLD`, evre
   cesaretine göre ±1, seyahatte +2) üstündeyse sohbete girer. Çağrı başarısızsa yedek zar
   (`CHANCE`). Bu, açık bir sohbette kanaldaki HERKESE otomatik cevap vermeyi engeller — yalnız
   gerçek muhatabına.
7. Sohbet açıksa kullanıcı satırını geçmişe ekle (son 20).
8. Kilit dışı: `reply`.

## Bir tepki (reaction) atıldı
`Handler::reaction_add` (`GUILD_MESSAGE_REACTIONS` intent gerekir) her tepkide tetiklenir, ama
yalnız **botun kendi mesajına** atılan, insan kaynaklı, sunucu içi (DM değil) ve
`GUILD_ID`/`CHANNELS` filtresinden geçen tepkilerle ilgilenir — gerisi hemen çıkar. `Reaction`
olayı ne kimin attığını ne de tepki verilen mesajın metnini taşır: ikisi de `add_reaction.user`/
`.message` ile HTTP'den çekilir. Metni boş (yalnız embed'li) bir mesaja atılan tepki atlanır —
kartlara/durum satırlarına tepki "botun sözüne" sayılmaz. `reaction_label` emoji'yi okunabilir
hale getirir: unicode emoji olduğu gibi, özel sunucu emoji'si `:ad:` biçiminde (Discord'un ham
`<:ad:id>` mention biçimi değil). Sonuç `"(tepki 💀) \"...\" mesajına tepki verdi"` satırı olarak
`remember` (ham hafıza) ve `channel_note`'a düşer; sohbet o kanalda açıksa `chat.history`'ye de
eklenir (model bir sonraki cevabında bunu bağlam olarak görür). **Kendiliğinden cevap
tetiklemez** — isteklilik değerlendirmesi yok, yeni mesaj gitmez; bot yalnız fark eder, sıradaki
doğal cevabında değinebilir. `debug` açıksa `tepki: <isim> → <emoji>` izi düşer.

## Çıktı protokolü (her kişilikli cevap bundan geçer)
Model düz metin değil **satır bazlı bir protokol** yazar; `parse_reply` çözer (`strip_name`
uygulanmış metin üzerinde, yeniden soyma yok):
- **Her satır ayrı bir discord mesajıdır.** Boş satırlar atılır, en çok `BURST_LIMIT` (4) satır
  gider; fazlası düşer (debug log). 1900'ü aşan satır `split` ile kendi içinde bölünür.
- **`tepki: 💀`** satırı yazı olarak GİTMEZ, cevaplanan mesaja emoji tepkisi olur. Büyük/küçük harf
  ve "tepki :" boşluğu tolere edilir; iki noktadan sonraki ilk emoji dizisi alınır (harf, boşluk ve
  bilinen emoji bloklarından bir karakter — U+2600–27BF, U+2B00–2BFF, U+1F000–1FAFF, ©/®/™ gibi
  tekiller — + peşindeki varyasyon seçici/ZWJ/keycap, en çok 8 char). Tanım bilerek dar: `—`, `…`,
  `→`, tipografik tırnak emoji değildir, Discord bunlara 400 döner. `:kekw:` gibi özel emoji biçimi
  ve emoji bulunmayan satır sessizce düşer. İlk tepki kazanır.
- **Susma:** tek başına `-` (ya da `"-"`, `'-'`, `[sus]`, `(sus)`) satırı `silent` bayrağını kaldırır ve
  satır olarak gitmez. Yalnız `silent` varsa hiçbir şey gönderilmez.
- **Kırıntı ve slop:** `'` ile başlayan satır (önceki mesajın devamı) atılır; `clean_slop` baştaki
  `- `/`* `/`• ` madde öneklerini ve `**`/`__` işaretlerini siler (backtick'in kendisi de İÇİ de
  korunur: `` `__init__` `` bozulmaz). `1. `/`2) ` numara öneki yalnız cevapta **≥2 numaralı satır**
  varsa (gerçek liste) silinir — tek satırdaki "3. sınıftayım" Türkçe sıra sayısıdır. Aynı turda
  birebir tekrar eden satır ikinci kez gitmez. **Kısa satır elenmez**: "he", "yok", "la" doğal
  tepkidir.
- Geçmişe ve kanal notuna giren biçim `Reply::protocol_text()`: satırlar `\n` ile, varsa sonunda
  `tepki: 💀`. Model bir sonraki turda kendi biçimini görsün diye böyle.

## reply (bir sohbet turu, stream)
```
kilit ── meşgul? çık ── sohbet var? ── talimat seç ── meşgul=1 ── kilit bırak
bekle 0,15-0,35 sn ── güncel geçmiş + son mesaj + bekleyenler ── ruh hali (4 turda bir) ── research(link/haber/araştır) ── hedef seçimi (2+ yazan varsa) ── soru tavanı (too_many_questions) ── yazıyor…
generate_stream(stream, bütçe: reply_budget!) ── (hata: meşgul=0, çık)
send_stream: mesaj ilk ANLAMLI içerikle açılır (yerleşim boş kaldığı sürece "ilk" harcanmaz; stream_slice kısa yarım satırı bekletir) ── STREAM_EDIT_INTERVAL (1,2 sn) aralıkla düzenlenir ── düşünürken (cevap başlamadı): göster="Düşünüyorum...", gizle=canlı kelime sayacı, sessiz/kapalı=hiçbir şey (mesaj cevap başlayana dek hiç açılmaz) ── cevap başlayınca aynı mesaj düzenlenerek stream ── göster: thinking newline'sız tek satır, hem spoiler hem kod bloğu ── gizle: thinking mesajda yok, cevap sonunda "Düşünce Sürecini Göster" butonu (interaction_create tıklayana ephemeral kod bloğu açar, düşünce deposu 50 mesaj) ── sessiz: reasoning isteniyor (arka planda çalışıyor) ama hiç toplanmıyor/gösterilmiyor, buton da yok ── kapalı: istek reasoning'siz ── discord yanıtı yalnız ilk mesajda
akış SÜRERKEN görünen kısım: tamamlanmış satırlar (ardında \n olan) + son yarım satır ancak HALF_LINE_THRESHOLD (12) karakteri geçtiyse (stream_slice) ── böylece "tep" yarım hâlde mesaj olup silinmez
akış BİTİNCE parse_reply:
  silent ∧ satır yok ∧ TEPKİ DE YOK → açılan geçici mesajlar silinir, StreamResult::Silent (geçmişe hiçbir şey girmez, sayac artmaz, last_activity tazelenmez, yedek generate ÇAĞRILMAZ; hacked yine de azalır) ── "-" ile "tepki: 💀" birlikte gelirse susma değil, emoji düşer
  hiçbir şey yok → StreamResult::Empty → stream'siz yedek generate + satır bazlı tekrar elemesi + send_reply
  is_repeat SATIR BAZLI: son 5 bot satırıyla aynı olanlar düşer; hiç satır kalmaz ve tepki de yoksa bir kez yeniden üret, yine tekrarsa (ya da yeni cevapta ne satır ne tepki varsa) açılanları sil ve Empty
  final yerleşim write_stream ile yazılır (fazla mesajlar silinir) ── tepki varsa context.reaction_target mesajına create_reaction (hata warn log, akış durmaz; yalnız tepki de geçerli bir cevaptır)
üst üste 2+ farklı kişi yazdıysa TARGET_PICK mini çağrısı hedef kişiyi seçer; yanıt o kişinin mesajına bağlanır, talimata "ona seslen" notu girer
üretim sırasında yeni mesaj gelse de akış tamamlanır (sil-baştan yok); yeni mesaj sıradaki turda ele alınır
kilit ── meşgul=0 ── her görünen satır ayrı ayrı own_messages'a, hepsi TEK dosya yazımıyla channel_notes'a (tepki "bot: tepki: 💀" satırı olarak) ── asistan satırı = protocol_text ── counter++ ── hacked-- ── kilit bırak
… yeni mesaj yoksa çık, varsa bir tur daha
```
Talimat önceliği: hack devam > hack çıkış > boş. Üstüne eklenenler: ruh hali, internet bulgusu,
hedef kişi notu, soru tavanı.

## Soru tavanı
`too_many_questions(state, kanal)`: kanal geçmişindeki son 4 bot satırından (`tepki:` satırları sayılmaz)
≥2'si `?` ile bitiyorsa talimata "Bu sefer soru sorma; düz laf et ya da sus." eklenir. Kod ölçer,
uygulamayı model yapar — kesme/kırpma yok. `reply` ve CLI sohbet modu ikisi de uygular.

## send_lines (stream OLMAYAN yollar)
`strip_name` + `parse_reply` → `send_reply` (gövde; elinde çözülmüş `Reply` olan yollar doğrudan onu
çağırır) → satırlar sırayla ayrı mesaj. Aralarına `300 ms + 15 ms × karakter`
(tavan 1500 ms) gecikme + `broadcast_typing` girer: stream'in kendi temposu burada yok, üç mesaj
aynı anda düşmesin. Discord yanıtı yalnız ilk satıra takılır; ping de öyle ama **protokol
çözüldükten sonra**, gönderim anında ilk satırın başına `<@id> ` diye eklenir — metne baştan
yapıştırılırsa `-` ve `tepki:` satırları tanınmıyordu. Tepki hedefi verildiyse emoji atılır ve
kanal notuna protokol biçimiyle yazılır; **hedef yoksa tepki düşürülür** (kanalda görünmeyecek
tepki "gönderildi" sayılmasın). `silent` ya da gidecek hiçbir şey kalmayan cevapta **hiçbir şey gitmez**,
`None` döner — açılış göndericileri (poke, sorun, haber tanıtımı, hoş geldin, uyandım, uyanış
cevabı, yolda, gidiyorum, isim duyurusu) o turu atlar, sohbet açılmaz. Döndürdüğü `protocol_text`
sohbeti tohumlayan açılış metni olur. `run_prank` görsel + metni tek mesajda yolladığı için
protokolden yalnız ilk satırı alır.

## Sohbet yaşam döngüsü
- Başlangıç kaynakları: rastgele araya girme, etiket, hoş geldin, haber paylaşımı, poke, şaka, uyanınca dönüş, yoldan mesaj, gidiyorum duyurusu. Açılışlı olanlar `counter=1` ile başlar.
- Mesaj sınırı ve veda yok: sohbet son mesajdan 30 dk sonra sessizce kapanır (dakika tikinde `close_timed_out`), kanal yasağı yok. Kapanan sohbetin dökümü günlükçüye ve eleştirmene gider.
- Yasak yalnız *araya girmeyi* engeller; etiket her zaman cevap alır.
- Model çağrısı hata verirse sayaç ilerlemez, sohbet açık kalır.

## generate (her kişilikli çağrı)
1. Geçmişteki `user` satırlarından `isim` (": " öncesi) ve metin ayrıştırılır.
2. `memory::keywords(metinler)` → ≤40 kelime.
3. Kilit: `memory::retrieve(katilimcilar, name_to_id, anahtar, ham hafıza, 20)` → bütçeli bağlam; `system_text`.
4. `ask` → `clean` (ad öneki, tırnak, 1900).
Sohbet cevapları bunu kullanmaz; `generate_stream` aynı sistemi kurup stream açar (`send_stream` yazar), kırpma yoktur.

## CLI sohbet (`cargo run -- chat`)
Discord'a hiç bağlanmadan protokolü denemek için terminal tezgâhı (`src/chat_cli.rs`).
```
main: ilk argüman "chat" mi → Bot::setup() (DISCORD_TOKEN İSTEMEZ, yalnız model anahtarı)
  anahtar yoksa → "chat mode failed to start: <sebep>" + çıkış kodu 1
bot_name boşsa (ready hiç gelmiyor) growth.name, o da yoksa "bot"
start_chat(ChannelId::new(1)) — gerçek durum/ dosyalarından tohumlanır, kişilik gerçekçi
döngü: stdin satırı "isim: metin" (iki nokta yoksa ya da bir yanı boşsa yazan "misafir") · !quit ya da EOF → çık
  remember + kanal geçmişi (yalnız BELLEKTE, append_history) + sohbet geçmişine user satırı
  soru tavanı talimatı ── generate (stream YOK) ── strip_name ── parse_reply
  çıktı: her satır "bot_name: satır" · tepki "[reaction 💀]" · sus "(silent)" · hiçbir şey yoksa "(empty)" · model hatası "(error: …)" ve döngü sürer
  geçmişe protocol_text iter, counter++
```
Durum içeriğine hiçbir şey yazılmaz: `channel_note` yerine bellek içi `append_history` kullanılır, ajanlar ve
döngüler bu kipte hiç çalışmaz. (Tek istisna: `Bot::setup()` canlı yolla ortak olduğu için boş
`durum/{kisiler,konular,olaylar,arsiv,kanallar}` ve `resimler/` klasörlerini oluşturur.) **Doğrulanmadı:** gerçek model anahtarı bu makinede yok, canlı
cevap alışverişi görülmedi (bkz. AGENTS.md "Bilinen açıklar").

## Slash komutlar (komut yöneticisi → embed → detay modalı)
`ready` → her sunucuya kayıt (`modal::register_commands`, idempotent): liste `command::definitions()`
tablosundan çıkar (ad/açıklama/seçenekler tek kaynak, elle iki yerde tutulmaz) → kullanıcı slash
çalıştırır → `interaction_create(Command)` → tabloda `cmd.data.name` eşleşmesi bulunur, ilgili
çalıştırıcı çağrılır. Her komut **embed** döner, düz metin yok:
- Yerel/hızlı komutlar (`durum, yardim, ayarlar, zihin` varsayılan görünüm, `sifirla, dusunme,
  model` sorgu, `debug`) doğrudan `CreateInteractionResponse::Message` ile yanıtlar
  (`send_response`/`reply_info`, embed `modal::info_embed`).
- Ağ/model çağrısı yapan komutlar (`haber, sorun, gez, saka, hack, ajanlar, uyan, uyu, zihin
  test:true, model id değişimi`) Discord'un 3 sn'lik ilk yanıt sınırını aşabileceği için önce
  `defer` (`Defer`) ile anında onay verir, iş bitince `report_result` (`edit_response`) ile kısa
  bir sonuç embed'i yazar — asıl içerik (haber/şaka/vb.) zaten kendi `Bot::send` çağrısıyla
  kanala gidiyordu, buradaki yalnız bir "tamam" notu.

`/zihin` kartı: üç sütun (Kişiler/Konular/Olaylar) + üstte kişi select menüsü, altta Konular/Olaylar/Bot özeti butonları.
Menüden kişi seç ya da butona bas → `interaction_create(Component)` → ilgili **detay modalı**
(`person_modal` / `topics_modal` / `events_modal` / `summary_modal`); her bölüm kendi etiketli alanında, tek kutuya boca yok.
Kullanıcı modal'ı gönderirse → `interaction_create(Modal)` → kısa ephemeral onay; girdi toplanmaz.
`/zihin test:true` eski panel-teşhis yolunun yerini alır (aşağıda "Zihin zinciri" bölümü).

## Sunucuya bağlanınca
`guild_create` (sunucu başına bir kez) → arka planda: 14 gün geriye tarama (izinli kanallar, 100'lük sayfalar) → ham hafıza son 2000 → profiler → coach (huy boşsa). Yeniden bağlanmada tekrar taranmaz.
`guild_create` ayrıca süreç başına bir kez (`Handler.announced`) varsayılan kanala tek satır sürüm duyurusu atar: `geldim · v0.2.0 (69e2851, 2026-09-02) · model … · düşünme …` — hafızaya/kanal notuna yazılmaz (bot bunu kendi lafı sanmasın). `ready`'de değil, çünkü sunucu önbelleği orada henüz dolu değil.

## 6 saatlik tur (news_cycle)
uyanık değil → geç · seyahatte → profiler, coach, geç · profiler → diarist("gözlem", son 300) → coach → kanalda sohbet açıksa geç → news_agent (HN 12 + Sözcü 12, atılmamışlar) → seçim → tanıtım (`generate`) → gönder → sohbet aç, 2 saat yorum bekle, haberi "atıldı" say.

## Poke (saatte bir)
%25 (`PROBLEM_SHARE`): varsayılan kanala `post_problem` (uydurma kod derdi + soru), sohbet açılır. Aksi halde aşağıdaki akış.
uyanık değil → geç · seyahatte: bugün yazdıysa geç, %25 → `ON_THE_WAY` · yarın seyahat: bir kez `LEAVING` · değilse %30 → `OUT_OF_THE_BLUE` · `idle_channel` yoksa geç → `generate(son 40 satır)` → gönder → sohbet aç.

## Şaka (3 saatte bir, %10)
uyanık ∧ seyahatte değil ∧ boş kanal ∧ `resimler/` dolu → %30 hack (`HACK_ENTER` metni + görsel, sohbet `hacked=3`: 2 tur `HACK_CONTINUE`, 1 tur `HACK_EXIT`) · %70 düz görsel (`image_commenter`: model görseli görür).

## Gündem gezintisi (10 dk sonra, sonra 4 saatte bir)
rss 20 → seçim (`WANDERER_PICK`, huy+profil) → ≤3 sayfa (`read_page`: firecrawl ya da düz) → `generate(WANDERER_NOTE)` botun kendi günlüğü → `gundem.md` (12 giriş, eskisi arşiv) → `State.agenda` son 3 → her cevabın "GÜNDEM" bölümü ve coach girdisi.

## Uyku (dakikada bir)
`/uyan`: aktif planın bitişine kadar `forced_awake_until` (planı silmek işe yaramaz, dakika sonra yeniden kurulup uyutur). `/uyu [saat]`: geçici plan, zorlama sıfırlanır.
`update`: dün+bugün için plan yoksa kur (gergin ise %20, değilse %7 uykusuz gece). Uyanık→uyudu / uyudu→uyandı geçişi loglanır. Uyku hali konuşma promptuna karakter bahanesi olarak girmez.
**Uyurken dinleme sürer:** mesajlar ham hafızaya girer; `memory_cycle` 2 saatte bir gece gözlemi yapıp zihne işler; haber turu uyurken haber seçer ama atmaz, `stashed_news`'e koyar.
**Uyanınca:** bekleyen etiket varsa `WOKE_UP` ile kesin dönüş (hata durumunda liste geri konur, kaybolmaz). Etiket yoksa `uyanis.md` ajanı gece mesajlarını değerlendirir (`{"ilgi":0-10,"konu"}`); ilgi ≥5 ise `uyanis-cevap.md` ile son konuşulan kanala sabah sözü. Stok haber uyanık ilk turda "sabah haberi" olarak atılır.

## Seyahat (takvimden)
`travel::now()` bugünü tabloya bakarak bulur. Etkisi: "ŞU AN" satırı, araya girme ×0.3, haber/şaka yok, poke yerine günde ≤1 yoldan mesaj, bir gün önce `LEAVING`. Durum tutulmaz; yalnız `last_road_message` ve `announced_trip` işaretleri.

## Gelişim
Her biten sohbet `growth.chats++`, her mesaj `growth.messages++`. `check_growth`: gün ve sohbet
eşiklerine göre evre yalnız ileri atlar (yeni → isinma → yerlesik → eski-toprak). Evre: sistem
mesajında "GELİŞİM EVREN" bölümü, araya girme şansı × stage.confidence, poke × stage.poke. Yerleşik
evresine girince bir kez isim seçer: model tek kelime verir, takma ad her sunucuda değişir,
`bot_name` olur, gruba duyurulur. Sayaçlar `durum/gelisim.md`'de; yeniden başlatma sıfırlamaz.

## Biten sohbet → hafıza
Sohbet 30 dk sessiz kalınca `close_timed_out` kapatır; döküm `State.memory_queue`'ya düşer.
`memory_cycle` (10 dk'da bir, uykudan bağımsız) kuyruğu işler: `diarist` JSON → `olaylar/AA.md`
satırı (saniyeli), kişi dosyaları id bazlı (isim `name_to_id` ile çevrilir; puan, not, bilgiler,
etiket, olay), konu dosyaları, `kendim.md`, `INDEX.md` → `summarizer` sınır aşanları küçültür
(arşivle) → biten sohbette ayrıca `critic` → `duzeltmeler.md`. 6 saatlik turun gözlemi de
aynı kuyruktan geçer.

## Zihin zinciri (sohbet → günlükçü) ve teşhis
`close_timed_out` → info log `mind: chat closed [kanal] (30 min quiet) → queued (n), diarist within 10 min`
→ `memory_cycle` (10 dk) → `diarist` → info `mind: diarist [kaynak]: k person(s), m topic(s), o event(s) written`
ya da warn `mind: diarist failed [kaynak]: <sebep>`. `diarist` artık `Result<DiaristSummary, BotError>` döner.
`/zihin test:true`: kanalın son 30 satırını hemen günlükçüye verir, sonucu tek mesajla yazar (40 dk beklemeden).
Reasoning zorunlu modelde (glm-5.3-flash) `ask_raw`: 400 "mandatory" → alanlar kaldırılır + openrouter'da
`reasoning.effort=low` + bütçe max(2×, 1500); 200 ama content boş → JSON bekleyen kategorilerde
(gunlukcu, isteklilik, hedef_sec, ruh_hali, uyanis) düşünce alanındaki `{…}` içerik sayılır (warn log),
düzyazı çağrısında sayılmaz; yine boşsa bütçe büyütülüp bir kez daha denenir; hata mesajı kategori/model/
bütçe/düşünce uzunluğunu içerir.

## Debug modu (`/debug`, ayar paneli)
`State.debug` açıkken `debug_note` tek satır (⚙ …, ≤300 kr) DEBUG_CHANNEL'a, yoksa mesajın kanalına yazar
ve info loglar; hafızaya/kanal notuna girmez. İzler (İngilizce, geliştirici tanılaması): mesaj kararı
(`tag` / `dialog ongoing` / `willingness p/threshold · reason: … → reply|silent` / `2min limit` /
`fallback die`), reply turu (`mood`, `target`, `question cap`, `n line(s) sent · reaction X` /
`silent (-)` / `stream empty → fallback generate`), `sohbet kapandı (30 dk sessiz)`.

## Ayar paneli (`/ayarlar`)
Embed (sürüm, model, düşünme, debug, uyku, seyahat) + butonlar: düşünme göster/gizle/sessiz/kapat
(etkin olan Primary), debug aç/kapat, uyandır / uyut (8 saat). Buton → `interaction_create(Component)`
`setting_*` → `Handler::setting_button`: komutlarla aynı yollar (`ThinkingMode` + dusunme.md, `set_debug`,
`wake`/`put_to_sleep` + `sleep_transition`) → `UpdateMessage` ile panel yerinde yenilenir. Yanıt ephemeral
(yalnız çağırana görünür).
