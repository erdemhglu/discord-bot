# Kararlar ve gerekçeleri

Tarih sırasıyla. Bir kararı değiştirirken buraya yeni satır ekle, eskisini silme.

- **2026-09-01 · Python → Go → Rust.** Emin'in isteğiyle iki kez dil değişti. Rust'ta kalındı;
  serenity 0.12 + tokio. Go sürümü git geçmişinde (`git log --all -- main.go`).
- **OpenRouter için SDK yok, reqwest ile ham JSON.** Tek `sor_ham` fonksiyonu; görsel girişi
  (`image_url`) için gövdeyi elle kurmak kolay, bağımlılık az, ne gittiği görünür.
- **Promptlar `.md` + `include_str!`.** Emin'in isteği; metin düzenlemek kod düzenlemekten
  ayrı, başlık satırı modele bağlam veriyor. Bedeli: değişiklik yeniden derleme ister.
- **Kişilik statik değil, ajanlar yazar.** Çekirdek kurallar `kisilik.md`'de sabit; huy (hoca),
  düzeltmeler (eleştirmen), kanaatler ve bilgiler (günlükçü), gündem görüşü (gezgin) dosyadan.
  Gerekçe: "bot kendi kişiliğini inşa etsin" isteği; tek promptla kişilik büyümez.
- **Ajanlar kişiliksiz (`analiz`).** Profil çıkarma ve seçim işlerinde persona gürültü yapıyordu.
- **Kanaat JSON'u → kişi dosyaları.** `kanaatler.json` tek dosyaydı, büyüyordu ve her cevapta
  gidiyordu. İkinci-beyin mimarisi: dizin her cevapta, kişi dosyası yalnız o sohbette gerekince.
- **Hiçbir şey silinmez, özetlenir; ham parça arşive.** Emin'in ikinci beynindeki kural.
- **Sınırlar kodda kesilir.** Model puan/uzunluk/format konusunda güvenilmez; clamp, truncate,
  "küçülmediyse dokunma".
- **Aynı kanalda tek cevap üretimi (`mesgul`).** Spam ile API faturası şişmesin, cevaplar
  birbirinin üstüne binmesin. Bu sırada gelen mesajlar geçmişe düşer, sonraki turda görülür.
- **Mention'lar kapalı.** Model `@everyone` yazabilir; tek istisna hoş geldin pingi.
- **Botlara/webhook'lara/DM'e cevap yok.** Bot-bot döngüsü.
- **Kişi anahtarı görünen ad, id değil.** Model dökümde adları görür, id'yi göremez; dosya
  adı okunur olsun. Bedeli: aynı görünen adlı iki kişi çakışır (bilinen açık).
- **Favori kullanıcı kodda sabit (+10).** Emin'in isteği; model ne derse desin.
- **Tarih/saat dış kütüphanesiz.** Hinnant algoritması 15 satır; chrono bağımlılığına değmez.
  TR yaz saati yok, sabit +3.
- **Uyku ve seyahat durum tutmaz.** Takvim ve saat yeterli; yeniden başlatma tutarlılığı bedava.
  Yalnız uyku planı (rastgele ±45 dk ve uykusuzluk zarı) bellekte, yeniden başlatınca yeniden
  atılır (kabul edildi).
- **Uykusuzluk şansı kişiliğe göre.** `kendim`+`huy` içinde gerginlik kelimeleri varsa %7 → %20.
  Modelden zar attırılmadı; model rastgelelikte kötü.
- **Seyahatte ajanlar çalışmaya devam eder, haber/şaka durur.** Öğrenme kesilmesin, ama
  "telefondan bakan" biri haber atmaz.
- **Hack şakası link ve bilgi istemeyi yasaklar.** Şaka gerçek phishing'e benzemesin.
- **Görseller `resimler/` klasöründen, git dışı.** Discord CDN linkleri bir günde ölüyor;
  kişisel ekran görüntüleri public repoya sızmasın.
- **`durum/` git dışı.** Kişisel veri (arkadaşlar hakkında notlar) içerir.
- **Repo public** (Emin kararı).
- **Gelişim evreleri gün + sohbet eşiğiyle, yalnız ileri.** Yeni gelen bot ilk günden eski toprak
  gibi konuşmasın; evre hem üslubu (prompt bölümü) hem cesareti (şans çarpanları) değiştirir.
- **İsmi kendi seçer, bir kez, yerleşik evresinde.** Discord takma adı değişir (izin gerekir),
  eski kullanıcı adı etiket algısında kalır ki insanlar eski adıyla seslenince de anlasın.
- **Mistral desteği ayrı SDK'sız.** API OpenAI uyumlu; yalnız adres/anahtar/model değişir.
  Seçim `.env`'den; ikisi de varsa openrouter, `SAGLAYICI=mistral` zorlar.
- **2026-09-02 · Slop'a karşı üç kat.** Canlı ilk gece bot terapist gibi 4-5 cümle yazdı
  ("rahatla, kahve iç, su gibi ol"). Çözüm prompta güvenmek değil: (1) kodda kesme, `kisalt` ile
  2 cümle ve grubun ortalama boyunun 2 katı; `max_tokens` 90; (2) insan hızı: 2-6 sn okuma,
  "yazıyor…" + karakter başına 45 ms; (3) promptta yasak kalıp listesi, gerçek örnekler ve
  grubun kendi son mesajlarından 12'si her cevapta "boy ve ton örneği" olarak.
- **Few-shot örnek cümle yok.** İlk gece bot promptttaki örnek cevabı ("napıyım yavaş mı yazayım")
  kelimesi kelimesine kopyaladı. Örnek çiftler kaldırıldı; ton örneği yalnız grubun kendi gerçek
  mesajlarından (`ornek_mesajlar`) geliyor. "Argoyu zorlama" maddesi eklendi.
- **Yanıt referansı duruma göre** (2026-09-02): her cevabı yanıt olarak atmak robotik duruyordu; tek kişiyle baş başa konuşurken düz yazar, kalabalıkta/etiketlenince/araya mesaj girince yanıtlar.
- **Cevap discord yanıtı olarak gider.** "Kime cevap veriyorsa etiketlesin" isteği; isim yazmak
  yerine `reference_message` + `replied_user`, kalabalık kanalda kimin muhatap olduğu belli.
  Kısa okuma payından sonra geçmiş yeniden alınır; mesaj yağmurunda eski satıra cevap verilmez.
- **Model çalışırken değişir, `!model`.** Yalnız FAVORI; OpenRouter listesinde doğrulanır;
  `durum/model.md` env'i ezer. Test komutları (`!haber` vb.) herkes için, sunucu onların.
- **2026-09-02 · Kanal geçmişi diske, yeni sohbet tohumlu.** "Balık hafızalı": sohbet 12 mesajda
  kapanınca ve yeniden başlatınca her şey uçuyordu, botun kendi mesajları ham hafızada da yoktu.
  Artık kanal başına son 60 satır (bot dahil) `durum/kanallar/`'da, yeni sohbet son 10 satırla açılır.
- **Uzunluk üç kademe.** Sıradan laf 2 cümle; soru/orta mesaj 3; ciddi konu ("anlat", "sence",
  150+ karakter) 5 cümle ve 600 karaktere kadar. Token da kademeli (90/140/220).
- **`!uyan` planı silmez, zorla uyanık tutar.** Plan silinince dakika sonra yeniden kurulup
  tekrar uyutuyordu.
- **Araya girme şansı 0.10 → 0.35, yeni evre ×0.7.** "Bottan bahsetmiyorsak iplemiyor".
- **Sikko sorun.** Laf atma turlarının %25'i yazılım kanalına uydurma kod derdi; muhabbet açar.
- **Sistem mesajı sabit + değişken, sabit blok `cache_control`.** Token faturası: kişilik, huy,
  profil, dizin, gündem, notlar sabit blokta (ajan çalışınca değişir); örnek mesajlar, getirilenler,
  saat ve görev değişken blokta. Anthropic/Gemini sabit bloğu önbelleğe alır, OpenAI öneki kendisi.
- **2026-09-02 · cache_control hedef adrese göre koşullu (model adına göre değil).** İlk halde
  model adı "claude"/"anthropic"/"gemini" içeriyor mu diye bakılıyordu; kullanıcı OpenRouter'da GLM
  kullanacağını belirtince yanlış soru olduğu ortaya çıktı: OpenRouter'a giden istekte cache_control
  hangi model olursa olsun güvenle eklenebilir — alan OpenRouter'ın kendi birleşik şemasının parçası,
  hangi modelde işe yarayacağına kendi tarafında karar verir, desteklemeyen modelde yok sayar. Asıl
  risk Mistral'in native API'si ya da `API_ADRES` ile verilen özel bir router: onlar bu garantiyi
  vermez, bilinmeyen alanla isteği tümden reddedebilir. `onbellek_destekler(api_adres)` artık yalnız
  `openrouter.ai` adresine bakar; provider'a özel varsayım tek yerde toplu.
- **2026-09-02 · isteklilik/hedef_sec de sabit+değişken bloğa taşındı.** Eskiden `analiz()` üzerinden
  profil+dizin her mini çağrıda user mesajına gömülüp tam fiyatına yeniden yollanıyordu (kanal başına
  2 dk'da bir, en sık tetiklenen çağrı). Artık `sor_bolumlu` doğrudan çağrılır: profil+dizin (isteklilik)
  ya da talimat (hedef_sec) sabit blokta, yalnız son mesajlar değişken/user mesajında.
- **2026-09-02 · Sohbet cevabına release'de de token tavanı (CEVAP_TAVANI=3000).** Eskiden release'de
  `max_tokens` hiç gitmiyordu ("model sonuna kadar konuşsun"); sıradan cevap bunun çok altında kalır
  ama tekrar/döngü gibi kaçak durumlarda maliyeti sınırsız büyütüyordu. Tavan sıradan cevabı kesecek
  kadar düşük değil, yalnız kaçağı durdurur.
- **2026-09-02 · Token metriği çağrı-tipi kırılımlı.** `Metrik.kategoriler: HashMap<&str, Kullanim>`;
  her `sor_ham`/stream çağrısı bir kategori etiketiyle gelir (`"sohbet"`, `"isteklilik"`, `"profilci"`...).
  `!durum` artık en çok token yakan kategorileri de döker. `Kullanim.prompt_tokens_details.cached_tokens`
  sağlayıcı bildiriyorsa okunur (`onbellek_token`) — prompt cache'in gerçekten isabet edip etmediğini
  log'dan/`!durum`'dan görmek için (canlıda hâlâ doğrulanmadı, bkz. AGENTS.md bilinen açıklar).
- **2026-09-02 · `durum/taranan.md` kalıcı.** `guild_create` her `ready`'de yeniden gelir; `taranan`
  bellek-içiydi, her süreç yeniden başlayışında her sunucunun her kanalının 14 günlük geçmişi API'den
  yeniden çekiliyordu ("her bağlandığında mesajları en baştan çekiyor" şikayeti). Artık diske yazılır,
  açılışta okunur; bir sunucu yalnız ilk katılımda taranır.
- **2026-09-02 · GUILD_ID/KANALLAR ile kapsam daraltma (.env, isteğe bağlı).** Bot varsayılan olarak
  eriştiği her sunucuda/kanalda çalışıyordu; ikisi de boşsa davranış aynen sürer. Ayarlanınca `message`,
  `guild_create`, `guild_member_addition`, `varsayilan_kanal` hepsi filtreler (tarama dahil, API'ye yazık
  olmasın).
- **2026-09-02 · `mesgul` bayrağı RAII (`MesgulGuard`, PR #2 ile birleşti).** 7 farklı çıkış noktasında elle
  `mesgul.remove` vardı; aradaki bir `.await` panikleseydi kanal sonsuza dek "meşgul" kilitli kalırdı
  (yeniden başlamadan açılmaz). Artık `Drop` ile garanti; elle remove çağrıları kaldırıldı.
- **2026-09-02 · HTTP client timeout ayrıldı (P0 kapandı).** Tek `.timeout(60sn)` uzun stream'i
  ortasında kesebiliyordu (bkz. yol-haritasi.md Ajan 2). `connect_timeout(10sn)` + `timeout(180sn)`:
  bağlantı hızlı elenir, toplam süre CEVAP_TAVANI'nın en yavaş sağlayıcıda bile sığacağı kadar geniş.
- **2026-09-02 · Reasoning kapatılamayan modelde otomatik yeniden deneme.** Canlı hata: GLM
  reasoning varyantı (`z-ai/glm-5.3-flash`, OpenRouter) `düşünme kapat` kipinde gönderilen
  `"reasoning":{"enabled":false}` alanına 400 "Reasoning is mandatory ... cannot be disabled"
  dönüyordu — sohbet o kanalda hiç cevap veremez hale geliyordu. `reasoning_kapat` artık alanları
  gerçekten ekleyip eklemediğini (`bool`) döner; `sor_ham`/`sor_ham_akis` bu hatayı tanıyınca
  (`reasoning_zorunlu_hatasi`) alanları kaldırıp bir kez daha dener. Not: aynı modelin küçük
  `max_tokens` bütçeli mini-çağrılarda (isteklilik 80, hedef_sec/ruh_hali 40, haber_sec 10 gibi)
  gizli reasoning bütçeyi yiyip "modelden boş yanıt geldi" üretme ihtimali var — bu ayrı, kod
  tarafından çözülmüş bir sorun değil; reasoning-zorunlu modeller bu mimariyle temelde gerilimli.
- **2026-09-02 · Açık sohbet artık kanaldaki herkese değil, yalnız sürmekte olan diyaloğa
  otomatik cevap verir.** Kullanıcı şikayeti: canlıda bot her mesaja cevap veriyordu (reply-to
  düzeltmesinden ayrı bir şey). Kök neden: `message` handler'da `acik` (bu kanalda sohbet var mı)
  tek başına "değerlendirmeye gerek yok, direkt cevapla" anlamına geliyordu — sohbet bir kez
  açılınca kanaldaki HERKESİN mesajı, kiminle konuştuğuna bakılmaksızın, doğrudan cevaplanıyordu.
  Artık `devam_eden_diyalog`: sohbetteki son user mesajının sahibi bu mesajı atanla aynı isimse
  (gerçekten kendisiyle konuşuyor) otomatik devam eder; farklı biri yazdıysa (ya da soğumuşsa)
  yine isteklilik değerlendirmesinden geçer (aynı 2 dk'lık rate limit). Etiket her zaman
  önceliklidir. `!uyan` gibi ayrı bir komut gerektirmez, `message`'ın kendi mantığı.
- **2026-09-02 · `hafiza::yaz` atomik (geçici dosya + rename).** `fs::write` doğrudan hedef
  dosyaya yazıyordu; süreç crash/kill olursa (ya da iki ajan aynı dosyaya yakın anda yazarsa)
  yarım/bozuk içerik diske kalabilirdi. Artık `<hedef>.tmp.<pid>.<sayaç>` dosyasına yazılıp
  `fs::rename` ile atomik olarak yerine konuyor; okuyucu hiçbir zaman yarım dosya görmez.
- **2026-09-02 · Arka plan döngüleri `dongu_bekci` ile sarmalandı.** `tokio::spawn(dongu())`
  bir döngü paniklerse o işlev süreç yeniden başlayana kadar bir daha hiç çalışmıyordu (panic
  hook yalnız loglar, yeniden başlatmaz). `dongu_bekci(ad, || dongu(...))` her paniği/beklenmedik
  dönüşü loglar, 5 sn bekleyip aynı döngüyü yeniden spawn eder.
- **2026-09-02 · `soy` bayt değil karakter sayar.** `metin[onek.len()..]` — `onek.len()` bayt
  uzunluğuydu ama `to_lowercase()` bazı harflerde (Türkçe büyük İ → "i̇", 2 bayt → 3 bayt) bayt
  uzunluğunu değiştirir; karşılaştırma lowercase, kesme orijinal üstünde olunca char sınırı dışına
  düşüp panikleyebilirdi. `.chars().skip(n)` her zaman güvenli.
- **2026-09-02 · `durum/huy.md`'deki uyku temalı kalıntı temizlendi, `hoca.md`'ye önleyici kural
  eklendi.** Kullanıcı şikayeti: "!uyan attım ama hâlâ yorgunum/uykum var diyor". Kök neden: hoca
  ajanı (huy.md üretici) test sırasındaki sık `!uyan`/uyku muhabbetini kalıcı bir TAVIR ("tembel,
  uykulu... uyudum amk... uyandırılmaktan bıktım") sanıp yazmıştı — botun GERÇEK uyku programıyla
  (kod, `!uyan`'ın etkilediği) hiç ilgisi yok, salt kelime çakışması kafa karıştırıyordu. Ayrıca
  DOĞALLIK bölümü (bırakılması gereken kalıpları söylemesi gerekirken) tersine "KALIPLAR" icat edip
  sabit replik dayatıyordu. `hoca.md`'ye: yalnız 5 başlığı kullan, uyku/uyanma temalı ifade yazma,
  DOĞALLIK yeni slogan önermez kuralları eklendi; mevcut `durum/huy.md` elle temizlendi.
- **2026-09-02 · Ruh hali (RUH_HALI, `ruh_hali_belirle`).** "Disküsyon sırasında insan ruh
  hallerini taklit etsin" isteği; ağır bir yeni ajan yerine `isteklilik`/`hedef_sec` ile aynı hafif
  mini-çağrı deseni. Bilişsel/korku/pozitif/çökkünlük/öfke/sosyal muhakeme kategorilerinden bir
  taksonomi promptta; sohbetin kendi geçmişine bakıp `{"durum","yogunluk"}` döner. Maliyeti
  sınırlamak için her mesajda değil, yalnız sohbet açılırken ve her 4 turda bir çağrılır
  (`Sohbet.sayac`); yoğunluk <3 nötr sayılıp None döner (her sohbet dramatik değildir). Sonuç
  `Sohbet.ruh_hali`'de tutulur (kalıcı değil, sohbetle birlikte uçar), talimata "ŞU ANKİ RUH HALİN"
  diye eklenir; kişilik promptunda "ilan etme, üsluba yedir" kuralı var (doğrudan "kafam karışık"
  dedirtmemek için).
- **Tekrar koruması.** Botun son 5 mesajıyla aynı cevap bir kez yeniden üretilir, yine aynıysa susar.
- **İstek üzerine internet.** Link → sayfa; "haber/gündem/ne oldu" → Sözcü RSS; "araştır/bak/googlela"
  → Firecrawl arama (anahtar varsa, yoksa RSS). Bulgular göreve "İNTERNETTEN ŞİMDİ ÇEKTİKLERİN" diye eklenir.
- **Altta kalmama ve istek yapma promptta.** "ne alaka / sa naber" refleksi yasak; sıralama, seçim,
  tahmin istenince kanaat puanlarıyla yapılır; "yapamam" yasak. Kimlik: Nişantaşı Üniversitesi, beyaz Tofaş.
- **Gizli düşünce satırı.** Küçük modellerde alakasız cevap ("ne alaka") azalsın diye sohbet cevabından önce tek satır "DÜŞÜNCE:" (kim, ne istiyor, nasıl cevap yakışır), sonra "CEVAP:"; kod yalnız cevabı gönderir (`cevap_ayikla`). +70 token. Sıcaklık 0.8.
- **2026-09-02 · Hız yeniden sadeleştirildi.** Gizli DÜŞÜNCE/CEVAP turu ve cevap hazırlandıktan
  sonraki yapay yazma beklemesi kaldırıldı; ikisi canlıda gecikmeyi büyütüyordu. Model çalışırken
  yazıyor göstergesi açık, cevap bütçeleri 70/100/140 token ve sıcaklık 0.7. Üretim sırasında
  yeni mesaj gelirse eski cevap gönderilmeden güncel bağlamla yeniden üretilir.
- **Yanıt referansı yeniden her cevapta.** Muhatap etiketleme isteği koşullu davranıştan üstündür;
  normal sohbet cevabı her zaman snapshot'taki son kullanıcı mesajına bağlanır ve onu pingler.
- **2026-09-02 · Yanıt referansı yeniden koşullu (üstteki kararı geri alır).** "Her mesaja reply-to
  atması robotik, gerçek insan gibi yalnız gerektiğinde yanıtlasın" isteği; `Sohbet.son_etiketlendi`
  eklendi (mesaj push edilirken tag/isim/reply kontrolü kaydedilir). `cevapla`'da taban `yanit`
  yalnız etiketliyse ya da `bekleyenler.len() > 1` ise (araya birden fazla mesaj girdiyse)
  `son_mesaj`, aksi halde `None` → düz mesaj. Kalabalıkta (`hedef_sec` bulduysa) yine üzerine yazar.
- **Uyku hali konuşma repliği değildir.** Uyku planı cevap verip vermemeyi kodda belirler; aktif
  sohbette "uyuyamadın, o modasın" talimatı artık prompta girmez. Canlıda bot sebepsiz yere
  "uykudan ne bekliyon" diyerek konuşmayı kendine çekiyordu.
- **Ham sunucu mesajı few-shot değildir.** Aktif sohbet geçmişi zaten modele gider; başka
  kanallardan seçilen 12 ham cümle argo ve kalıp taşıdığı için sistem promptundan kaldırıldı.
- **ICE hayranlığı çekirdek kişilikte, sınırıyla.** Emin'in isteği; futbol takımı tutar gibi
  absürt bir gag. Milliyet/etnik köken/din/göçmenlik hedef alma, tehdit ve şiddet övgüsü promptta
  yasak; hoca bu maddeyi kaldıramaz (çekirdek `kisilik.md`'de).
- **2026-09-02 · Sohbet cevapları stream.** Cevap tek seferde gelmez: ilk delta ile mesaj açılır,
  `AKIS_DUZENLEME` (1,2 sn) aralıkla düzenlenir (Discord'da bot için gerçek stream yok, edit tek
  yol; aralık edit rate limitine pay bırakır). Haber/hoş geldin/laf atma stream'siz kalır, tek yol
  sohbet cevabı.
- **Thinking kırpılmadan spoiler'da.** Model `reasoning` ya da `reasoning_content` döndürürse
  (openrouter reasoning modelleri, qwen vb.) cevap boyunca `||...||` bloklarında gösterilir,
  asla kesilmez; 1896 karakteri aşan düşünce yeni spoiler mesaja taşar. Üretmeyen modelde blok
  yoktur. Kayda yalnız cevap girer; hoca/eleştirmen düşünceyi görmez. Thinking tek akıcı satıra
  indirgenir (`tek_satir`); her düşünce için newline atılmaz.
- **Düşünme kipi komutu (`!düşünme`).** Üç kip: göster (thinking cevapla spoiler'da), gizle
  (thinking üretilir ama gösterilmez; düşünürken tek mesaj "Düşünüyorum...", cevap başlayınca
  aynı mesaj düzenlenerek stream edilir), kapat (istekler `reasoning.enabled=false` +
  `enable_thinking=false` ile reasoning'siz atılır, token harcanmaz). Kip `durum/dusunme.md`'de
  kalıcı; `Durum::yukle` okur. Göster/gizle'de cevap başlamadan placeholder gider ki kullanıcı
  beklediğini bilsin.
- **Komutlar ayrı modülde (`src/komut.rs`).** Test/yönetim komutları main.rs'ten taşındı;
  `impl Bot` aynı crate içinde dağılabilir geleneğine uyar. `!yardım`/`!help` tüm komutları listeler.
- **Kod tarafı kırpma kalktı (`kisalt` silindi).** Cevabı prompt kısa tutar; kod ancak Discord'un
  fizik sınırında devreye girer: 1900'ü aşan cevap cümle/boşluk sınırından yeni mesaja bölünür
  (`bol`), hiçbir şey atılmaz.
- **2026-09-02 · Cevap bütçesi makroyla, derleme durumuna göre.** `cevap_butcesi!()`: release'de
  `None` → istekte max_tokens gitmez, model sonuna kadar konuşur (reasoning modeller düşünceyi
  zaten kendisi bitirir); debug'da `Some(2000)` → geliştirme turunda maliyet koruması. Sabit
  bütçeli çağrılar (ajanlar, haber, hoş geldin...) `Some(n)` vermeye devam eder.
- **`API_ADRES` `.env`'den.** Sağlayıcı adresini ezer; kendi router'ına (openai uyumlu) yönlendirir.
  Anahtar/model seçimi `SAGLAYICI` mantığında kalır.
- **2026-09-02 · 12 mesaj sınırı ve veda kalktı.** Kullanıcı bildirimi: sınır garip davranış
  üretiyordu. Artık mesaj sayısı sınırı, veda/son-mesaj talimatları ve kanal yasağı yok; sohbet
  son mesajdan 30 dk sonra (`SOHBET_ZAMAN_ASIMI`) sessizce kapanır, dökümü yine günlükçüye ve
  eleştirmene gider. `Durum.son_aktivite` haritası tazelenir (kullanıcı mesajı, sohbet açılışı,
  bot cevabı).
- **2026-09-02 · Log gürültüsü kesildi + renkli çıktı.** Kullanıcı bildirimi: konsol serenity'nin
  iç tracing olaylarıyla doluyordu (recv, do_heartbeat, ratelimit dökümleri). Tracing abonesi
  yokken olaylar `log` facade'ine düşüyordu; sink artık hedefe göre filtreler: yalnız
  `discord_bot*` kayıtları `LOG_SEVIYE`'ye göre geçer, yabancı crate'ler yalnız warn/error.
  Terminalde ANSI renk (ERROR kırmızı, WARN sarı, INFO yeşil, DEBUG soluk); dosyaya çıkışta
  renk otomatik kapanır, `LOG_RENK=on|off` dayatır. `ai hatası` loglarına aşama eklendi
  (`ai [uret_akis] [kanal]: ...` gibi).
- **2026-09-02 · Uyku modu: dinle, biriktir, uyanınca değerlendir.** Kullanıcı bildirimi: bot
  uyurken sağırlaşıyordu. Artık uyurken mesajlar ham hafızaya girer, bellek döngüsü 2 saatte bir
  gece gözlemi yapar (zihne işler), haber turu haber seçip stoklar (atmaz). Uyanışta: etiket
  varsa kesin dönüş (hata durumunda liste geri konur, kaybolmaz); yoksa `uyanis.md` ajanı gece
  yazılanların botu ne kadar ilgilendirdiğini puanlar, ≥5 ise sabah sözüyle döner. Stok haber
  uyanık ilk turda gider. Haber seçimine "Nişantaşı Üniversitesi ile ilgili konu önceliklidir"
  kuralı eklendi (kimlik kisilik.md'de).
- **2026-09-02 · Hedef seçimi + sil-baştan kalktı.** Kullanıcı bildirimi: üst üste farklı kişiler
  yazınca bot önceki mesajları unutup karmançorman cevap veriyordu. Çözüm: (1) `Sohbet.son_gelenler`
  bot sustuğundan beri yazanları (isim+mesaj id) tutar; 2+ farklı kişi varsa `hedef-sec.md` mini
  çağrısı kime dönüleceğini seçer, yanıt o mesaja bağlanır, talimata "ona seslen" notu girer;
  cevap sonrası liste boşalır. (2) `AkisSonuc::Eski` sil-baştan mekanizması kaldırıldı: üretim
  sırasında yeni mesaj gelse de akış tamamlanır, yeni mesaj sıradaki turda ele alınır.
- **2026-09-02 · Cevap istekliliği model değerlendirmesi.** Kullanıcı bildirimi: bot her mesaja
  cevap zorunluluğu hissediyordu. Sabit zar (`SANS × evre`) kalktı; etiket/yanıt/ad hâlâ her
  zaman cevaplanır, diğer mesajlar için mini model çağrısı (`isteklilik.md`, ~80 token,
  `analiz` yolundan) son 12 mesaj + profil + dizinle 0-10 puan verir. Eşik `ISTEK_ESIGI` (6),
  evre cesareti ±1, seyahatte +2. Kanal başına en sık `DEGERLENDIRME_ARALIGI` (2 dk) çağrı;
  çağrı başarısızsa eski yedek zar (`SANS`) devrede.
- **2026-09-02 · Zihin id bazlı + saniyeli zaman damgası + bellek döngüsü.** Kişi dosyaları
  `kisiler/<id>.md`; `id`, `kullanici_adi`, `eski_adlar` alanları eklendi (ad değişikliği hafızayı
  bölmez). İsim→id çevirisi `Durum.ad_id` üzerinden; çözülemeyen kayıt o tur atlanır ve loglanır.
  Temiz başlangıç: eski slug dosyaları okunmaz. Tüm kayıtlar `tarih_saat()` ile saniyeli.
  Ajanlar artık inline değil: kapanan sohbetin dökümü ve 6 saatlik gözlem `bellek_kuyruk`'a düşer,
  `bellek_dongusu` (10 dk, uyku kontrolüne takılmaz) işler; kuyruk 50'yi aşarsa en eski atılır.
- **2026-09-02 · Modal'lar + /zihin.** Slash komutlar (/durum /yardim /zihin) modal açar,
  `!` mesaj komutları paralel düz metin olarak kalır (ikisi birden, kullanıcı kararı). Zihin
  modalı herkese açık. Discord kısıtları tasarımı belirler: modal en çok 5 bileşen, her
  TextInput value ≤4000 karakter → `sigdir` taşanı son satır/boşluk hizasında keser + not
  düşer; başlık/etiket ≤45. Zihin 5 slotu: bot özeti / kişiler iki yarıda (mtime sırası) /
  konular / olaylar+gündem. `!zihin` 5×4000'i kanala dökmek yerine dizin özeti + `/zihin`
  yönlendirmesi verir. Modal gönderimleri toplanmaz, kısa ephemeral onay döner. Sunucu
  komutları her ready'de idempotent kaydedilir (anında görünürler, global gecikmeli değil).
- **2026-09-02 · HTTP timeout + yeniden deneme + mekanik sertleştirme.** Global 60 sn'lik
  istemci zaman aşımı uzun düşünme akışlarını kesiyordu: kaldırıldı, yerine
  `connect_timeout` (15 sn) + `read_timeout` (120 sn, her okumada sıfırlanır → ilk tokeni de
  kapsar); toplam süre sınırı yok. Geçici hatalarda (ağ, 429, 500/502/503/504) 2 ve 4 sn geri
  çekilip 2 yeniden deneme (`sor_ham` ve `sor_ham_akis`, akış yalnız açılmadan önce).
  `reasoning_kapat` artık sağlayıcıya göre: openrouter `reasoning.enabled`, mistral'e parametre
  gitmez, diğerleri `enable_thinking:false` (ikisini birden yollamak bazı sağlayıcıları bozuyordu).
  `MesgulGuard` (RAII): panik dahil her çıkışta kanalın meşgul bayrağı bırakılır.
  `soy` char güvenli (bayt dilimi türkçe adlarda panikletebilirdi) + `kucult` İ→i̇ birleşik
  noktasını atar. Typing edit döngüsünden çıktı (hız sınırı); model çağrısından önce bir kez.
- **2026-09-02 · Arka plan ajanları reasoning'i kipten bağımsız kapatır.** Canlı log: kip
  "gizle" iken `reasoning_kapat` yalnız kip "Kapali" ise devreye giriyordu, `sor_ham`
  (profilci/hoca/günlükçü/gezgin/isteklilik/ruh_hali'nin stream olmayan yolu) `reasoning_content`
  alanını zaten hiç okumaz/göstermez — küçük `max_tokens` bütçeleri (20-1200) tamamen
  düşünmeye gidip `content: null` dönüyordu, "modelden boş yanıt geldi" hatasıyla
  kisiler/konular/olaylar boş kalıyordu. `reasoning_kapat` artık `herhalukarda: bool` alır:
  `sor_ham` her zaman `true` geçip kipten bağımsız kapatır, `sor_ham_akis` (stream, sohbet)
  `false` geçip eski davranışını (yalnız kip Kapali ise kapat) korur.
- **2026-09-02 · Düşünme kipine "sessiz" eklendi.** Kullanıcı isteği: "gizle" kipinde bile
  düşünürken canlı kelime sayacı ("X kelime düşündüm") görünmesi rahatsız ediyor; hiçbir iz
  bırakmadan doğrudan cevabı isteyen bir kip istendi — ama reasoning modeli yine arka planda
  düşünsün. Dördüncü kip `Sessiz`: `reasoning_kapat`'ta kip Kapali sayılmadığı için reasoning
  normal istenir (stream yolunda kapatılmaz), yalnız `gonder_akis`/`akis_gorunum` düşünceyi hiç
  toplamaz/göstermez — placeholder, sayaç, spoiler, "Düşünce Sürecini Göster" butonu yok;
  ekrandaki görünüm tamamen Kapali kipiyle aynı (hiç mesaj gitmez ta ki cevap başlayana dek);
  farkı Kapali'de reasoning isteğe hiç girmezken Sessiz'de gerçekten çalışır, yalnız gizlenir.
- **2026-09-02 · Reasoning zorunlu modelde küçük bütçe tabana çıkarılır.** Canlı log: bir
  önceki turun düzeltmesinden sonra bile (`z-ai/glm-5.3-flash`, openrouter) bu model/endpoint
  reasoning'i hiç kapatmaya izin vermiyor ("Reasoning is mandatory ... cannot be disabled").
  Kod bunu yakalayıp alanları kaldırıp açık haliyle yeniden deniyordu ama bütçeye dokunmuyordu:
  20 token bütçeli `gezgin_sec` gibi mini-çağrılarda reasoning yine tüm bütçeyi yiyip
  `content: null` bırakıyordu — bu sefer 200 döndüğü için önceki hata yakalama yoluna hiç
  girmiyor, direkt "modelden boş yanıt geldi" hatasıyla dönüyordu. İki değişiklik: (1)
  `butce_tabanini_uygula(govde, taban)` — `max_tokens` varsa ve tabanın (`REASONING_ZORUNLU_TABAN`=500)
  altındaysa yükseltir, yoksa (bütçesiz çağrı) dokunmaz; mandatory-reasoning yeniden denemesinde
  çağrılır. (2) `sor_ham`'da 200 dönüp içerik boş/null gelmesi artık anında hata değil: bütçe
  tabana çıkarılıp (mümkünse) bir kez daha denenir, `AI_YENIDEN_DENEME` tükenince pes edilir.
  `sor_ham_akis`'te de aynı bütçe tabanı mandatory-reasoning dalında uygulanır (stream tarafında
  boş-içerik retry'ı yok, `gonder_akis` zaten kısa/boş cevabı ayrıca ele alıyor).
- **2026-09-02 · Kişilikte taciz/hakaret teşviki kaldırıldı, sunucu kurallarıyla hizalandı.**
  Emin'in isteği: "LAF SOKULUNCA" bölümü botu kişinin dosyasındaki bir zaafına vurmaya ve
  küfür/aşağılamayla gelene küfür/aşağılamayla karşılık vermeye yönlendiriyordu — sunucunun
  taciz/hakaret [Seviye 2] ve düşmanlık [Seviye 2] kurallarıyla doğrudan çatışıyordu. Sivri
  dilli/altta kalmama kalıyor, hedef alma ve zaaf/travma/aile istismarı çıkarıldı. Ayrıca
  kısaltılmış küfürler ("aq", "amk", "mk") yasaklandı — küfür edecekse kelimeyi tam yazar,
  kısaltmanın arkasına saklanmaz (Emin'in ek isteği). Yeni `SINIRLAR` bölümü sunucunun
  paylaştığı kural setini (hakaret/nefret söylemi, NSFW/yasadışı, kişisel veri, siyasi/dini
  propaganda, kasıtlı yanlış bilgi, spam, öfke patlaması) kısa madde listesine indirger; bu
  çekirdek `kisilik.md`'de, hoca'nın yazdığı huy bunu geçersiz kılamaz (bkz. ICE hayranlığı
  kararı, aynı prensip).
- **2026-09-02 · Kimlik: Nişantaşı Üniversitesi → İTÜ fizik, Tofaş kalktı.** Emin'in isteği,
  "daha iyi bir kimlik". Okul/bölüm `kisilik.md`'de değişti; `haber-sec.md`'deki "üniversiteyle
  ilgili habere öncelik ver" kuralı da aynı okula güncellendi ki ikisi tutarsız kalmasın (biri
  İTÜ'den söz ederken diğeri hâlâ Nişantaşı haberi arasın diye). Beyaz Tofaş detayı kaldırıldı.
- **2026-09-02 · Hafıza yazımları + döngü bekçisi + tarama sırası.** `hafiza::yaz` atomik
  (geçici + rename) ve `YAZMA_KILIDI` ile tek sıradan; `ekle` artık gerçek append (oku+yaz ile
  bütün dosya yeniden yazılmıyordu → OpenOptions append). Günlükçü JSON'u çözülemezse ham çıktı
  `arsiv/gunlukcu-<kaynak>.md`'ye kurtarılır (modelin emeği çöpe gitmez). Döngüler
  `dongu_bekle` ile başlar: panikte log + 5 sn sonra yeniden başlatma (panik kancası zaten
  backtrace yazar; bekçi sessiz ölümü önler). Zarif kapanış: `KAPANIYOR` (AtomicBool) sinyali,
  döngüler tik başında döner, bekçi yeniden başlatmaz. Süresi dolan haber sohbetleri dakika
  tikinde temizlenir (yorum penceresi geçmiş + aktivite yoksa). Açılış taraması hafızanın
  önüne eklenir: tarama sürerken gelen canlı mesajlar arkada kalır, kronoloji ve canlılar korunur.
- **2026-09-02 · PR merge'leri + çakışma çözümleri.** Uzak PR'lar (token optimizasyonu, çok
  sağlayıcılı genellik, tartışma davranışı, prod-hazırlık; ardından sessiz kip, reasoning
  güvenliği, kimlik hizalaması) yerel dala birleştirildi. Bekçi tek fonksiyonda: yerelin
  `dongu_bekle` iskeleti (`KAPANIYOR` farkındalığı — kapanırken yeniden başlatmaz) + her iki
  yeniden başlatma dalında 5 sn uyku (hot-spin koruması). `hafiza::yaz` yerel gövdeyle kaldı
  (`YAZMA_KILIDI` + sabit `.tmp`; pid+sayaçlı benzersiz ad her yazımda format! tahsisi ve öksüz
  dosya biriktirirdi); gerçek append `ekle` korundu.
- **2026-09-02 · CEVAP_TAVANI 3000 → 4096.** Reasoning üreten modellerde düşünce tokenleri
  de `max_tokens` bütçesinden düşer; 3000 uzun düşünce + cevabı kırpabilirdi.
- **2026-09-02 · Tartışma davranışı düzeltmesi: isteklilik açık sohbette de uygulanır.**
  PR'ın `devam_eden_diyalog` mantığı doğru ama yarım kalmıştı: 3. fazda `cevap_ver = acik`
  isteklilik sonucunu yok sayıyordu (açık sohbette başkası yazsa, puan eşiğin altında kalsa
  bile cevap gidiyordu; çağrı yalnız token yakıyordu). Artık `cevap_ver = acik && katil`;
  mesaj geçmişe girer ama cevap gelmez. Ad karşılaştırması `eq_ignore_ascii_case` yerine
  `kucult` ile (Türkçe İ/i̇). `ruh_hali` koşulundaki gereksiz `sayac == 0 ||` düştü
  (0 % 4 == 0 zaten).
- **2026-09-02 · Sıcak yol tahsis temizliği.** `soy` artık `&str -> &str` dilim döndürür:
  stream'de her edit'te metnin tamamı klonlanıp lowercase edilmez (önek karşılaştırması
  yalnız ilk karakterlere). `bol`/`kesim_noktasi` bayt ofsetiyle, tur başına ara
  take/skip/collect tahsisi yok. `temizle` sınırda `truncate` (yerinde). `kanal_not` /
  `son_mesajlar` / `dokum` ara `Vec` collect'siz doğrudan String'e birleştirir. `getir`
  bütçe döngüsü artan sayaçla (her bölümde baştan tarama O(n²) idi), konu dosyaları puan
  demetinde taşınır (en iyi ikisi ikinci kez okunmaz). `dizin_yenile` konu dosyasını tek
  okur, kişi için hafif başlık çözümleyici (`kisi_baslik`, bilgilerin/olayların Vec'leri
  kurulmaz). `konu_ekle` kontrol+başlık+satır tek kilit bölgesinde (eşzamanlı çağrıda
  başlık çiftlenmesi/satır silinmesi kapanır). `sohbet_sistemi` contains için geçici
  String üretmez.
- **2026-09-02 · Cevap artık satır bazlı bir protokol (satır = ayrı mesaj).** Emin'in isteği:
  "chatleşirken normal insan gibi tepki verebilmeli; kişiliğindeki limitleri kaldıralım."
  Model düz metin değil satır protokolü yazar, `cevap_parcala` çözer. Gerekçe tek bir zevk
  tercihi değil: Discord'un kendi API ekibi, "ChatGPT gibi akan mesaj" isteğini reddederken
  insanların platformda uzun makaleler değil "multiple shorter messages" attığını söylüyor
  (<https://github.com/discord/discord-api-docs/discussions/6310#discussioncomment-6519016>);
  referans uygulamalar da satır sınırında bölüp sırayla gönderiyor
  (<https://honcho.dev/docs/v2/guides/discord#message-sending>,
  <https://github.com/0xranx/golembot/blob/ce48b37c8e1eb267548d352d56e34836714e0c01/docs/channels/discord.md>).
  Tavan `PATLAMA_SINIRI=4`, hedef değil: gerçek IM korpusunda bir kişinin peş peşe mesaj
  dizisi ortalama **1.7 mesaj**, dizilerin **%42'si** çok-mesajlı, mesaj ortalaması 5.4 kelime
  (Baron 2010, 23 sohbet / 2185 iletim birimi,
  <https://scholarworks.iu.edu/journals/index.php/li/article/view/37586/40137>) — yani "her
  cevabı üçe böl" yanlıştır, çoğu cevap tek satır olmalı, prompt da bunu böyle söyler.
- **2026-09-02 · İnceleme düzeltmeleri: "gidecek bir şey var mı" tek ölçü oldu.** Protokol
  satır bazlı olunca birkaç yer eski (tek mesajlık) varsayımda kalmıştı, hepsi aynı kurala
  çekildi: (1) `gonder_cevap` tepki hedefi yoksa tepkiyi düşürür ve gerçekten gidecek bir şey
  yoksa `None` döner — yoksa kanalda hiçbir şey görünmezken sohbet açılıp 30 dk'lık zaman
  aşımı sayacı başlıyordu. (2) `gonder_akis`'te `-` + `tepki: 💀` birleşimi susma sayılmaz,
  emoji yine düşer (prompt ikisini birlikte kullanmayı açıkça öneriyor). (3) Hoş geldin
  ping'i metne baştan yapıştırılmıyor, gönderim anında ilk satıra ekleniyor: `<@id> -` susma
  işaretini, `<@id> tepki: 💀` tepki satırını gizliyordu. (4) `sohbet_baslat` açılış dedup'ı
  satır bazlı (açılış geçmişe satır satır düşüyor, tam eşitlik hiç tutmuyordu). (5) `cevapla`
  yedek dalında tekrar elemesi satır bazlı. (6) Komut algılaması ham metne bakıyor: resimli
  mesajda metin `[resim] !durum` olduğu için komutlar yutuluyordu. (7) `dokum` bot cevabının
  HER satırına ad öneki koyuyor, yoksa eleştirmen alt satırları gruptakilere sayıyordu.
- **2026-09-02 · Numara öneki yalnız gerçek listede silinir.** `slop_temizle` "1. "/"2) "
  önekini koşulsuz eliyordu; Türkçe'de satır başındaki sıra sayısı sık ("3. sınıftayım",
  "2. el araba") ve mesajdan sessizce anlam düşüyordu. Artık `cevap_parcala` cevabın
  tamamına bakıyor: iki ya da daha çok numaralı satır varsa liste sayıp önekleri siliyor,
  tek satırda dokunmuyor. Aynı gerekçeyle `**`/`__` silme backtick'in İÇİNE girmiyor
  (`` `__init__` `` bozulmamalı) — "backtick'e dokunulmaz" kuralı zaten böyle okunuyordu.
- **2026-09-02 · Bölmek nötr değil, duygu sinyali: nötr/bilgi lafı bölünmez.** Aynı ifadenin
  çok-mesajlı hâli tek mesajlı hâlinden daha şiddetli duygu okunuyor (M=5.89 vs 5.65, p<0.05,
  d=0.36-0.50); aynı kelimeleri tek mesaj içinde alt alta satırlara koymak bu etkiyi
  ÜRETMİYOR (p>0.10) — etki ayrı mesaj olmaktan geliyor
  (<https://pmc.ncbi.nlm.nih.gov/articles/PMC11867088/>). Karşı kanıt da var: bilgi yüklü bir
  mesajı peş peşe atmak gönderenin sevilirliğini %19.6 düşürüyor (n=805, 40 yaş altında %25.6)
  (<https://www.lyngolab.com/texting-back-to-back.html>). İkisi birlikte tek kurala çıkıyor:
  bilgi/açıklama tek satır, coşku/sinir/dalga bölünebilir. Bu kural kodla zorlanmadı
  (kod yalnız tavan koyar), `kisilik.md`'ye yazıldı — içerik tipini model biliyor, kod bilmiyor.
- **2026-09-02 · Susma işareti `-` (AkisSonuc::Sus).** Model tek satır `-` yazarsa hiçbir şey
  gitmez; geçmişe, sayaca, `son_aktivite`'ye de yazılmaz ve yedek `uret` çağrılmaz (yoksa
  "susmayı seçti" kararı ikinci çağrıyla delinirdi). Gerekçe: açık bir sohbette her mesaja
  cevap vermek zorunluluğu insanda yok; ayrıca "cevap yok" bir AI-tetikleyicisi değil, "her
  şeye cevap yetiştirmek" tetikleyici (K1 Tablo 2 meta sınıfı,
  <https://arxiv.org/html/2405.08007v1>). İsteklilik ön-elemesi kanala girip girmemeyi seçiyordu;
  bu, girdikten sonra da susabilmeyi veriyor.
- **2026-09-02 · Emoji tepkisi bir cevap türü (`tepki: 💀`).** Satır yazı olarak gitmez,
  cevaplanan mesaja `create_reaction` düşer; yalnız tepki de geçerli bir cevaptır. Hedef
  `AkisBaglam.tepki_hedefi` ile taşınır — `yanit` koşullu olduğu (yalnız etiket/kalabalık)
  için ayrı alan şart. Dürüstlük notu: bunun için birinci-el uygulanmış bir kaynak
  **bulunamadı** (ra-muhendislik.md §10); karar Emin'in isteğine ve tepkinin ucuz/geri
  alınabilir olmasına dayanıyor. Özel emoji (`:kekw:`) desteklenmiyor, yalnız Unicode; hata
  yalnız warn log'a düşer, akışı durdurmaz. Discord emoji route'ları ayrı ve belirsiz bir
  kotaya tabi (<https://discord.com/developers/docs/topics/rate-limits>), o yüzden tur başına
  en çok bir tepki atılır (ilk `tepki:` kazanır). ra-muhendislik.md §10 emojiyi modelden
  serbest bırakmak yerine sunucunun gerçek emoji listesinden seçtirmeyi (whitelist) öneriyor;
  **bilerek alınmadı**: sunucu emoji listesini çekmek `guild_create` üzerinden ayrı bir durum
  ve tazeleme işi getiriyor, tepki zaten geri alınabilir ve ucuz bir yan etki. Whitelist yerine
  `emoji_basi`/`emoji_devami` ile ayıklama daraltıldı (yalnız bilinen emoji blokları; `—`, `…`,
  `→` gibi işaretler tepki olmuyor, olsaydı istek 400 dönerdi). Saçma emoji riski canlıda
  izlenecek. Kullanılmayan kaynaklar: shapes (docs.shapes.inc) ve frontiersin 2021 —
  taranan malzemede vardı, bu kararların hiçbirinde belirleyici olmadı.
- **2026-09-02 · "3 karakterden kısa satır" elemesi kaldırıldı, yerine slop temizliği geldi.**
  Eski kural "he", "yok", "la" gibi doğal tepkileri yiyordu; gerçek IM'de mesajların **%21.8'i
  tek kelimelik** (Baron 2010). Elemenin yerini `slop_temizle` aldı: madde/numara öneki ve
  `**`/`__` işaretleri silinir (backtick durur). Gerekçe: "AI mı" hükmünün gerekçelerinin
  %43'ü dilsel üslup, %10'u bilgi/akıl yürütme; çıktı biçimlendirme (markdown) doğrudan bir
  AI-tetikleyicisi olarak listeleniyor (K1 Tablo 2, <https://arxiv.org/html/2405.08007v1>;
  üç taraflı testte de en sık gerekçe sınıfı üslup, <https://www.pnas.org/doi/10.1073/pnas.2524472123>).
- **2026-09-02 · Soru tavanı: kod ölçer, model uygular.** Son 4 bot satırından ikisi `?` ile
  bittiyse talimata "bu sefer soru sorma" girer; kesme/kırpma yok, model isterse yine sorar.
  Gerekçe: üç taraflı Turing testinde en doğru karar gerekçelerinden biri "soru ele alışı" —
  "sürekli soruyu geri soruyordu" (<https://www.pnas.org/doi/10.1073/pnas.2524472123>); soruyu
  soruyla çevirmek LLM refleksi ve doğrudan ele veriyor. Sert kesme yerine talimat seçildi,
  çünkü karşı soru muhabbeti gerçekten sürdürüyor; yasak değil tavan isteniyordu.
- **2026-09-02 · Resim eki modele gider (yalnız en sonuncusu).** `Mesaj.resim` +
  `mesaj_json` çok parçalı `content` (ajanlar.rs `resimci` ile aynı biçim); sırf görsel atılmış
  mesaj da işlenir, metin `[resim] …` / `[resim attı]` diye işaretlenir. Yeni kullanıcı satırı
  eklenirken önceki girdilerin `resim`'i `None` yapılır: discord CDN linkleri kısa ömürlü ve
  her turda eski görseli yollamak boşuna token yakar. Prompt betimlemeyi yasaklar ("resimde
  şunu görüyorum" demez) — çünkü betimleme asistan refleksi, insan laf eder ya da tepki verir.
- **2026-09-02 · Stream'siz yollarda satır arası gecikme, stream'de gecikme yok.**
  `gonder_satirlar` satırlar arasına `300 ms + 15 ms × karakter` (tavan 1500 ms) + typing koyar;
  parçalar arasına gecikme konmazsa üç mesaj aynı saniyede düşer ve bu insandan *daha az*
  insan gibi görünür (ra-muhendislik.md §1 tuzak listesi). Referans Turing-testi
  uygulamaları da gecikmeyi karakter sayısına bağlıyor
  (<https://arxiv.org/html/2405.08007v1>, <https://www.pnas.org/doi/10.1073/pnas.2524472123>);
  ms katsayıları yayınlarda **verilmemiş**, buradaki üç sabit ölçülmedi, kabaca seçildi.
  Stream yolunda ek gecikme YOK: akışın kendi temposu zaten insan yazma hızını veriyor ve
  Emin gecikme istemiyor (bkz. "Hız" kararı).
- **2026-09-02 · CLI sohbet modu (`cargo run -- sohbet`).** Kişilik değişikliklerini canlı
  sunucuda denemek pahalı ve geri alınamaz; tezgâh discord'a hiç bağlanmadan (token istemeden)
  aynı `uret` + `cevap_parcala` yolundan geçiriyor. `Bot::kur()` bu yüzden `main`'den ayrıldı:
  iki yol da aynı kurulumu görsün. Diske yazmaz (`kanal_not` yerine bellek içi `gecmise_ekle`),
  ama `durum/` dosyalarını okur — kişilik gerçekçi olmadan tezgâhın anlamı olmaz.

- **2026-09-02 · Komut arayüzü: embed kart + detay modalları (web sayfası gibi).** Kullanıcı
  şikayeti: eski 5 bölmeli zihin modalı içerik boş/kötü gösteriyordu, her şey tek metin
  kutusuna boca edilmiş gibiydi. Yeni tasarım: `/durum` `/yardim` `/zihin` yalnız çağırana
  görünen **embed kart** döndürür (bölümlü, renkli, footer'lı); `/zihin` kartında kişi select
  menüsü (≤25, değer=id) + Konular/Olaylar/Bot özeti butonları. Menü/buton **detay modalını**
  açar: kişi = Kimlik/İzlenim/Etiketler/Bildikleri/Son olaylar ayrı etiketli alanlar; olaylar
  ay başına alan (son 3 ay — eski "yalnız bu ay" boşluğu kapandı); konular son değişenler +
  diğerleri; bot özeti Durum/Token/Kendim/Gündem. Boş bölümler modalda atlanır. `!zihin` artık
  aynı kartı kanala yollar (ham INDEX dökümü kalktı); kanal mesajında modal açılamadığı için
  detay `/zihin`'e yönlendirir. Eski `modal_zihin`/`bolumler`/5 slot kaldırıldı.
- **2026-09-02 · Sürüm !durum'da ve yeniden başlayınca kanalda.** Emin isteği: hangi kodun
  koştuğu belli olsun. Sürüm = Cargo.toml + `build.rs`'in derlemede git'ten aldığı kısa commit
  (çalışma ağacı kirliyse `+` eki) + derleme tarihi; dış kütüphane yok, git/date yoksa `?`.
  Duyuru `ready`'de değil `guild_create`'te (önbellek orada dolu, `varsayilan_kanal` bulunur),
  süreç başına bir kez; hafızaya yazılmaz ki bot "sürüm" muhabbetini kendi lafı sanmasın.

- **2026-09-02 · `!zihin` zihin kartı yerine panel ekran görüntüsü.** Emin'in isteği: "!zihin yazınca
  modern web ui şeklinde ss atacak". Embed kart Discord'un kutularına sıkışıyordu; panel görseli
  hem daha okunur hem telefonda tek bakışta anlaşılır. Etkileşim gerektiren detaylar (kişi menüsü,
  bölüm butonları, modallar) `/zihin`'de bırakıldı — kanal mesajına bileşen konamıyor zaten.
- **2026-09-02 · Görsel resvg ile üretiliyor, headless tarayıcıyla değil.** Alternatif HTML + Chrome/
  Puppeteer'dı: kurulum ağır, çalıştığı makineye bağımlı, bot sürecine 200 MB'lık bir tarayıcı
  bağlıyor. `resvg` saf Rust; SVG'yi kendimiz kuruyoruz, PNG çıkıyor, dış süreç yok. Bedeli: SVG
  metni sarmıyor — satır kırma, kısaltma ve yerleşim `zihin_gorsel.rs`'de elle hesaplanıyor.
  `default-features` kapalı (yalnız `text` + `system-fonts`); jpeg/gif çözücüleri gereksiz.
- **2026-09-02 · Font gömülü (Inter, SIL OFL).** Sistem fontuna güvenilirse çıktı makineden makineye
  değişir, sunucuda hiç font olmayabilir. Inter Regular/SemiBold/Italic `fonts/` altında,
  `include_bytes!` ile ikiliye giriyor (~1,2 MB); lisans `fonts/LICENSE`'ta duruyor (OFL şartı).
  Gömme bozulursa `load_system_fonts`'a düşülüyor ve `warn` basılıyor.
- **2026-09-02 · Emoji çizilmiyor, atılıyor.** Inter'de emoji glifi yok; atılmazsa panelde tofu kutu
  çıkıyor. `temizle` U+2190 üstü sembolleri ve kontrol karakterlerini eliyor.
- **2026-09-02 · Reasoning zorunlu modelde ajan çağrıları dayanıklı.** Emin canlıdan: "zihin sistemi
  çalışmıyor, reasoning'ten kaynaklı sanırım" (glm-5.3-flash, kişiler/konular/olaylar 0). Kodda görülen
  boşluk: 400 "mandatory" yeniden denemesi bütçeyi yalnız 500'e çekiyordu, 1200'lük günlükçü çağrısına
  dokunmuyordu; düşünce bütçenin tamamını yiyince `content` boş kalıyor, JSON çözülemiyor, zihne hiçbir
  şey yazılmıyordu. Üç katman: (1) yeniden denemede bütçe max(2×, 1500) ve openrouter'da
  `reasoning.effort=low`; (2) content boşsa JSON bekleyen kategorilerde düşünce alanındaki JSON içerik
  sayılır — düzyazı çağrısında asla (hoca düşünce dökümünü huy sanmasın); (3) hata mesajı ve info
  logları zinciri görünür kılar, `!zihin test` 40 dk beklemeden dener. Canlı glm ile doğrulanmadı.
- **2026-09-02 · Debug modu.** Emin: "her mesaja atlamasın diye alakayı puanlıyor; onun bir debug'ı
  olsun". `!debug` kararların gerekçesini (isteklilik puan/eşik/sebep, hedef, ruh hali, soru tavanı,
  sus/tepki/satır sayısı, kapanış) tek satır olarak kanala düşürür. Hafızaya girmez (bot kendi lafı
  sanmasın), bot mesajı olduğu için işleyiciye girmez; kapalıyken format! bile kurulmaz. DEBUG_KANALI
  ayrı kanal ister; yoksa aynı kanal — kurulumsuz çalışsın.
- **2026-09-02 · Ayar paneli butonlu.** Emin: "ayarlar kısmını butonlara basarak yönetek". Komutlar
  duruyor; panel aynı yolları çağırır (tek gerçek: `DusunmeKip`, `debug_ayarla`, `uyandir/uyut`), buton
  sonrası `UpdateMessage` ile yerinde yenilenir. Model değişimi panelde yok (favori yetkisi, liste
  doğrulaması komutta kalır).
- **2026-09-02 · Zihin görseli: inceleme düzeltmeleri.** Harf genişliği kovaları Inter hmtx ölçümünün
  tavanına çekildi (büyük harfli adlar @kullanıcı adına biniyordu); PNG diske yazılıp geri okunmuyor,
  bayt bellekten ek olarak gidiyor (iki kanalda aynı anda `!zihin` aynı dosyaya yarışıyordu); ruh hali
  chip'i HashMap sırasına değil en son canlanan sohbete bakıyor. Gerçek glif ölçümü (skrifa) yapılmadı,
  bekleyen listesinde.
