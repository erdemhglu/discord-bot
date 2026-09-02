# Modüller ve fonksiyonlar

Her satır: imza · ne yapar · kim çağırır · kilit/await notu. Satır numaraları yaklaşıktır,
`grep -n "fn ad"` ile bul.

## src/main.rs

### Tipler
- `Mesaj { role: &'static str, content: String, resim: Option<String> }` — OpenRouter'a giden mesaj. `kullanici(..)`, `kullanici_resimli(metin, url)`, `asistan(..)` kurucular. `resim` `#[serde(skip)]`: istek gövdesi elle kurulduğu için serileştirmeye girmez, `mesaj_json` okur. Yalnız en son kullanıcı mesajında dolu kalır (`Handler::message` yeni satır eklerken eskilerinkini `None` yapar) — discord cdn linki ömürlü, eski görseli her turda yollamak token yakar.
- `mesaj_json(&Mesaj) -> Value` — mesajı openai uyumlu bloğa çevirir: resim yoksa `{role, content: "…"}`, varsa `content` = `[{type:text,text},{type:image_url,image_url:{url}}]` (ajanlar.rs `resimci` gövdesiyle aynı biçim). `sor_bolumlu` ve `sor_ham_akis` ikisi de bunu kullanır, yani görsel hem stream'li hem stream'siz yolda gider.
- `Cevap { satirlar: Vec<String>, tepki: Option<String>, sus: bool }` — model cevabının çözülmüş hâli (çıktı protokolü, bkz akislar.md). `Cevap::bos()` ne söz ne tepki ne susma kararı var mı; `Cevap::protokol_metni()` geçmişe/kanal notuna giren biçim (satırlar `\n` ile + varsa `tepki: 💀`), model bir sonraki turda kendi biçimini görsün diye.
- `Sohbet { gecmis: Vec<Mesaj>, sayac: u32, hackli: u32, son_mesaj, son_etiketlendi: bool, gelen: u32, son_gelenler, ruh_hali: String }` —
  bir kanaldaki açık sohbet. `sayac` botun yazdığı mesaj sayısı; `hackli` hack şakasında kalan cevap
  sayısı; `son_etiketlendi` reply-to kararı için (bkz `cevapla`); `ruh_hali` `ruh_hali_belirle`'nin
  son sonucu, "durum (yoğunluk)" biçiminde, boşsa nötr.
- `Durum` — tek paylaşılan durum (bkz mimari.md). `Durum::yukle()` diskten profil/huy/duzeltmeler/kendim/gundem/taranan okur, dizini yeniler.
- `Bot { durum: Mutex<Durum>, http: reqwest::Client, anahtar, haber_kanali, firecrawl, guild_id: Option<GuildId>, izinli_kanallar: Option<HashSet<ChannelId>> }`.
- `Bot::durum() -> MutexGuard<Durum>` — zehirli kilidi de açar. **Await üstünde tutma.**
- `Handler { bot: Arc<Bot>, baslatildi: AtomicBool }` — serenity `EventHandler`.
- `Hata = Box<dyn Error + Send + Sync>`.

### Yardımcılar
- `simdi_unix() -> i64` — şu an, unix saniye.
- `ad(&User) -> String` — görünen ad (`global_name`), yoksa kullanıcı adı. Hafıza ve kişi dosyaları bu adla.
- `kanal_not(&mut Durum, kanal, satir)` — kanal geçmişine (bellek 60 + `durum/kanallar/<id>.md`) ekler; kullanıcı satırları `message`'dan, bot satırları `gonder`'dan. `kanal_not_coklu(&mut Durum, kanal, satirlar)` aynı işi birden çok satır için TEK dosya yazımıyla yapar (`kanal_not` onun tek elemanlı hâli); `gonder_akis` çok satırlı cevabı bununla yazar, yoksa satır başına bütün geçmiş baştan yazılıyordu.
- `hatirla(&mut Durum, isim, metin)` — ham hafızaya "isim: metin" ekler, 2000'i aşarsa baştan atar.
- `son_mesajlar(&Durum, n) -> String` — ham hafızanın son n satırı, `\n` ile.
- `dokum(&[Mesaj], bot_adi) -> String` — sohbeti "isim: metin" satırlarına çevirir. Bot cevabı çok satırlı olabildiği için (protokol metni) **her** satırına `bot_adi:` öneki konur — tepki satırı dahil; yoksa eleştirmen/günlükçü/hoca alt satırları gruptaki insanlara sayar.
- `soy(String, bot_adi) -> String` — model çıktısı: baştaki `bot_adi:` kalıbı ve dış tırnak atılır.
- `temizle(String, bot_adi) -> String` — `soy` + 1900 karakterde keser. `uret`'in çıkışında, yani stream'siz yolda cevabın TAMAMINA uygulanır (protokol satırlara ayrılmadan önce): 4 satırlık bir cevabın toplamı 1900'ü aşarsa son satır(lar) sessizce kırpılır. Stream yolunda kırpma yok, her satır ayrı ayrı `bol` ile bölünür.
- `cevap_parcala(metin) -> Cevap` — **çıktı protokolünü çözen tek yer.** `soy` uygulanmış metinde çalışır (yeniden soymaz): `\n` ile böler, trim, boşları atar; `sus_isareti` satırı → `sus`; `tepki_govdesi` + `emoji_ayikla` → `tepki` (ilk kazanır, satır mesaj olarak gitmez); `'` ile başlayan kırıntı satır atılır; `slop_temizle` uygulanır; **gerçek liste** ise (≥2 satırda numara öneki) `numara_oneki` ile `1. `/`2) ` önekleri de silinir — tek satırdaki "3. sınıftayım" sıra sayısıdır, dokunulmaz; aynı turda birebir tekrar eden satır ikinci kez alınmaz; en çok `PATLAMA_SINIRI` (4) satır kalır (fazlası debug log ile düşer); her satır `bol(satir, MESAJ_SINIRI)` ile düzleştirilir. **Kısa satır elenmez** ("he", "yok", "la" doğal tepkidir). Çağıranlar: `gonder_akis`, `gonder_satirlar`, `akis_gorunum`, `saka_yap`, `sohbet_cli`.
- `tepki_govdesi(satir) -> Option<&str>` — satır `tepki:` ile mi başlıyor (büyük/küçük harf ve "tepki :" boşluğu tolere edilir); iki noktadan sonrası döner. `soru_fazla_mi` de bunu tepki satırlarını saymamak için kullanır.
- `emoji_ayikla(metin) -> Option<String>` — ilk emoji dizisi: `emoji_basi` (bilinen emoji blokları: U+2600–27BF, U+2B00–2BFF, U+1F000–1FAFF ve ©/®/™ gibi tekiller) ile başlar, `emoji_devami` (aynılar + VS15/VS16, ZWJ, keycap) ile en çok 8 char sürer. Tanım bilerek dar: "harf değilse emojidir" demek `—`, `…`, `→`, tipografik tırnak gibi işaretleri de emoji sayıyordu ve Discord isteği 400 ile dönüyordu. `:kekw:` gibi özel emoji biçiminde ve emoji hiç yoksa `None`.
- `sus_isareti(satir) -> bool` — satır tek başına `-`, `"-"`, `'-'`, `[sus]` ya da `(sus)` mu.
- `slop_temizle(satir) -> String` — "yapay zeka yazmış" izlerini siler: baştaki `- `/`* `/`• ` madde öneki, `**` ve `__` markdown işaretleri. Backtick hem kendisi hem İÇİ korunur (satır `` ` `` ile bölünür, tek indeksli parçalara dokunulmaz) — `` `__init__` `` bozulmasın. Numara öneki burada değil `cevap_parcala`'da (`numara_oneki`) elenir, çünkü "gerçek liste mi Türkçe sıra sayısı mı" ancak cevabın tamamına bakınca ayırt edilir.
- `numara_oneki(satir) -> Option<&str>` — `1. ` / `2) ` önekinden sonrası. Tek başına uygulanmaz; `cevap_parcala` yalnız cevapta ≥2 numaralı satır varsa çağırır.
- `soru_fazla_mi(&Durum, kanal) -> bool` — kanal geçmişindeki son 4 bot satırından (`tepki:` satırları sayılmaz) ≥2'si `?` ile bitiyor mu. `cevapla` ve `sohbet_cli` talimata "Bu sefer soru sorma; düz laf et ya da sus." ekler; kesme yok. Kod ölçer, uygulamayı model yapar.
- `bol(metin, sinir) -> Vec<String>` — metni en çok `sinir` karakterlik parçalara böler: önce cümle sınırı, sonra boşluk, o da yoksa sert keser; hiçbir şey atılmaz. Cevap 1900'ü aşınca ve uzun thinking'te kullanılır.
- `kesim_noktasi(metin, sinir) -> usize` — `bol`'ün kesim yeri; sınırın dörtte birinden önceki cümle/boşluk sayılmaz.
- `spoiler(metin) -> String` — `||...||`; içindeki `|` kaçırılır.
- `akis_gorunum(kip, dusunce, cevap, bitti) -> Vec<String>` — kipe göre ekran: düşünme sürerken (cevap boş, düşünce var) göster kipinde "Düşünüyorum...", gizle kipinde `dusunce_sayaci` (canlı kelime sayısı), sessiz/kapalı kipinde hiçbir şey; cevap başlayınca göster kipinde `tek_satir(dusunce)` hem spoiler hem `kod_bloklari` + cevap satırları, gizle/sessiz/kapalı kiplerde yalnız satırlar. Cevap artık `bol` ile değil `cevap_parcala(...).satirlar` ile mesajlara ayrılır: model yeni satıra geçince yeni mesaj açılır, önceki mesaj değişmez. `tek_satir(metin)` thinking'i tek akıcı satıra indirger (her düşüncede newline atılmaz).
- `akis_kesiti(cevap, bitti) -> &str` — akış sürerken cevabın gösterilebilir kısmı: tamamlanmış satırlar (ardında `\n` olan) + son yarım satır ancak `YARIM_SATIR_ESIGI` (12) karakteri geçtiyse. Gerekçe: "tep" yarım hâlde `tepki: 💀` ya da `-` olabilir, mesaj olarak açılıp bir sonraki edit'te silinmesin; kısa satır için boşuna edit atılmasın. `bitti=true` ise metnin tamamı.
- `akis_yerlesim(kip, dusunce, satirlar) -> Vec<String>` — düşünce blokları (yalnız göster kipinde) + satır mesajları. Satırlar dışarıdan gelir: `gonder_akis` final yerleşimde tekrar elemesinden geçmiş hâllerini verir.
- `dusunce_sayaci(dusunce)` — "Düşünüyorum... Şu ana kadar N kelime düşündüm." `kod_bloklari(metin)` — thinking'in kod blokları (1900'e bölünmüş). `dusunce_gosterim(metin)` — butonun ephemeral yanıtı: tek mesaja sığan kod bloğu, uzunsa kısaltma notu.
- `Durum::dusunce_bagla(mesaj, dusunce)` — gizle kipinde butonun bulması için düşünceyi son mesaj id'sine bağlar (`dusunce_deposu` 50 giriş, `dusunce_sirasi` ile eskiden düşer). `Handler::interaction_create` — `DUSUNCE_DUGMESI` tıklanınca depodan alır, yalnız tıklayana görünen ephemeral kod bloğu gönderir.
- `DusunmeKip { Goster, Gizle, Sessiz, Kapali }` (main.rs) — düşünme kipi; `arg_ile` komut argümanı çözer, `oku/dosya_degeri` `durum/dusunme.md`, `ad` ekran adı. Sessiz kipte reasoning normal istenir (kapatılmaz), yalnız `gonder_akis` düşünceyi hiç toplamaz/göstermez (placeholder/sayaç/buton yok) — göster/gizle'nin ekranda gösterdiği şeyi arka planda bırakır. Kapalı kip `Bot::reasoning_kapat` ile isteğe `reasoning.enabled=false` + `enable_thinking=false` ekler (yalnız stream/sohbet yolunda kipe bakılır; `sor_ham`'ın stream olmayan yolu — arka plan ajanları — kipten bağımsız her zaman kapatır, çünkü o yol reasoning'i zaten hiç okumaz).
- `cevap_butcesi!()` — makro; sohbet cevabı token bütçesi derleme durumuna göre: release `Some(CEVAP_TAVANI=4096)` (sıradan cevap altında kalır, yalnız tekrar/döngü gibi kaçak durumları keser), debug `Some(2000)` (maliyet koruması).
- `json_ayikla(&str) -> &str` — ilk `{` ile son `}` arası (kod bloğu süsünü atar).

### OpenRouter (impl Bot)
- `sor_ham(Value, kategori) -> Result<String>` — POST `/chat/completions`, `choices[0].message.content`; boşsa hata. Tek HTTP noktası. Zaman aşımı: bağlantı 15 sn, iki veri arası 120 sn, toplam sınır yok (uzun düşünme kesilmez). Ağ hatası / 429 / 5xx'te (`durum_denenebilir`) 2+4 sn geri çekilip `AI_YENIDEN_DENEME` kez yeniden dener; reasoning zorunlu bir modelde 400 dönerse (`reasoning_zorunlu_hatasi`) alanları kaldırıp yeniden dener — bu durumda `butce_tabanini_uygula` ile `max_tokens` (varsa) `REASONING_ZORUNLU_TABAN`'a (500) çıkarılır, yoksa küçük bütçeli mini-çağrılarda (20-80 token) reasoning bütçenin tamamını yiyip `content: null` bırakır. Aynı sebeple 200 dönüp içerik boş gelirse de (reasoning bütçeyi yemiş olabilir) hata hemen dönmez: bütçe tabana çıkarılıp bir kez daha denenir, taban da yetmezse `AI_YENIDEN_DENEME` sonunda pes edilir. Başarılı yanıtın `usage`'ı `kategori` ile `metrik_ekle`'ye gider (`!durum` kırılımı). `sor_ham_akis` de aynı mantıkta (yalnız akış açılmadan önce; boş-içerik-sonrası yeniden deneme yalnız `sor_ham`'da var, stream tarafı `gonder_akis`'te ayrıca ele alınır).
- `reasoning_kapat(govde, herhalukarda) -> bool` — kip Kapali ise ya da `herhalukarda=true` ise sağlayıcıya göre düğme: openrouter `reasoning.enabled`, mistral'e bir şey gitmez, diğerleri (qwen tarzı router) `enable_thinking:false`. `sor_ham` (stream olmayan) her zaman `true` geçer — o yol `reasoning_content`'i zaten okumaz, kullanıcı kipi ne olursa olsun kapatır; `sor_ham_akis` `false` geçer, yalnız kip Kapali ise kapatır. Alanları gerçekten eklediyse `true` döner (mandatory-reasoning yeniden denemesi için).
- `MesgulGuard` — `cevapla` kanalın meşgul bayrağını RAII ile bırakır: normal/erken dönüş ve panikte Drop çalışır; yeni tur için `drop(_mesgul)` + üstte yeniden insert.
- `soy(metin, bot_adi)` — ad öneki + tırnak soyma; char güvenli (bayt dilimi yok), `kucult` İ→i̇ birleşik noktasını atarak karşılaştırır.
- `sor(sistem, gecmis, max_tokens, kategori)` — `system` + geçmiş → `sor_bolumlu` (bütçe `Some`).
- `uret(gecmis, talimat, butce: Option<u32>, kategori)` — **kişilikle konuşan tek yol.** `sohbet_sistemi` ile sistem mesajını kurar → `sor_bolumlu` → `temizle`. Bütçe `None` ise max_tokens gitmez (yalnız kimi tekil çağrılarda; sohbet cevabı `cevap_butcesi!()` ile hep `Some`). Çağıranlar: stream yedeği/tekrar denemesi, dürtme, şaka, haber tanıtma, hoş geldin, uyandım, gezgin notu, resimci yedeği, isim.
- `sohbet_sistemi(gecmis, talimat) -> (sabit, degisken, bot_adi)` — geçmişteki `user` mesajlarından katılımcı adlarını (`"isim: "` öneki) ve metinleri çıkarır → `hafiza::anahtarlar` → kilit altında `hafiza::getir` + `sistem_metni`. `uret` ve `uret_akis` ortak kullanır.
- `sor_ham_akis(sabit, degisken, gecmis, butce, kategori) -> Result<AkisOkuyucu>` — `stream:true` POST; hata kontrolü `sor_ham` ile aynı. `Parca{metin,dusunce}` döndüren `AkisOkuyucu::sonraki()` SSE satırlarını çözer (`sse_ayikla`; reasoning `reasoning` ya da `reasoning_content` alanından), utf-8 chunk ortasında bölünse de tamponda bekletir.
- `bellek_dongusu(bot)` — 10 dakikada bir, uyku kontrolüne takılmaz: uykudaysa 2 saatte bir gece gözlemini kuyruğa koyar; sonra `bellek_kuyruk`'u işler (`gunlukcu`, biten sohbette + `elestirmen`).
- `uret_akis(gecmis, talimat, butce, kategori) -> Result<(AkisOkuyucu, bot_adi)>` — sohbet cevabını akış olarak açar. Çağıran: yalnız `cevapla`.
- `gonder_akis(ctx, kanal, okuyucu, AkisBaglam) -> Result<AkisSonuc>` — parçaları biriktirir (kapalı kipte reasoning biriktirilmez), `AKIS_DUZENLEME` aralıkla `akis_gorunum(..., bitti=false)` + `yaz_akis`; bitince `cevap_parcala(soy(...))`:
  **sus** (satır yok VE tepki yok) → açılan geçici mesajlar `sil_mesajlar` ile silinir, `AkisSonuc::Sus`; (`-` ile `tepki: 💀` birlikte gelirse susma değil: emoji yine düşer)
  **boş** (ne satır ne tepki ne sus) → aynı temizlik, `AkisSonuc::Bos`;
  **tekrar** artık satır bazlı: son 5 bot satırıyla aynı olan satırlar düşer, hiç satır kalmaz ve tepki de yoksa bir kez `uret` ile yeniden üretim, o da tekrarsa sil + Bos;
  final `akis_yerlesim` + `yaz_akis`; tepki varsa `baglam.tepki_hedefi` mesajına `ctx.http.create_reaction(..., ReactionType::Unicode(emoji))` (hata yalnız warn log, akış durmaz — yalnız tepki de geçerli bir cevaptır); kayıt: her görünen satır ayrı ayrı `kendi_mesajlarim` + `kanal_not`, tepki `"{bot}: tepki: 💀"` satırı olarak (tohum tutarlılığı), thinking hiç girmez. Döndürdüğü `Gonderildi(String)` içeriği `Cevap::protokol_metni()`.
  `AkisSonuc::{Gonderildi(String), Bos, Sus}`; `AkisBaglam{bot_adi, yanit, tepki_hedefi, gecmis, talimat, butce}` argüman yığını yerine tek yapı. `tepki_hedefi` `yanit`'tan ayrı bir alandır çünkü `yanit` koşulludur (yalnız etiket/kalabalık durumunda dolu), tepkinin ise her zaman düşeceği bir mesaj gerekir. `tepki_hedefi` **her zaman** sohbetin `son_mesaj`'ıdır; `yanit` ise `hedef_sec` bir kişi seçtiğinde o kişinin mesajına kayar — yani ikisi ayrışabilir: cevap erdem'e reply olarak bağlanırken emoji son yazana düşebilir. Bilinçli: hedef seçimi cevabın muhatabını değiştirir, tepki hâlâ "az önceki mesaja" düşer.
- `gonder_satirlar(ctx, kanal, ham, yanit, tepki_hedefi, ping) -> Option<String>` — **stream OLMAYAN yolların ortak göndericisi.** `soy` + `cevap_parcala` yapıp `gonder_cevap`'a devreder. `gonder_cevap(ctx, kanal, Cevap, yanit, tepki_hedefi, ping)` gövdedir: elinde zaten çözülmüş/tekrar elenmiş `Cevap` olan yollar (cevapla'nın yedek dalı) metne geri dönmeden bunu çağırır. Satırlar sırayla ayrı mesaj (`gonder`); aralarına `SATIR_GECIKME_TABAN + SATIR_GECIKME_HARF × karakter` (tavan `SATIR_GECIKME_TAVAN`) bekleme ve `broadcast_typing` girer — stream'in kendi temposu burada yok, satırlar aynı anda düşmesin. `yanit` yalnız ilk satıra takılır; `ping` de öyle ama **protokol çözüldükten sonra**, gönderim anında ilk satırın başına `<@id> ` olarak eklenir (metne baştan yapıştırılınca "`<@id> -`" susma işareti, "`<@id> tepki: 💀`" de tepki satırı sayılmıyordu). `tepki_hedefi` yoksa tepki düşürülür (kanalda görünmeyecek bir tepki "gönderildi" sayılmasın). `sus` ya da gidecek hiçbir şey kalmayan cevapta hiçbir şey gitmez, `None` döner. Döndürdüğü `protokol_metni` çağıranda sohbet açılış metni olur. Çağıranlar: `cevapla`'nın yedek `uret` dalı, `sorun_at`, `haber_gonder` (tanıtım), `durtme_dongusu` (DURUP_DURURKEN/YOLDA/GIDIYORUM), `guild_member_addition` (hoş geldin, ping'li), `uyku_gecisi` (UYANDIM), `uyanis_degerlendir`, `isim_sec` (duyuru). `None` dönerse o açılış atlanır ve sohbet açılmaz (debug log).
- `yaz_akis(ctx, kanal, &mut Vec<Message>, yerlesim, yanit)` — serbest fonksiyon. Yerleşimi açık mesajlarla uzlaştırır: değişeni `EditMessage` ile düzenler, eksiği açar (yalnız ilk mesaj yanıt/mention taşır), fazlasını siler; typing burada atılmaz (edit döngüsünde tekrarlanırdı, discord hız sınırına takılıyordu — `cevapla` model çağrısından önce bir kez atar). `sil_mesajlar(ctx, Vec<Message>)` açılanları geri alır.
- `analiz(metin, talimat, max_tokens, kategori)` — **kişiliksiz tek yol.** Sistem = `ANALIST`; kullanıcı mesajı = `metin + "---" + talimat`. Çağıranlar: profilci, gunlukcu, hoca, elestirmen, ozetleyici, haberci seçim, gezgin seçim, uyanis değerlendirme.
- `isteklilik() -> Option<u8>` — "bu konuşmaya katılayım mı?" mini değerlendirmesi: profil+dizin sabit blokta (cache_control), son 12 mesaj değişken → `sor_bolumlu(..., 80, "isteklilik")` → `isteklilik_puan` JSON'dan 0-10. Çağıran: `Handler::message` (kanal başına en sık 2 dk, `son_degerlendirme`, yalnız farklı biri yazdıysa ya da sohbet yoksa). Hata/bozukta `None` → yedek zar.
- `hedef_sec(bekleyenler) -> Option<String>` — 2+ farklı kişi yazınca kime dönüleceğini seçer: son 12 mesaj + bekleyen isimler → sabit blok HEDEF_SEC{ad}, değişken bekleyenler → `sor_bolumlu(..., 40)` → `hedef_ayikla` (JSON ya da düz metin, bilinen adlarla eşleştirilir). Çağıran: `cevapla`; seçilen kişinin mesajı `yanit` olur, talimata not girer.
- `ruh_hali_belirle(gecmis) -> Option<String>` — bu sohbetin ruh halini belirler: ANALIST sabit, RUH_HALI{ad} değişken, sohbetin kendi geçmişi mesaj listesi olarak gider (görseller bu kopyada `None`'lanır: 40 token'lık analize resim yükü yollanmaz, vision'suz route hataya düşmesin) → `sor_bolumlu(..., 40, "ruh_hali")` → `ruh_hali_ayikla` (yoğunluk <3 ise None, nötr sayılır). Çağıran: `cevapla`, yalnız sohbet açılırken (`sayac==0`) ve her 4 turda bir; sonuç `Sohbet.ruh_hali`'ye yazılır ve talimata "ŞU ANKİ RUH HALİN" satırı olarak eklenir.
- `gonder(ctx, kanal, metin, ping, dosya, yanit: Option<MessageId>)` — `yanit` verilirse discord yanıtı (`reference_message`) olur ve yanıtlanan kişi pinglenir (`replied_user`).  mention'lar kapalı (`CreateAllowedMentions::new()`, yalnız `ping` açılır), isteğe bağlı ek dosya; başarılıysa `kendi_mesajlarim`'a (50) ekler. Kilit gönderimden SONRA alınır.
- `Bot::sor_bolumlu(sabit, degisken, gecmis, butce: Option<u32>)` — sistem mesajını `sistem_json` ile iki metin bloğu olarak gönderir, ilki `cache_control: ephemeral`; bütçe `None` ise max_tokens yok.
- `sistem_json(sabit, degisken) -> Value` — değişken boşsa düz system, değilse iki blok. Serbest fonksiyon.
- `Bot::tekrar_mi(kanal, cevap)` — kanal geçmişindeki son 5 bot satırıyla aynı mı. `Bot::arastir(metin) -> Option<String>` — link/haber/araştır tetiklerine göre sayfa, RSS ya da Firecrawl arama sonucu.
- `sistem_metni(&Durum, talimat, getirilen) -> (String, String)` — (sabit, değişken); bölümleri sırayla ekler (mimari.md listesi). Serbest fonksiyon, kilit çağıranda.

### Sohbet motoru
- `Bot::sorun_at(ctx, kanal)` — `uret(SORUN, 160)` ile uydurma kod derdi, gönder, sohbet aç. Dürtme döngüsü (%25) ve `!sorun`.
- `sohbet_baslat(&mut Durum, kanal, acilis: Option<String>) -> &mut Sohbet` — kanal geçmişinin son 10 satırıyla tohumlar (bot satırları assistant). Açılış zaten gönderilip geçmişe SATIR SATIR düşmüş olur: tohumun sonundaki bot bloğu taranır ve açılışın satırlarıyla eşleşenler atılır (araya haber linki gibi başka bir bot mesajı girmiş olabilir), böylece açılış modele iki kez görünmez;  varsa mevcut sohbeti döner (`entry().or_insert`), yoksa yeni; açılış varsa `asistan` mesajı + `sayac=1`.
- `sohbet_bitir(&mut Durum, kanal) -> Option<Sohbet>` — haber bekleme silinir, sohbet çıkarılıp döner; kanal yasağı yok, kapatma `zaman_asimi_kapat`'tan gelir.
- `Bot::zaman_asimi_kapat(ctx)` — dakika tikinde: `SOHBET_ZAMAN_ASIMI` (30 dk) sessiz kalan sohbetleri meşgul değilse kapatır, dökümü `gunlukcu`+`elestirmen`'e verir, `gelisim.sohbet++`; ayrıca süresi dolan haber sohbetlerini temizler (yorum penceresi geçmiş + o pencerede aktivite yoksa sessizce kapanır, `haber_bekleyen` haritası şişmez).
- `Bot::komut` → artık `src/komut.rs` içinde (aşağıda).
- `Bot::haber_at(ctx, kanal) -> bool` — haberci → link → tanıtım → gönder → sohbet + 2 saat yorum bekleme. `haber_dongusu` ve `!haber` çağırır.
- `Bot::saka_yap(ctx, kanal, hack)` — görsel seç, hack ise `HACK_GIRIS`, değilse `resimci`; metin `cevap_parcala`'dan geçer ve yalnız **ilk satır** alınır (görsel tek mesajda gider, satır patlaması burada anlamsız); model sustuysa şaka atlanır; gönder; sohbet (`hackli=3`). `saka_dongusu` ve `!saka`/`!hack` çağırır.
- `Bot::cevapla(ctx, kanal)` — döngü: (1) kilit: meşgulse çık; sohbet yoksa çık; talimat seç ve meşgul işaretle. (2) 0,15-0,35 sn mesaj biriktirme payı; güncel geçmiş, hedef mesaj, `gelen`; ruh hali (4 turda bir), `arastir` bulgusu ve hedef kişi notu göreve eklenir; `soru_fazla_mi` ise "bu sefer soru sorma" talimatı; `broadcast_typing`. (3) `uret_akis` (bütçe `cevap_butcesi!()`) ile stream açılır. (4) `gonder_akis` (`tepki_hedefi = son_mesaj`): her satır ayrı mesaj, thinking kipe göre, tekrar eden satırlar düşer. (5) `Sus` → geçmişe/sayaca/`son_aktivite`'ye **hiçbir şey yazılmaz**, yedek `uret` çağrılmaz; yeni mesaj varsa bir tur daha, yoksa çık. (6) `Bos` → yeni mesaj varsa bir tur daha, yoksa `uret` + `gonder_satirlar` ile stream'siz yedek (o da susarsa çık). (7) meşgul kaldır, asistan satırı (`protokol_metni`) ekle, sayaçları ilerlet, `son_aktivite` tazele. Yeni mesaj geldiyse başa dön. Kapatma yok; sessiz kalan sohbeti zaman aşımı kapatır.

### Hafıza (discord tarafı)
- `gecmisi_oku(bot, ctx, guild)` — botun üyeliğini çeker, izinli (`VIEW_CHANNEL|READ_MESSAGE_HISTORY`) metin kanallarını pozisyon sırasıyla gezer, `GetMessages` 100'lük sayfalarla 14 gün geriye okur, bot/boş mesajları atlar, `content_safe` ile mention'ları ada çevirir, zamana göre sıralar; favori id görürse `favori_adi` yazar, `ad_id` yalnız boşsa dolar (canlı eşleme öncelikli). Tarih hafızanın ÖNÜNE eklenir (tarama sürerken gelen canlı mesajlar arkada kalır, kronoloji bozulmaz, boca canlıları ezmez).
- `varsayilan_kanal(bot, ctx) -> Option<ChannelId>` — `HABER_KANALI` → sunucu sistem kanalı → en üst metin kanalı. Önbellekten, await yok.
- `bos_kanal(bot) -> Option<(ChannelId, String)>` — son konuşulan kanal; sohbet açık değil, yasaklı değil, profil var → (kanal, son 40 satır). Dürtme ve şaka bunu kullanır.

### Döngüler (`dongu_bekle`, `ready`'de bir kez)
`dongu_bekle(ad, kur)` — her döngüyü bekçiyle başlatır: panikte log + 5 sn sonra yeniden,
temiz dönüşte de yeniden (döngüler sonsuzdur). `KAPANIYOR` (AtomicBool) kapanış sinyali:
döngüler tik başında bakar ve döner, bekçi yeniden başlatmaz; `main`'in kapanış görevi kurar.
- `haber_dongusu(bot, ctx)` — 6 saatte bir: **uykudaysa** haber seçer ama atmaz, `stok_haber`'e koyar (bir kez); seyahatteyse profilci+hoca, geç; uyanıkken: profilci → gözlem kuyruğa → hoca → varsayılan kanalda sohbet açıksa geç → stok varsa `haber_gonder(stok)`, yoksa `haber_at`.
- `durtme_dongusu(bot, ctx)` — saatte bir: uyanık değilse geç; seyahatteyse günde bir kez %25 ile `YOLDA`; yarın seyahat başlıyorsa bir kez `GIDIYORUM`; değilse %30 ile `DURUP_DURURKEN`; `bos_kanal` → `uret(son 40 satır)` → gönder → sohbet başlat.
- `saka_dongusu(bot, ctx)` — 3 saatte bir: uyanık değilse/seyahatteyse geç; %10; `bos_kanal`; `rastgele_resim` yoksa geç; %30 hack: `uret(HACK_GIRIS)`, değilse `resimci(resim)`; görselle gönder; sohbet başlat, hack ise `hackli = 3`.
- `gezgin_dongusu(bot)` — ilk 10 dk sonra, sonra 4 saatte bir, uyanıksa `gezgin`.
- `Bot::uyku_gecisi(ctx)` — uyudu/uyandı geçişini loglar; uyurken `uyku_basi`+`uyku_basi_hafiza_len` işaretlenir. Uyanışta: bekleyen etiket varsa `UYANDIM` ile dönüş (hata durumunda liste geri konur); yoksa `uyanis_degerlendir` gece mesajlarını değerlendirir. Döngü ve `!uyan`/`!uyu` çağırır.
- `Bot::uyanis_degerlendir(ctx, gece)` — `analiz(UYANIS{ad}, 100)` → `{"ilgi","konu"}`; ilgi ≥5 ise `uret(UYANIS_CEVAP{ad,konu}, 250)` ile son konuşulan kanala sabah sözü + sohbet.
- `uyku_dongusu(bot, ctx)` — dakikada bir: `uyku::guncelle`, uyandı/uyudu geçişini loglar; uyanınca `bekleyen_etiketler` varsa son etiketin kanalına `uret(UYANDIM)` ile döner, sohbet başlatır.

### Discord olayları (Handler)
- `ready` — bot adını yazar; her gelişte sunuculara slash komutları kaydeder (`modal::komutlari_kayit`, idempotent); `baslatildi` ilk kez ise beş döngüyü başlatır.
- `interaction_create` — `Command` → adına göre ephemeral embed kart (`durum_mesaji`/`zihin_mesaji`/`yardim_mesaji`); `Modal` → kısa ephemeral onay (girdi toplanmaz); `Component` → düşünce butonu (`dusunce_dugmesi`) ya da zihin detay katmanı (`ZIHIN_KONULAR/OLAYLAR/OZET` butonları bölüm modalı, `ZIHIN_KISI_SEC` menüsü kişi modalı açar).
- `guild_create` — `taranan`'a ilk kez giriyorsa arka planda `gecmisi_oku → profilci → hoca (huy boşsa)`.
- `guild_member_addition` — kanal: sunucu sistem kanalı → varsayılan; favori ise adını kaydet; sohbet açık/yasaklıysa çık; `uret(HOS_GELDIN)` → mention'lı gönder (ping açık) → sohbet başlat.
- `message` — bot/webhook/DM ise çık; `GUILD_ID`/`KANALLAR` dışıysa çık; `content_safe`; **ek görsel:** `attachments` içinde `content_type`'ı `image/` ile başlayan ilk ekin URL'i alınır, erken çıkış "metin de ek de boş" hâline gelir (sırf resim atılmış mesaj işlenir); metin `[resim] <metin>` ya da `[resim attı]` olarak işaretlenir ve bu işaretli metin hafızaya, kanal notuna ve sohbet satırına aynen gider — URL yalnız sohbet geçmişindeki `Mesaj.resim`'e konur ve yeni kullanıcı satırı eklenirken önceki girdilerin `resim`'i `None` yapılır (yalnız en son görsel modele gider). **1. faz (kilit):** etiketlendi mi (mention listesi, yanıtlanan mesaj botun mu, metinde bot adı geçiyor mu) → `hatirla`, `ad_id`/`kullanici_adlari`, `son_kanal`, favori adı; haber bekleme süresi dolduysa sohbeti kapat; **uyuyorsa**: etiketlendiyse `bekleyen_etiketler`'e (20) ekle, çık; `devam_eden_diyalog` — sohbet açık VE sohbetteki son user mesajının sahibi bu mesajı atanla aynı isimse (gerçekten kendisiyle konuşuyor) → doğrudan cevaplanır, isteklilik değerlendirmesi atlanır. Etiket de aynı şekilde doğrudan cevaplanır. İkisi de değilse (kanalda başka biri yazdı, ya da sohbet yok) isteklilik değerlendirmesi gerekir (kanal başına en sık 2 dk). **2. faz (kilitsiz):** gerekiyorsa `isteklilik()`; puan ≥ eşik (evre ±1, seyahat +2) ise katılır; çağrı yoksa yedek zar (`SANS`). **3. faz (kilit):** katılıyorsa `sohbet_baslat`, kullanıcı satırını geçmişe ekle (20'de tut), `kanal_not`. Kilit dışı: `cevapla`. Not: bu, bir kez açılan sohbette kanaldaki HERKESE otomatik cevap verme davranışını (eski tasarım) kaldırır — yalnız gerçek muhatabına.

### Başlangıç
- `ayar(isim)` — boş olmayan env değişkeni ya da açık hata.
- `kapanis_bekle()` — ctrl-c veya SIGTERM.
- `Bot::kur() -> Result<Arc<Bot>, Hata>` — sağlayıcı seçimi (`SAGLAYICI`/anahtarlar/`MODEL`/`API_ADRES`), `HABER_KANALI`/`GUILD_ID`/`KANALLAR`, `durum/{kisiler,konular,olaylar,arsiv,kanallar}` + `resimler/` klasörleri, `Durum::yukle` + `uyku::guncelle` + `durum/model.md`, reqwest istemcisi. **Discord'a bağlanmaz, DISCORD_TOKEN istemez**: hem `main`'in bot yolu hem `cargo run -- sohbet` buradan geçer (ikisi aynı kurulumu görsün diye tek fonksiyona çıkarıldı).
- `main` — `.env`, loglama, panic hook; ilk argüman `sohbet` ise `Bot::kur()` + `Bot::sohbet_cli()` (kurulum hatasında tek satır mesaj + çıkış kodu 1) ve döner. Değilse `DISCORD_TOKEN` + `Bot::kur()`, intents `GUILDS|GUILD_MESSAGES|GUILD_MEMBERS|MESSAGE_CONTENT`, kapanışta `shard_manager.shutdown_all`.

## src/komut.rs (impl Bot)
Test ve yönetim komutları; `Handler::message` metin `!`/`/` ile başınca `Bot::komut(ctx, msg, komut, arg)`'a düşer, tanınan komut true döner ve mesaj sohbete girmez.
- `komut` dalları: sifirla · haber · sorun · gez · saka/hack · ajanlar · uyan · uyu · durum · zihin · düşünme · model · yardım/help.
- `zihin`: `modal::zihin_embedleri` kartını kanala gönderir + "detay için `/zihin`" (kanal mesajında bileşen/modaal açılamaz, etkileşim gerekir).
- `durum`: `modal::durum_metni` ile ortak metin (`!durum` ve `/durum` aynı satırı gösterir).
- `düşünme` (`dusunme` da tanınır): argüman `DusunmeKip::arg_ile` ile çözülür (göster/aç, gizle, sessiz, kapat/kapalı); kipi `Durum.dusunme`'ye yazar, `durum/dusunme.md`'de kalıcılaştırır. Argümansız çağrı mevcut kipi söyler.
- `yardım`/`yardim`/`help`: `YARDIM` sabiti, tüm komutların kısa listesi.
- `model_var_mi(id)` — OpenRouter `/models` listesinde arar; liste çekilemezse engel olmaz.

## src/sohbet_cli.rs (impl Bot)
Terminal sohbet tezgâhı: discord'a bağlanmadan çıktı protokolünü (satır = mesaj, `tepki:`, `-`)
denemek için. `cargo run -- sohbet` ile açılır (`main`).
- `CLI_KANAL = 1` — sahte kanal id'si; gerçek bir discord kanalı değil, yalnız sohbet durumunun anahtarı.
- `gecmise_ekle(&mut Durum, kanal, satir)` — kanal geçmişine **yalnız bellekte** ekler (`KANAL_GECMIS` sınırıyla). `kanal_not` aynı işi yapıp diske de yazdığı için burada kullanılmaz: tezgâh gerçek `durum/kanallar/*.md` dosyalarını kirletmemeli. (`hatirla` zaten bellek içi, o çağrılır.)
- `satir_coz(satir) -> (isim, metin)` — `"isim: metin"`; iki nokta yoksa ya da bir yanı boşsa yazan `emin`, satırın tamamı metin.
- `Bot::sohbet_cli(&self)` — `bot_adi` boşsa (`ready` hiç gelmiyor) `gelisim.isim`, o da yoksa `"bot"`; `sohbet_baslat` gerçek `durum/` dosyalarından tohumlar (kişilik gerçekçi kalsın). stdin bloklayarak satır satır okunur (bu kipte arka plan döngüsü yok, ayrı okuyucuya değmez); `!cik` ya da EOF çıkar. Her tur: `hatirla` + bellek geçmişi + sohbet geçmişine `kullanici` satırı → `soru_fazla_mi` talimatı → `uret` (**stream yok**: burada akış temposu değil çıkan protokol ölçülüyor) → `soy` → `cevap_parcala` → her satır `"bot_adi: satır"`, tepki `[tepki 💀]`, susma `(sustu)`, hiçbir şey `(boş)`, model hatası `(hata: …)` (döngü sürer) → geçmişe `protokol_metni` iter, `sayac++`.
- Testler: `satir_cozulur`, `gecmis_bellekte_sinirli`.

## src/modal.rs
Komut arayüzü: slash komutlar ephemeral **embed kart** döndürür (web sayfası gibi bölümlü),
detaylar **etiketli modal alanlarına** dağıtılır — tek metin kutusuna her şey boca edilmez.
Discord sınırları: embed field value ≤1024, modal ≤5 bileşen × value ≤4000, başlık/etiket ≤45,
select menü ≤25 seçenek (etiket ≤45, açıklama ≤100).
- `durum_metni(d)` — evre/gün, sayaçlar, model, uyku, düşünme, seyahat, token metriği (+ kırılım); `!durum` kullanır. `token_kirilimi(m)` — kategoriler toplam tokene göre sıralı.
- `zihin_embedleri(d)` — tek kart: description evre/gün/model/kip; üç satır içi alan: Kişiler (ilk 8: ad+puaan+etiket), Konular (ilk 8: ad+son not), Olaylar (son 5, en yeni aydan kronolojik); footer bot adı + tarih. Boş alan "—".
- `zihin_bilesenleri()` — 1. satır: kişi select menüsü (`ZIHIN_KISI_SEC`, ≤25 kişi, değer=id, açıklama etiket+not); 2. satır: Konular/Olaylar/Bot özeti butonları.
- `zihin_mesaji(d)` / `durum_mesaji(d)` / `yardim_mesaji()` — ephemeral `CreateInteractionResponseMessage` (embed + bileşen).
- Detay modalları (`modal_kisi(id)` / `modal_konular()` / `modal_olaylar()` / `modal_ozet(d)`) — her konu kendi etiketli alanında: kişi = Kimlik/İzlenim/Etiketler/Bildikleri(son 8)/Son olaylar(son 5); konular = Son değişenler(15)+Diğer; olaylar = ay başına alan (son 3 ay, her ayın son 10 kaydı, başlık "Eylül 2026"); özet = Durum/Token/Kendim/Gündem. Boş bölümler atlanır, hepsi boşsa tek "(henüz boş)" alanı.
- `sigdir(metin, sinir)` — sınır aşımı son satır/boşluk hizasında kesilir + not. `ay_adi("2026-09")` → "Eylül 2026".
- `komutlari_kayit(http, guild)` — `/durum` `/yardim` `/zihin` sunucu komutları; her ready'de idempotent.
- `hafiza.rs` yardımcıları: `kisi_dokumleri` (mtime sırası `Kisi` listesi), `konu_dokumleri` (ad + son not), `olay_aylari(n)` (son n ayın "- " satırları, en yeni ay başta).

## src/ajanlar.rs (impl Bot)
- `profilci()` — son 600 satır → `analiz(PROFIL_CIKAR, 1200)` → `profil.md` + `Durum.profil`.
- `gunlukcu(dokum, kaynak, kanal)` — `analiz(GUNLUKCU{ad,kaynak,favori}, 1200)` → JSON `Kayit{olay, kisiler[{isim,puan_degisimi,not,bilgiler,etiketler}], konular[{ad,not}], kendim}`; JSON çözülemezse ham çıktı `arsiv/gunlukcu-<kaynak>.md`'ye kurtarılır (emek kaybolmaz). → olay satırı (`olay_ekle`, saniyeli), her kişi: isim `Durum.ad_id` ile id'ye çevrilir (çözülemeyen atlanır, loglanır), `kisiler/<id>.md` oku, ad değiştiyse eskisi `eski_adlar`'a, puan += clamp(-3..3) sonra clamp(-10..10), not/bilgiler/etiketler, favori ise +10 ve sabit not, `kisi_yaz`; konular `konu_ekle`; kendim → `kendim.md`; dizin yenile; sonra `ozetleyici`.
- `ozetleyici()` — `hafiza::sinir_asanlar()` için: kişi → `analiz(OZETLEYICI_KISI{sinir=1000}, 700)`, konu → `OZETLEYICI_KONU{800}`, olay → eski %60 satır `OZETLEYICI_OLAYLAR` ile 3-5 satıra, yeni %40 kalır. Sonuç boş değil ve eskisinden kısaysa: kişi/konu için eski dosya arşive, yeni yazılır; olayda taşınan satırlar arşive. Küçülmediyse dokunmaz. Dizin yenile.
- `haber_gonder(ctx, kanal, h) -> bool` — seçilmiş haberi paylaşır (tur haberi ya da uyku stoku); tanıtım `uret(HABER_TANIT)`, sohbet açılır, 2 saat yorum beklenir.
- `hoca()` — profil + dizin + gündem + kendim + mevcut huy + son 200 satır + botun son mesajları → `analiz(HOCA{ad}, 800)` → `huy.md`.
- `elestirmen(dokum)` — `analiz(ELESTIRMEN{ad,mevcut}, 400)` → `duzeltmeler.md`.
- `haberci() -> Result<Haber>` — HN ilk 12 (atılmamış) + Sözcü RSS ilk 12 (atılmamış, kimlik = link hash) → liste "n. [hn|gündem] başlık" → `analiz(HABER_SEC{profil}, 10)` → numara → `Haber{id,title,url,score,kaynak}`.
- `resimci(&PathBuf) -> Result<String>` — görseli base64 `image_url` olarak sistem=`sistem_metni(RESIM_AT)` ile `sor_ham`; hata olursa `uret` ile körlemesine. `temizle`.
- `rastgele_resim() -> Option<PathBuf>` — `resimler/` içinden png/jpg/jpeg/gif/webp.
- `Haber` — serde; `kaynak` `#[serde(skip)]`, `score` HN dışı 0.

## src/hafiza.rs
- Sabitler: `KISI_SINIRI 1800 / KISI_HEDEF 1000 / KONU_SINIRI 1500 / KONU_HEDEF 800 / OLAY_SINIRI 6000 / BAGLAM_BUTCESI 6000 / DIZIN_KISI 40 / FAVORI_NOTU`.
- `yol(parca)`, `oku(parca)`, `yaz(parca, icerik)` — `YAZMA_KILIDI` (static Mutex) ile tek sıradan ve atomik (geçici + rename); `ekle(parca, satir)` — gerçek append (OpenOptions), aynı kilit; `arsivle(parca, icerik)` (`arsiv/parca`'ya tarihli başlıkla ekler).
- `kisi_dokumleri` / `konu_dokumleri` / `olay_dokumu` — modal gösterimi için mtime sıralı dökümler.
- `slug(isim)` — küçük harf, Türkçe harf sadeleştirme, alfanümerik dışı `-`, boşsa "bilinmeyen".
- `tarih()`, `tarih_unix(unix)` (Hinnant civil-from-days), `ay()` "YYYY-AA".
- `Kisi { id, isim, kullanici_adi, eski_adlar, puan, etiket, not, bilgiler, olaylar }` — `coz(id, metin)` dosyadan, `metin()` dosyaya; dosya `kisiler/<id>.md`. Biçim: `# İsim` / `id:` / `kullanici_adi:` / `eski_adlar:` / `puan: +3` / `etiket: a, b` / `not: ...` / `## Bildiklerin` `- ...` / `## Son olaylar` `- tarih saat: ...`.
- `kisi_oku(isim)`, `kisi_yaz(&Kisi)` — `kisiler/<slug>.md`.
- `konu_ekle(ad, not)` — `konular/<slug>.md`, yoksa başlık+etiket satırı, sonra `- tarih: not`.
- `olay_ekle(kanal, olay)` — `olaylar/YYYY-AA.md`'ye `- tarih #kanal: olay`.
- `dosyalar(klasor)` — `.md`'ler, son değişen önce. `ilk_satir(p)`.
- `dizin_yenile() -> String` — `## Kişiler` (≤40: `- ad (+p) · etiketler · not`), `## Konular` (≤30: `- ad · son: tarih`), `## Olaylar` (≤3 ay: `- YYYY-AA · n kayıt`); `INDEX.md`'ye yazar.
- `DURAK` — elenen sık kelimeler. `anahtarlar(&[String])` — 4+ harf, durak değil, tekrarsız, ≤40.
- `puanla(metin, anahtar)` — kaç anahtar geçiyor. `kirp(metin, sinir)` — karakter sınırı + `…`.
- `getir(katilimcilar, ad_id, anahtar, hafiza, atla_son) -> String` — sırayla: katılımcıların kişi dosyaları (`ad_id` ile id'ye çevrilir, ≤4, her biri ≤1200), en çok eşleşen 2 konu dosyası (≤800), ayın son 8 olayı, ham hafızadan (son `atla_son` hariç) ≥2 anahtarla eşleşen en fazla 12 satır (puan sonra yenilik sırası, sonra kronolojik). Bütçe 6000 karakter; sığmayan bölüm ve sonrası atlanır.
- `sinir_asanlar() -> Vec<(tür, yol)>` — boyutu aşan kişi/konu dosyaları ve bu ayın olay dosyası.

## src/gundem.rs
- Sabitler: `RSS_ADRESI`, `GUNDEM_KAYIT 12`, `SAYFA_SINIRI 3500`.
- `temiz_html(ham)` — CDATA, script/style blokları, etiketler atılır; temel entity'ler; boşluk toplanır.
- `etiket_ici(parca, etiket)` — `<etiket>`/`<etiket ` … `</etiket>` içi, temizlenmiş.
- `rss(http) -> Result<Vec<RssHaber{baslik,link,ozet}>>` — `<item` bölerek; başlık ve http link şart.
- `kimlik(link) -> u64` — DefaultHasher; atılan haber takibi için.
- `girisler(metin) -> Vec<String>` — `gundem.md`'yi `## ` başlıklı girişlere böler. `son_gundem(metin)` son 3 giriş.
- `Bot::sayfa_oku(url)` — firecrawl anahtarı varsa `POST api.firecrawl.dev/v1/scrape {url, formats:[markdown], onlyMainContent}` → `data.markdown`; yoksa `GET` + `temiz_html`. 3500 karakter.
- `Bot::firecrawl_ara(sorgu) -> Result<String>` — `POST /v1/search` limit 5; başlık, açıklama, adres satırları.
- `Bot::gezgin()` — rss ilk 20 → `analiz(GEZGIN_SEC{ad,huy,profil}, 20)` → ≤3 numara → her biri `sayfa_oku` (hata: özet) → `uret(GEZGIN_NOT, 350)` (kişilikle, kendi günlüğü) → `gundem.md`'ye `## tarih saat` girişi; 12'yi aşan en eski giriş arşive; `Durum.gundem` = son 3.

## src/uyku.rs
- Sabitler: `SAAT_FARKI +3h`, `UYKUSUZLUK_SANSI 0.07`, `UYKUSUZLUK_GERGIN 0.20`.
- `Plan { gun, uykusuz_bas: Option<i64>, bas, bit }` — bir gecenin planı (unix saniye).
- `yerel(unix) -> (gün no, gün içi saniye)`, `saat()` "SS:DD", `saat_metni()` "YYYY-AA-GG günadı SS:DD".
- `oynama()` ±45 dk. `gergin_mi(&Durum)` — `kendim`+`huy` içinde kırgın/sinir/gergin/takıntı/uyku/kafayı/bunalt geçiyor mu.
- `plan_kur(gun, gergin)` — normal: 01:00±45 → 09:00±45; uykusuz: 01:00 ayakta, 06:00±45 → 13:00±45.
- `guncelle(&mut Durum)` — dün ve bugün için plan yoksa kurar, biteni atar. `uyanik_mi`, `uykusuz_mu`, `durum_metni` ("ŞU AN" satırı).

## src/seyahat.rs
- `Seyahat { yer, sebep, bas, bit }` (yerel gün no). `Etkinlik` tablosu `ETKINLIKLER` (yıllık + yıla özel bayramlar 2026-2027).
- `gun_no(y,m,d)` — Hinnant days-from-civil. `yil(gun)`.
- `gunde(gun) -> Option<Seyahat>` — bu yıl ve geçen yıl (yılbaşı sarkması) için tabloyu tarar; yer = `(y + ay*31 + gun) % yerler.len()` ile sabit.
- `bugun()`, `simdi()`, `yarin()` (yarın başlayan, bugün olmayan). `durum_metni()` — "Şu an X'desin (...); n gündür, m gün sonra dönüyorsun" / "Yarın X'ye gidiyorsun" / boş.

## src/gelisim.rs
- `Evre { ad, min_gun, min_sohbet, sans, durtme, aciklama }`, `EVRELER` (4 evre), `ISIM_EVRESI = 2`.
- `Gelisim { dogum, sohbet, mesaj, evre, isim }` — `yukle()` `durum/gelisim.md`'den (yoksa doğum = şimdi), `kaydet(&Gelisim)` `anahtar: değer` satırları.
- `gun(&Gelisim)` doğumdan bu yana gün. `hak_edilen(&Gelisim)` gün ve sohbet eşiklerini geçen en yüksek evre. `evre(&Gelisim)` mevcut evre. `evre_metni` "GELİŞİM EVREN" bölümü.
- `isim_temizle(&str) -> Option<String>` — ilk kelime, alfanümerik, 2..20 karakter.
- `Bot::gelisim_kontrol(ctx)` (main.rs) — her biten sohbet ve 6 saatlik turda: hak edilen evre > mevcut ise atlar, kaydeder; evre ≥ ISIM_EVRESI ve isim yoksa `isim_sec`.
- `Bot::isim_sec(ctx)` (main.rs) — `uret(ISIM_SEC, 12)` → `isim_temizle` → her sunucuda `edit_nickname` → `gelisim.isim`, `bot_adi` → varsayılan kanala `uret(ISIM_DUYURU{isim})` + sohbet. Etiket algısı hem seçilen adı hem kullanıcı adını tanır.

## src/promptlar.rs
Yalnız `pub const X: &str = include_str!("../promptlar/x.md");` satırları. Bkz docs/promptlar.md.
