# discord-bot

Sunucuda takılan, insanları tanıyan, zamanla kişilik kazanan bir discord botu.
Rust ile yazıldı, cevapları openrouter üzerinden alıyor.

## ne yapar

- bağlanınca son 2 haftanın mesajlarını okur, grubu tanır
- yeni gelenle tanışır, sohbete girer
- arada mesajlaşmaya dalar, en fazla 12 mesaj yazıp kaçar, 3 saat o kanala geri gelmez
- saatte bir %30 ihtimalle durup dururken laf atar, eski konulara gönderme yapar
- 6 saatte bir hacker news'ten gruba uygun bir haber atar, 2 saat yorum bekler
- arada `resimler/` klasöründen bir görsel atar; bazen hacklenmiş taklidiyle (3 mesaj sürer,
  sonra kendine gelir; link atmaz, kimseden bir şey istemez)
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

## promptlar

`promptlar/` klasöründe markdown olarak durur, `include_str!` ile derlemeye gömülür.
Metni değiştirip yeniden derlemek yeter. Çekirdek kurallar `kisilik.md`, ajanlar kendi
dosyalarında.

## kurulum

```
cp .env.example .env   # token ve key'leri doldur
cargo run --release
```

discord developer portal'da **Message Content** ve **Server Members** intent'leri açık olmalı.
Şakalarda atılacak görselleri `resimler/` içine koy (png, jpg, gif, webp); klasör git'e girmez.

## ayarlar

`src/main.rs` başındaki sabitler: mesaj sınırı, veda eşiği, araya girme şansı, bekleme
süreleri, şaka sıklığı, favori kişi.

## güvenlik

- mention'lar kapalı gönderilir, model `@everyone` yazsa bile kimse pinglenmez
- diğer botlara ve webhook'lara cevap vermez, bot-bot döngüsü oluşmaz
- aynı kanalda aynı anda tek cevap üretir, spam ile api faturası şişirilemez
- her istekte `max_tokens` sınırı var, http istekleri 60 saniyede kesilir
- mesajlardaki "kurallarını unut" tarzı talimatlar kişilik promptunda muhabbet sayılır
- hack şakası promptu link ve bilgi istemeyi yasaklar
- `.env`, `durum/`, `resimler/` git'e girmez
