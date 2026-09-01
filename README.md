# discord-bot

Sunucuda takılan, insanları tanıyan bir discord botu. Go ile yazıldı,
cevapları openrouter üzerinden alıyor.

## ne yapar

- açılışta son 2 haftanın mesajlarını okur, grubun profilini çıkarır (`profil.txt`)
- yeni gelenle tanışır, sohbete girer
- arada mesajlaşmaya dalar, en fazla 12 mesaj yazıp kaçar, 3 saat geri gelmez
- saatte bir %30 ihtimalle durup dururken laf atar, eski konulara gönderme yapar
- 6 saatte bir hacker news'ten gruba uygun bir haber atar, 2 saat yorum bekler

## kurulum

```
cp .env.example .env   # token ve key'leri doldur
go run .
```

discord developer portal'da **Message Content** ve **Server Members** intent'leri açık olmalı.

## ayarlar

`main.go` başındaki sabitler: mesaj sınırı, veda eşiği, araya girme şansı, bekleme süreleri.
