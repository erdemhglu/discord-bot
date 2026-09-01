# Modüller ve fonksiyonlar

Her satır: imza · ne yapar · kim çağırır · kilit/await notu. Satır numaraları yaklaşıktır,
`grep -n "fn ad"` ile bul.

## src/main.rs

### Tipler
- `Mesaj { role: &'static str, content: String }` — OpenRouter'a giden mesaj. `kullanici(..)`, `asistan(..)` kurucular.
- `Sohbet { gecmis: Vec<Mesaj>, sayac: u32, hackli: u32 }` — bir kanaldaki açık sohbet. `sayac` botun yazdığı mesaj sayısı; `hackli` hack şakasında kalan cevap sayısı.
- `Durum` — tek paylaşılan durum (bkz mimari.md). `Durum::yukle()` diskten profil/huy/duzeltmeler/kendim/gundem okur, dizini yeniler.
- `Bot { durum: Mutex<Durum>, http: reqwest::Client, anahtar, haber_kanali: Option<ChannelId>, firecrawl: Option<String> }`.
- `Bot::durum() -> MutexGuard<Durum>` — zehirli kilidi de açar. **Await üstünde tutma.**
- `Handler { bot: Arc<Bot>, baslatildi: AtomicBool }` — serenity `EventHandler`.
- `Hata = Box<dyn Error + Send + Sync>`.

### Yardımcılar
- `simdi_unix() -> i64` — şu an, unix saniye.
- `ad(&User) -> String` — görünen ad (`global_name`), yoksa kullanıcı adı. Hafıza ve kişi dosyaları bu adla.
- `kanal_not(&mut Durum, kanal, satir)` — kanal geçmişine (bellek 60 + `durum/kanallar/<id>.md`) ekler; kullanıcı satırları `message`'dan, bot satırları `gonder`'dan.
- `hatirla(&mut Durum, isim, metin)` — ham hafızaya "isim: metin" ekler, 2000'i aşarsa baştan atar.
- `son_mesajlar(&Durum, n) -> String` — ham hafızanın son n satırı, `\n` ile.
- `dokum(&[Mesaj], bot_adi) -> String` — sohbeti "isim: metin" satırlarına çevirir, bot satırları `bot_adi:` ile.
- `temizle(String, bot_adi) -> String` — model çıktısı: baştaki `bot_adi:` kalıbı ve dış tırnak atılır, 1900 karakterde kesilir.
- `ortalama_boy(&Durum) -> usize` — son 200 ham mesajın ortalama karakteri (boşsa 60). Sohbet cevabı sınırı = 2× bu, 40..220.
- `kisalt(metin, sinir) -> String` — ilk iki cümlede ya da karakter sınırında (kelime sonunda) keser; son nokta/virgülü atar. Yalnız normal/veda sohbet cevaplarına uygulanır (hack, hoş geldin vb. hariç).
- `ornek_mesajlar(&Durum) -> String` — son 300 ham mesajdan 4..100 karakterlik 12 tanesi; sistem mesajında "GRUBUN GERÇEK MESAJLARI" bölümü (boy ve ton örneği).
- `json_ayikla(&str) -> &str` — ilk `{` ile son `}` arası (kod bloğu süsünü atar).

### OpenRouter (impl Bot)
- `sor_ham(Value) -> Result<String>` — POST `/chat/completions`, `choices[0].message.content`; boşsa hata. Tek HTTP noktası. Zaman aşımı istemciden (60 sn).
- `sor(sistem, gecmis, max_tokens)` — `system` + geçmiş → `sor_ham`.
- `uret(gecmis, talimat, max_tokens)` — **kişilikle konuşan tek yol.** Geçmişteki `user` mesajlarından katılımcı adlarını (`"isim: "` öneki) ve metinleri çıkarır → `hafiza::anahtarlar` → kilit altında `hafiza::getir` + `sistem_metni` → `sor` → `temizle`. Çağıranlar: cevapla, dürtme, şaka, haber tanıtma, hoş geldin, uyandım, gezgin notu, resimci yedeği.
- `analiz(metin, talimat, max_tokens)` — **kişiliksiz tek yol.** Sistem = `ANALIST`; kullanıcı mesajı = `metin + "---" + talimat`. Çağıranlar: profilci, gunlukcu, hoca, elestirmen, ozetleyici, haberci seçim, gezgin seçim.
- `gonder(ctx, kanal, metin, ping, dosya, yanit: Option<MessageId>)` — `yanit` verilirse discord yanıtı (`reference_message`) olur ve yanıtlanan kişi pinglenir (`replied_user`).  mention'lar kapalı (`CreateAllowedMentions::new()`, yalnız `ping` açılır), isteğe bağlı ek dosya; başarılıysa `kendi_mesajlarim`'a (50) ekler. Kilit gönderimden SONRA alınır.
- `Bot::sor_bolumlu(sabit, degisken, gecmis, max_tokens)` — sistem mesajını iki metin bloğu olarak gönderir, ilki `cache_control: ephemeral`. `sor` bunu boş değişkenle çağırır.
- `Bot::tekrar_mi(kanal, cevap)` — kanal geçmişindeki son 5 bot satırıyla aynı mı. `Bot::arastir(metin) -> Option<String>` — link/haber/araştır tetiklerine göre sayfa, RSS ya da Firecrawl arama sonucu.
- `sistem_metni(&Durum, talimat, getirilen) -> (String, String)` — (sabit, değişken);  bölümleri sırayla ekler (mimari.md listesi). Serbest fonksiyon, kilit çağıranda.

### Sohbet motoru
- `Bot::sorun_at(ctx, kanal)` — `uret(SORUN, 160)` ile uydurma kod derdi, gönder, sohbet aç. Dürtme döngüsü (%25) ve `!sorun`.
- `sohbet_baslat(&mut Durum, kanal, acilis: Option<String>) -> &mut Sohbet` — kanal geçmişinin son 10 satırıyla tohumlar (bot satırları assistant), açılış mesajı geçmişte zaten varsa iki kez koymaz;  varsa mevcut sohbeti döner (`entry().or_insert`), yoksa yeni; açılış varsa `asistan` mesajı + `sayac=1`.
- `sohbet_bitir(&mut Durum, kanal) -> Option<Sohbet>` — haber bekleme silinir, kanal 3 saat yasaklanır, sohbet çıkarılıp döner (günlükçüye gider).
- `girebilir_mi(&Durum, kanal) -> bool` — yasak süresi geçmiş mi.
- `Bot::komut(ctx, msg, komut, arg) -> bool` — test/yönetim komutları (bkz README). `Bot::model_var_mi(id)` OpenRouter `/models` listesinde arar; liste çekilemezse engel olmaz.
- `Bot::haber_at(ctx, kanal) -> bool` — haberci → link → tanıtım → gönder → sohbet + 2 saat yorum bekleme. `haber_dongusu` ve `!haber` çağırır.
- `Bot::saka_yap(ctx, kanal, hack)` — görsel seç, hack ise `HACK_GIRIS`, değilse `resimci`; gönder; sohbet (`hackli=3`). `saka_dongusu` ve `!saka`/`!hack` çağırır.
- `Bot::cevapla(ctx, kanal)` — döngü: (1) kilit: meşgulse çık; sohbet yoksa çık; talimat seç ve meşgul işaretle. (2) 0,4-1,2 sn okuma payından sonra bu sırada gelenler dahil güncel geçmişi, son Discord mesajını ve `gelen` sayacını al; `uret` (90 token); `kisalt` (2 cümle / 2×ortalama boy). (3) `broadcast_typing` + karakter×25 ms (0,35-2,5 sn); son mesaja Discord yanıtı olarak `gonder`, böylece kişi pinglenir. (4) meşgul kaldır, asistan mesajını ekle, sayaçları ilerlet. (5) sohbet bitmediyse üretim sırasında yeni mesaj gelmişse başa dön; yoksa çık. Biten sohbet `gunlukcu` ve `elestirmen`e gider.

### Hafıza (discord tarafı)
- `gecmisi_oku(bot, ctx, guild)` — botun üyeliğini çeker, izinli (`VIEW_CHANNEL|READ_MESSAGE_HISTORY`) metin kanallarını pozisyon sırasıyla gezer, `GetMessages` 100'lük sayfalarla 14 gün geriye okur, bot/boş mesajları atlar, `content_safe` ile mention'ları ada çevirir, zamana göre sıralar, son 2000'i `hatirla`; favori id görürse `favori_adi` yazar.
- `varsayilan_kanal(bot, ctx) -> Option<ChannelId>` — `HABER_KANALI` → sunucu sistem kanalı → en üst metin kanalı. Önbellekten, await yok.
- `bos_kanal(bot) -> Option<(ChannelId, String)>` — son konuşulan kanal; sohbet açık değil, yasaklı değil, profil var → (kanal, son 40 satır). Dürtme ve şaka bunu kullanır.

### Döngüler (tokio::spawn, `ready`'de bir kez)
- `haber_dongusu(bot, ctx)` — 6 saatte bir: uyanık değilse geç; seyahatteyse profilci+hoca, geç; profilci → gunlukcu(son 300, "gözlem") → hoca → varsayılan kanalda sohbet açıksa geç → `haberci` → link (http değilse HN sayfası) → `uret(HABER_TANIT)` → gönder → sohbet başlat (açılış = tanıtım), `haber_bekleyen` = +2 saat, `atilan_haberler` ekle.
- `durtme_dongusu(bot, ctx)` — saatte bir: uyanık değilse geç; seyahatteyse günde bir kez %25 ile `YOLDA`; yarın seyahat başlıyorsa bir kez `GIDIYORUM`; değilse %30 ile `DURUP_DURURKEN`; `bos_kanal` → `uret(son 40 satır)` → gönder → sohbet başlat.
- `saka_dongusu(bot, ctx)` — 3 saatte bir: uyanık değilse/seyahatteyse geç; %10; `bos_kanal`; `rastgele_resim` yoksa geç; %30 hack: `uret(HACK_GIRIS)`, değilse `resimci(resim)`; görselle gönder; sohbet başlat, hack ise `hackli = 3`.
- `gezgin_dongusu(bot)` — ilk 10 dk sonra, sonra 4 saatte bir, uyanıksa `gezgin`.
- `Bot::uyku_gecisi(ctx)` — uyudu/uyandı geçişini loglar, uyanınca bekleyen etiketlere `UYANDIM` ile döner; döngü ve `!uyan`/`!uyu` çağırır.
- `uyku_dongusu(bot, ctx)` — dakikada bir: `uyku::guncelle`, uyandı/uyudu geçişini loglar; uyanınca `bekleyen_etiketler` varsa son etiketin kanalına `uret(UYANDIM)` ile döner, sohbet başlatır.

### Discord olayları (Handler)
- `ready` — bot adını yazar; `baslatildi` ilk kez ise beş döngüyü başlatır.
- `guild_create` — `taranan`'a ilk kez giriyorsa arka planda `gecmisi_oku → profilci → hoca (huy boşsa)`.
- `guild_member_addition` — kanal: sunucu sistem kanalı → varsayılan; favori ise adını kaydet; sohbet açık/yasaklıysa çık; `uret(HOS_GELDIN)` → mention'lı gönder (ping açık) → sohbet başlat.
- `message` — bot/webhook/DM ise çık; `content_safe`; boşsa çık. Kilit: etiketlendi mi (mention listesi, yanıtlanan mesaj botun mu, metinde bot adı geçiyor mu) → `hatirla`, `son_kanal`, favori adı; haber bekleme süresi dolduysa sohbeti kapat; **uyuyorsa**: etiketlendiyse `bekleyen_etiketler`'e (20) ekle, çık; sohbet yoksa: etiketlendiyse ya da (yasaklı değil ve şans tuttuysa; seyahatte şans ×0.3) başlat; sohbet varsa kullanıcı mesajını geçmişe ekle (20'de tut). Kilit dışı: `cevapla`.

### Başlangıç
- `ayar(isim)` — boş olmayan env değişkeni ya da açık hata.
- `kapanis_bekle()` — ctrl-c veya SIGTERM.
- `main` — `.env`, anahtarlar, `durum/{kisiler,konular,olaylar,arsiv}` ve `resimler/` klasörleri, `Durum::yukle` + `uyku::guncelle`, reqwest 60 sn, intents `GUILDS|GUILD_MESSAGES|GUILD_MEMBERS|MESSAGE_CONTENT`, kapanışta `shard_manager.shutdown_all`.

## src/ajanlar.rs (impl Bot)
- `profilci()` — son 600 satır → `analiz(PROFIL_CIKAR, 1200)` → `profil.md` + `Durum.profil`.
- `gunlukcu(dokum, kaynak, kanal)` — `analiz(GUNLUKCU{ad,kaynak,favori}, 1200)` → JSON `Kayit{olay, kisiler[{isim,puan_degisimi,not,bilgiler,etiketler}], konular[{ad,not}], kendim}` → olay satırı (`olay_ekle`), her kişi: dosya oku, puan += clamp(-3..3) sonra clamp(-10..10), not değişirse, bilgiler tekrar etmeden, etiketler ≤6, olay satırı kişiye de, favori ise +10 ve sabit not, `kisi_yaz`; konular `konu_ekle`; kendim → `kendim.md`; dizin yenile; sonra `ozetleyici`.
- `ozetleyici()` — `hafiza::sinir_asanlar()` için: kişi → `analiz(OZETLEYICI_KISI{sinir=1000}, 700)`, konu → `OZETLEYICI_KONU{800}`, olay → eski %60 satır `OZETLEYICI_OLAYLAR` ile 3-5 satıra, yeni %40 kalır. Sonuç boş değil ve eskisinden kısaysa: kişi/konu için eski dosya arşive, yeni yazılır; olayda taşınan satırlar arşive. Küçülmediyse dokunmaz. Dizin yenile.
- `hoca()` — profil + dizin + gündem + kendim + mevcut huy + son 200 satır + botun son mesajları → `analiz(HOCA{ad}, 800)` → `huy.md`.
- `elestirmen(dokum)` — `analiz(ELESTIRMEN{ad,mevcut}, 400)` → `duzeltmeler.md`.
- `haberci() -> Result<Haber>` — HN ilk 12 (atılmamış) + Sözcü RSS ilk 12 (atılmamış, kimlik = link hash) → liste "n. [hn|gündem] başlık" → `analiz(HABER_SEC{profil}, 10)` → numara → `Haber{id,title,url,score,kaynak}`.
- `resimci(&PathBuf) -> Result<String>` — görseli base64 `image_url` olarak sistem=`sistem_metni(RESIM_AT)` ile `sor_ham`; hata olursa `uret` ile körlemesine. `temizle`.
- `rastgele_resim() -> Option<PathBuf>` — `resimler/` içinden png/jpg/jpeg/gif/webp.
- `Haber` — serde; `kaynak` `#[serde(skip)]`, `score` HN dışı 0.

## src/hafiza.rs
- Sabitler: `KISI_SINIRI 1800 / KISI_HEDEF 1000 / KONU_SINIRI 1500 / KONU_HEDEF 800 / OLAY_SINIRI 6000 / BAGLAM_BUTCESI 6000 / DIZIN_KISI 40 / FAVORI_NOTU`.
- `yol(parca)`, `oku(parca)`, `yaz(parca, icerik)` (üst klasörü açar), `ekle(parca, satir)` (özel), `arsivle(parca, icerik)` (`arsiv/parca`'ya tarihli başlıkla ekler).
- `slug(isim)` — küçük harf, Türkçe harf sadeleştirme, alfanümerik dışı `-`, boşsa "bilinmeyen".
- `tarih()`, `tarih_unix(unix)` (Hinnant civil-from-days), `ay()` "YYYY-AA".
- `Kisi { isim, puan, etiket, not, bilgiler, olaylar }` — `coz(isim, metin)` dosyadan, `metin()` dosyaya. Biçim: `# İsim` / `puan: +3` / `etiket: a, b` / `not: ...` / `## Bildiklerin` `- ...` / `## Son olaylar` `- tarih: ...`.
- `kisi_oku(isim)`, `kisi_yaz(&Kisi)` — `kisiler/<slug>.md`.
- `konu_ekle(ad, not)` — `konular/<slug>.md`, yoksa başlık+etiket satırı, sonra `- tarih: not`.
- `olay_ekle(kanal, olay)` — `olaylar/YYYY-AA.md`'ye `- tarih #kanal: olay`.
- `dosyalar(klasor)` — `.md`'ler, son değişen önce. `ilk_satir(p)`.
- `dizin_yenile() -> String` — `## Kişiler` (≤40: `- ad (+p) · etiketler · not`), `## Konular` (≤30: `- ad · son: tarih`), `## Olaylar` (≤3 ay: `- YYYY-AA · n kayıt`); `INDEX.md`'ye yazar.
- `DURAK` — elenen sık kelimeler. `anahtarlar(&[String])` — 4+ harf, durak değil, tekrarsız, ≤40.
- `puanla(metin, anahtar)` — kaç anahtar geçiyor. `kirp(metin, sinir)` — karakter sınırı + `…`.
- `getir(katilimcilar, anahtar, hafiza, atla_son) -> String` — sırayla: katılımcıların kişi dosyaları (≤4, her biri ≤1200), en çok eşleşen 2 konu dosyası (≤800), ayın son 8 olayı, ham hafızadan (son `atla_son` hariç) ≥2 anahtarla eşleşen en fazla 12 satır (puan sonra yenilik sırası, sonra kronolojik). Bütçe 6000 karakter; sığmayan bölüm ve sonrası atlanır.
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
- `YOLDA_SANS_CARPANI 0.3`. `Seyahat { yer, sebep, bas, bit }` (yerel gün no). `Etkinlik` tablosu `ETKINLIKLER` (yıllık + yıla özel bayramlar 2026-2027).
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
