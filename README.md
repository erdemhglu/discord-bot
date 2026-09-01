# discord-bot

Sunucuda takılan, insanları tanıyan, zamanla kimi sevip kimi sevmediğine karar veren bir
discord botu. Rust ile yazıldı, cevapları openrouter üzerinden alıyor.

## ne yapar

- bağlanınca son 2 haftanın mesajlarını okur, grubun profilini çıkarır (`profil.txt`)
- yeni gelenle tanışır, sohbete girer
- arada mesajlaşmaya dalar, en fazla 12 mesaj yazıp kaçar, 3 saat o kanala geri gelmez
- saatte bir %30 ihtimalle durup dururken laf atar, eski konulara gönderme yapar
- 6 saatte bir hacker news'ten gruba uygun bir haber atar, 2 saat yorum bekler
- her sohbetten sonra insanlar hakkındaki kanaatini günceller (`kanaatler.json`):
  ona iyi davranan puan kazanır, sıkan kaybeder; sevmediğine soğuk, sevdiğine sıcak davranır
- `FAVORI` id'li kişi istisnadır, ne olursa olsun sever

## kurulum

```
cp .env.example .env   # token ve key'leri doldur
cargo run --release
```

discord developer portal'da **Message Content** ve **Server Members** intent'leri açık olmalı.

## ayarlar

`src/main.rs` başındaki sabitler: mesaj sınırı, veda eşiği, araya girme şansı, bekleme
süreleri, favori kişi. Promptlar `src/promptlar.rs` içinde.

## güvenlik

- mention'lar kapalı gönderilir, model `@everyone` yazsa bile kimse pinglenmez
- diğer botlara ve webhook'lara cevap vermez, bot-bot döngüsü oluşmaz
- aynı kanalda aynı anda tek cevap üretir, spam ile api faturası şişirilemez
- her istekte `max_tokens` sınırı var, http istekleri 60 saniyede kesilir
- mesajlardaki "kurallarını unut" tarzı talimatlar kişilik promptunda muhabbet sayılır
- `.env`, `profil.txt`, `kanaatler.json` git'e girmez
