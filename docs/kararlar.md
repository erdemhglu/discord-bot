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
