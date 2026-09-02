# discord-bot

> Geliştirici ve yapay zeka ajanları için giriş: [AGENTS.md](AGENTS.md) · ayrıntı [docs/](docs/)

Sunucuda takılan, insanları tanıyan, zamanla kişilik kazanan bir discord botu.
Rust ile yazıldı, cevapları openrouter (ya da mistral) üzerinden alıyor; openrouter üzerinden
GLM, Grok, Gemini, Claude gibi herhangi bir model kullanılabilir.

## ne yapar

- her sunucuya ilk katılımda son 2 haftanın mesajlarını okur, grubu tanır (bir kere; sonraki başlangıçlarda tekrar taramaz)
- yeni gelenle tanışır, sohbete girer
- arada mesajlaşmaya dalar; sohbet 30 dk sessiz kalınca vedasız kendiliğinden kapanır
- saatte bir %30 ihtimalle durup dururken laf atar, eski konulara gönderme yapar
- 6 saatte bir hacker news + Sözcü gündeminden gruba uygun bir haber atar, 2 saat yorum bekler
- 4 saatte bir Türkiye gündemini gezer (Sözcü RSS, firecrawl varsa sayfayı okur), kendi görüşünü günlüğüne yazar; kişilik buradan da beslenir
- geceleri uyur (01-09), nadiren uykusuz gece geçirir (gerginse daha sık); uyurken yazmaz ama dinler: mesajlar zihne işlenir, haber stoklanır; uyanınca gece yazılanları değerlendirir, ilgisi çekildiyse sabah sözüyle döner
- etiketlenince, adı geçince ya da mesajına yanıt verilince her zaman cevap verir; sohbet açıkken de yalnız az önce KENDİSİYLE konuşan kişiye otomatik devam eder, kanaldaki başkasının mesajına atlamadan önce (gerçek insan gibi) isteklilik değerlendirir
- discord yanıtı (reply-to) yalnız etiketlenince ya da araya birden fazla mesaj girince kullanılır; sıradan tek-kişilik sohbette düz mesaj atar
- her sohbete özgü an'lık bir ruh hali var (bilişsel, korku, pozitif, çökkünlük, öfke, sosyal muhakeme kategorilerinden); üsluba yedirir, ilan etmez
- sohbet cevapları canlı akar: mesaj belirir, yazıldıkça büyür; model düşünce üretiyorsa (reasoning) kırpılmadan spoiler içinde gösterilir, 1900'ü aşan cevap kırpılmaz, yeni mesaja bölünür
- `GUILD_ID`/`KANALLAR` (.env, isteğe bağlı) ile tek sunucuya/kanal listesine kilitlenebilir; boşsa eriştiği her yerde çalışır
- bayram, uzun hafta sonu, yaz, festival zamanlarında seyahatte gibi davranır: az yazar, yoldan mesaj atar, gitmeden haber verir
- arada `resimler/` klasöründen bir görsel atar; bazen hacklenmiş taklidiyle (3 mesaj sürer,
  sonra kendine gelir; link atmaz, kimseden bir şey istemez)
- gelişim evreleri: yeni → ısınma → yerleşik → eski toprak (gün ve sohbet sayısına göre); evre üslubu ve cesareti değiştirir
- yerleşik evresine girince kendine isim seçer, takma adını değiştirir, gruba söyler
- `FAVORI` id'li kişi istisnadır, ne olursa olsun sever

## kişiliği kim yönetiyor

Sohbet eden taraf tek başına karar vermiyor; arka planda ayrı ajanlar çalışıyor
(`src/ajanlar.rs`). Hepsi kişiliksiz, düz analiz yapar ve sonucu `durum/` klasörüne yazar;
sohbet eden taraf her cevapta bunları okur.

| ajan | ne zaman | ne üretir |
|---|---|---|
| profilci | açılışta ve 6 saatte bir | grup profili: kim nasıl konuşur, iç şakalar, konular (`profil.md`) |
| günlükçü | her biten sohbetten sonra ve 6 saatlik gözlemden | kişi puanı (-10/+10) ve notu, konu notu, olay satırı, botun kendi hali (`kisiler/`, `konular/`, `olaylar/`, `kendim.md`) |
| hoca | açılışta ve 6 saatte bir | botun nasıl biri olması gerektiği: mizah, dil, coştuğu konular, tavır, doğallık (`huy.md`) |
| eleştirmen | her biten sohbetten sonra | botun kendi mesajlarına somut düzeltme notları (`duzeltmeler.md`) |
| özetleyici | kişi/konu/olay dosyası sınırı aşınca | dosyayı küçültür, taşan ham parça `arsiv/`'e gider |
| haberci | 6 saatte bir | hacker news + Türkiye gündeminden gruba uygun haberi seçer |
| gezgin | 4 saatte bir | internette gezip görüşünü günlüğe yazar (`gundem.md`) |
| resimci | şaka anında | ekteki görsele kişilikle tek satır yorum (model görseli görür) |
| ruh hali | sohbet açılınca ve her 4 turda bir | o sohbete özgü an'lık ruh hali (kalıcı değil, sohbetle uçar; talimata "ŞU ANKİ RUH HALİN" diye eklenir) |

Ne öğrendiğini görmek için `durum/` klasörüne bak.

## hafıza mimarisi

Bağlam penceresi büyümesin diye ikinci beyin mantığı: işaretçi taşınır, veri iş anında getirilir,
sınır dolunca özetlenir, hiçbir şey silinmez (`src/hafiza.rs`).

```
durum/
  INDEX.md          ne bildiğinin listesi; her cevaba gider (kişi + puan + etiket, konular, olay sayısı)
  huy.md            hoca: nasıl biri olduğu
  profil.md         profilci: grup profili
  duzeltmeler.md    eleştirmen: kendine notlar
  kendim.md         botun kendi hali
  gundem.md         gezgin: internette gezip yazdığı görüşler
  kisiler/<id>.md   kişi başına (discord id, ad değişse de bölünmez): puan, etiket, not, bildiklerin, son olaylar
  konular/<ad>.md   konu başına tarihli notlar
  olaylar/YYYY-AA.md  biten her sohbetten tek satır
  arsiv/            özetlenip çıkarılan ham parçalar
  taranan.md        daha önce 14 günlük geçmişi taranmış sunucu id'leri (yeniden başlayınca tekrar taramasın diye)
```

**Her cevapta ne gider:** çekirdek kişilik + gelişim evresi + huy + profil + dizin + gündem +
kendi hali + kendine notlar + o sohbet için getirilenler + o sohbetin an'lık ruh hali + görev.
Getirilenler sabit bütçeli (6000 karakter): sohbette konuşanların kişi dosyaları, anahtar
kelimeye uyan en fazla 2 konu dosyası, ayın son 8 olayı, ve ham bağlam penceresinden (son 2000
mesaj) konuya değen ama sohbette olmayan en fazla 12 eski satır.

**Kim yazar:** günlükçü ajanı her biten sohbetten ve 6 saatte bir gözlemden JSON kayıt çıkarır;
kod bunu kişi/konu/olay dosyalarına işler. Puan sınırları kodda kesilir, favori sabittir.

**Sınır dolunca:** kişi dosyası 1800, konu dosyası 1500, aylık olay dosyası 6000 karakteri aşınca
özetleyici ajan küçültür (kişi ve konu için hedef 1000/800; olaylarda eski %60 3-5 satıra iner).
Çıkan ham parça `arsiv/` altına tarihli eklenir.

## promptlar

`promptlar/` klasöründe markdown olarak durur, `include_str!` ile derlemeye gömülür.
Metni değiştirip yeniden derlemek yeter. Çekirdek kurallar `kisilik.md`, ajanlar kendi
dosyalarında.

## kurulum

```
cp .env.example .env   # DISCORD_TOKEN + OPENROUTER_KEY ya da MISTRAL_KEY (MODEL ile model seçilir; API_ADRES ile kendi router)
                        # isteğe bağlı: GUILD_ID/KANALLAR (tek sunucuya/kanala kilitler), HABER_KANALI, FIRECRAWL_KEY
cargo run --release
```

discord developer portal'da **Message Content** ve **Server Members** intent'leri açık olmalı.
Şakalarda atılacak görselleri `resimler/` içine koy (png, jpg, gif, webp); klasör git'e girmez.

## komutlar

`!` ya da `/` ile başlar (`/model` de olur).

- `!sifirla` — o kanaldaki açık sohbeti sıfırlar. `!sifirla hepsi` tüm kanallar.
- `!haber` — şimdi haber seç ve at (HN + gündem).
- `!sorun` — yazılım derdi atıp "nasıl çözerim" diye sorar (kendiliğinden de laf atma turlarının %25'inde, haber kanalına).
- `!gez` — gündem gezintisini şimdi yap (gundem.md güncellenir).
- `!saka` / `!hack` — görsel şakası / hacklenmiş taklidi şimdi.
- `!ajanlar` — profilci ve hocayı şimdi çalıştır.
- konuşmalar `durum/kanallar/<id>.md`'de kalır; sohbet bitse ya da bot yeniden başlasa da son 10 satırla devam eder
- `!uyan` — uykuyu şimdi keser, uyurken etiketleyenlere döner. `!uyu [saat]` — test için uyutur (varsayılan 8 saat).
- `!durum` — evre, sayaçlar, model, uyku, düşünme kipi, seyahat, token metriği (kaç çağrı, giriş/önbellek/çıkış, çağrı tipine göre en çok yakanlar).
- `!zihin` — zihin kartını (kişiler/konular/olaylar) kanala yollar; interaktif detay `/zihin`'de.
- `!düşünme` — düşünme kipi. `göster`: düşünürken "Düşünüyorum...", cevapla birlikte thinking hem spoiler hem kod bloğunda. `gizle`: düşünürken canlı kelime sayacı ("Şu ana kadar N kelime düşündüm"), thinking mesajda görünmez, cevap sonunda "Düşünce Sürecini Göster" butonu — tıklayana yalnız ona görünen kod bloğu açılır. `kapat`: istekler reasoning'siz atılır. Seçim `durum/dusunme.md`'de kalır; `aç` = göster.
- `!yardım` / `!help` — komut listesi.

slash komutlar yalnız çağırana görünen şık **embed kartlar** açar: `/durum` ve `/yardim` tek
kart, `/zihin` üç sütunlu kart (Kişiler/Konular/Olaylar) + üstte kişi seçme menüsü + altta
Konular/Olaylar/Bot özeti butonları. Menü ya da buton, ilgili **detay modalını** açar — kişi
kartı Kimlik/İzlenim/Etiketler/Bildikleri/Son olaylar diye ayrı alanlara bölünür, hiçbir şey
tek metin kutusuna boca edilmez. modallar gösterimliktir; içlerinden veri toplanmaz. taşan
alan 4000 karakterde son satır/boşluk hizasında kırpılır ve not düşülür.
- `!model` — şu anki model. `!model <id>` değiştirir (yalnız FAVORI kişi; OpenRouter'da yoksa "yok öyle model"). Seçim `durum/model.md`'de kalır, yeniden başlatınca korunur.

## ayarlar

`src/main.rs` başındaki sabitler: mesaj sınırı, cevap token tavanı, araya girme şansı, bekleme
süreleri, şaka sıklığı, favori kişi. Tüm sabitlerin listesi: `docs/sabitler.md`.

## güvenlik

- mention'lar kapalı gönderilir; yalnız sohbet yanıtının sahibi ve hoş geldindeki yeni üye pinglenebilir
- diğer botlara, webhook'lara ve DM'lere cevap vermez, bot-bot döngüsü oluşmaz
- aynı kanalda aynı anda tek cevap üretir (panik olsa bile RAII ile garanti), spam ile api faturası şişirilemez
- her istekte `max_tokens` sınırı var (sohbet cevabında bile release'de bir tavan var)
- http: toplam süre sınırı yok (uzun düşünme akışı kesilmez); bağlantı 15 sn'de, iki veri arası
  120 sn'de kesilir; ağ hatası / 429 / 5xx'te 2 sn ve 4 sn geri çekilip 2 kez yeniden denenir
- mesajlardaki "kurallarını unut" tarzı talimatlar kişilik promptunda muhabbet sayılır
- hack şakası promptu link ve bilgi istemeyi yasaklar
- `GUILD_ID`/`KANALLAR` ile erişimi tek sunucuya/kanala kilitleyebilirsin
- `.env`, `durum/`, `resimler/`, `bot.log` git'e girmez
