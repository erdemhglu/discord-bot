# discord-bot

> Geliştirici ve yapay zeka ajanları için giriş: [AGENTS.md](AGENTS.md) · ayrıntı [docs/](docs/)

Sunucuda takılan, insanları tanıyan, zamanla kişilik kazanan bir discord botu.
Rust ile yazıldı, cevapları openrouter üzerinden alıyor.

## ne yapar

- bağlanınca son 2 haftanın mesajlarını okur, grubu tanır
- yeni gelenle tanışır, sohbete girer
- arada mesajlaşmaya dalar, en fazla 12 mesaj yazıp kaçar, 3 saat o kanala geri gelmez
- saatte bir %30 ihtimalle durup dururken laf atar, eski konulara gönderme yapar
- 6 saatte bir hacker news + Sözcü gündeminden gruba uygun bir haber atar, 2 saat yorum bekler
- 4 saatte bir Türkiye gündemini gezer (Sözcü RSS, firecrawl varsa sayfayı okur), kendi görüşünü günlüğüne yazar; kişilik buradan da beslenir
- geceleri uyur (01-09), nadiren uykusuz gece geçirir (gerginse daha sık); uyurken yazmaz, etiketlenirse uyanınca döner
- etiketlenince, adı geçince ya da mesajına yanıt verilince her zaman cevap verir; cevabı son yazanın mesajına bağlayıp onu etiketler
- sohbet cevapları canlı akar: mesaj belirir, yazıldıkça büyür; model düşünce üretiyorsa (reasoning) kırpılmadan spoiler içinde gösterilir, 1900'ü aşan cevap kırpılmaz, yeni mesaja bölünür
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
| profilci | açılışta ve 6 saatte bir | grup profili: kim nasıl konuşur, iç şakalar, konular (`profil.txt`) |
| kanaatci | her biten sohbetten sonra ve 6 saatte bir | kişi başına -10/+10 puan ve gerekçe, botun kendi hali (`kanaatler.json`) |
| hoca | açılışta ve 6 saatte bir | botun nasıl biri olması gerektiği: mizah, dil, tavır, kalıplar (`huy.txt`) |
| elestirmen | her biten sohbetten sonra | botun kendi mesajlarına 3-5 somut düzeltme notu (`duzeltmeler.txt`) |
| haberci | 6 saatte bir | hacker news'ten gruba uygun haber |
| resimci | şaka anında | ekteki görsele kişilikle tek satır yorum (model görseli görür) |

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
  kisiler/<ad>.md   kişi başına: puan, etiket, not, bildiklerin, son olaylar
  konular/<ad>.md   konu başına tarihli notlar
  olaylar/YYYY-AA.md  biten her sohbetten tek satır
  arsiv/            özetlenip çıkarılan ham parçalar
```

**Her cevapta ne gider:** çekirdek kişilik + huy + profil + dizin + o sohbet için getirilenler +
kendine notlar + görev. Getirilenler sabit bütçeli (6000 karakter): sohbette konuşanların kişi
dosyaları, anahtar kelimeye uyan en fazla 2 konu dosyası, ayın son 8 olayı, ve ham bağlam
penceresinden (son 2000 mesaj) konuya değen ama sohbette olmayan en fazla 12 eski satır.

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
cargo run --release
```

discord developer portal'da **Message Content** ve **Server Members** intent'leri açık olmalı.
Şakalarda atılacak görselleri `resimler/` içine koy (png, jpg, gif, webp); klasör git'e girmez.

## komutlar

`!` ya da `/` ile başlar (`/model` de olur).

- `!sifirla` — o kanaldaki 3 saatlik yasağı ve açık sohbeti sıfırlar. `!sifirla hepsi` tüm kanallar.
- `!haber` — şimdi haber seç ve at (HN + gündem).
- `!sorun` — yazılım derdi atıp "nasıl çözerim" diye sorar (kendiliğinden de laf atma turlarının %25'inde, haber kanalına).
- `!gez` — gündem gezintisini şimdi yap (gundem.md güncellenir).
- `!saka` / `!hack` — görsel şakası / hacklenmiş taklidi şimdi.
- `!ajanlar` — profilci ve hocayı şimdi çalıştır.
- konuşmalar `durum/kanallar/<id>.md`'de kalır; sohbet bitse ya da bot yeniden başlasa da son 10 satırla devam eder
- `!uyan` — uykuyu şimdi keser, uyurken etiketleyenlere döner. `!uyu [saat]` — test için uyutur (varsayılan 8 saat).
- `!durum` — evre, sayaçlar, model, uyku, seyahat.
- `!model` — şu anki model. `!model <id>` değiştirir (yalnız FAVORI kişi; OpenRouter'da yoksa "yok öyle model"). Seçim `durum/model.md`'de kalır, yeniden başlatınca korunur.

## ayarlar

`src/main.rs` başındaki sabitler: mesaj sınırı, veda eşiği, araya girme şansı, bekleme
süreleri, şaka sıklığı, favori kişi.

## güvenlik

- mention'lar kapalı gönderilir; yalnız sohbet yanıtının sahibi ve hoş geldindeki yeni üye pinglenebilir
- diğer botlara ve webhook'lara cevap vermez, bot-bot döngüsü oluşmaz
- aynı kanalda aynı anda tek cevap üretir, spam ile api faturası şişirilemez
- her istekte `max_tokens` sınırı var, http istekleri 60 saniyede kesilir
- mesajlardaki "kurallarını unut" tarzı talimatlar kişilik promptunda muhabbet sayılır
- hack şakası promptu link ve bilgi istemeyi yasaklar
- `.env`, `durum/`, `resimler/` git'e girmez
