# İlerleme günlüğü

Kronolojik. En yeni üstte. Her satır: tarih · commit (varsa) · ne+neden · doğrulama.

---

## 2026-09-02 · Adım 4 · cevap istekliliği
- Sabit zar (`SANS × evre`) kalktı: etiket/yanıt/ad her zaman cevaplanır, diğer mesajlar için
  mini model çağrısı (`promptlar/isteklilik.md`, ~80 token): son 12 mesaj + profil + dizin →
  `{"puan":0-10,"sebep"}`; eşik `ISTEK_ESIGI`=6, evre cesareti ±1, seyahatte +2.
- Rate limit: kanal başına en sık 2 dk'da bir çağrı (`Durum.son_degerlendirme`).
- Çağrı başarısızsa yedek zar (`SANS=0.35`). `YOLDA_SANS_CARPANI` kaldırıldı (seyahat etkisi
  eşik kaymasında).
- Doğrulama: 33 test (isteklilik_puan clamp/süs dahil), clippy 0 uyarı.

## 2026-09-02 · Adım 3 · zihin id bazlı + saniyeli zaman + bellek döngüsü
- Kişi dosyaları `kisiler/<id>.md`; `Kisi` alanları: id, kullanici_adi, eski_adlar + eskiler.
  Ad değişince eski ad `eski_adlar`'a düşer, hafıza bölünmez. Temiz başlangıç: eski slug
  dosyaları dizinde atlanır.
- `Durum.ad_id` (ad→id) ve `kullanici_adlari` (id→kullanıcı adı) her mesajda ve açılış
  taramasında beslenir; `gunlukcu` isimleri buradan id'ye çevirir, çözülemeyeni atlar+loglar.
- Tüm kayıtlar `tarih_saat()` ile saniyeli (olay/konu/kişi/arşiv/gündem).
- Bellek döngüsü: kapanan sohbetin dökümü ve 6 saatlik gözlem `bellek_kuyruk`'a düşer;
  `bellek_dongusu` (10 dk, uyku kontrolüne takılmaz) günlükçü+özetleyici (+biten sohbette
  eleştirmen) sırasıyla işler. Kuyruk 50'yi aşarsa en eski atılır (warn).
- Doğrulama: 32 test, clippy 0 uyarı.

## 2026-09-02 · Adım 1+2 · log sadeleştirme + 12 mesaj sınırı kalktı
- **Adım 1:** info logda yalnız kritik olaylar: uyudu/uyandı, PANİK/error, zihin kaydı
  (günlükçü), evre geçişi, açılış/kapanış. Ajan güncellemeleri, gezgin, mesaj taraması debug'a indi.
- **Adım 2:** `MAX_MESAJ`/`VEDA_ESIGI`/`BEKLEME` silindi; veda ve son-mesaj promptları kaldırıldı;
  kanal yasağı (`yasakli`/`girebilir_mi`) yok. Sohbet son mesajdan `SOHBET_ZAMAN_ASIMI` (30 dk)
  sonra sessizce kapanır: `Durum.son_aktivite` + dakika tikinde `zaman_asimi_kapat`
  (meşgul kanallara dokunmaz, kapanan döküm günlükçü+eleştirmene gider).
- Doğrulama: 31 test, clippy 0 uyarı.

## 2026-09-02 · Adım 0 · `dev/` klasörü kuruldu
- Oturum hafızası: `dev/README.md`, `dev/ilerleme.md`, `dev/yol-haritasi.md`.
- `AGENTS.md` ve `CLAUDE.md`'ye işaretçi eklendi (compact sonrası ilk okunacak yer).
- Amaç: context şişip compact olunca kaldığı yerden devam edebilmek.

## 2026-09-02 · b4ae7a0 · Gözlemlenebilirlik (Ajan 3)
- `log` + elle sink (`src/loglama.rs`, `LOG_SEVIYE` ortam değişkeni, varsayılan info).
- Panic hook: panikler backtrace ile log'a düşer (spawn'lı döngülerde sessiz ölüm azalır).
- 48 `println!/eprintln!` seviyeli makrolara çevrildi.
- Token kullanım metriği: `stream_options.include_usage`, `Kullanim`/`Metrik`, `!durum`'da gösterim.
- Akış özet logları (parça/ilk parça/toplam süre/done).
- Doğrulama: 31 test, clippy 0 uyarı, release build.

## 2026-09-02 · 01be248 · Düşünme arayüzü
- Gizle kipinde canlı kelime sayacı: "Düşünüyorum... Şu ana kadar N kelime düşündüm."
- Cevap sonunda "Düşünce Sürecini Göster" butonu → interaction_create → yalnız tıklayana ephemeral kod bloğu.
- Göster kipinde thinking hem spoiler hem kod bloğu.
- Discord Components (buton) kullanıldı; spoiler ile gerçek gizleme mümkün olmadığı için.

## 2026-09-02 · 2e5eb17 · Komut modülü + `!düşünme`
- Komutlar `src/komut.rs`'ye taşındı (`impl Bot`, `use super::*` geleneği).
- `!düşünme göster/gizle/aç/kapat` kipi (`durum/dusunme.md`'de kalıcı), `!yardım`/`!help`.
- Düşünürken "Düşünüyorum..." mesajı; kapalıyken istekler reasoning'siz (`reasoning_kapat`).
- Thinking'de newline yok (`tek_satir`).

## 2026-09-02 · b1665d8 · Sohbet cevapları stream
- Cevap tek seferde gelmez: ilk delta ile mesaj açılır, `AKIS_DUZENLEME` (1.2 sn) aralıkla düzenlenir.
- Thinking kırpılmadan spoiler'da (`reasoning` + `reasoning_content` alanları).
- 1900'ü aşan cevap cümle/boşluk sınırından yeni mesaja bölünür (`bol`), kırpma yok.
- `cevap_butcesi!()` makrosu: release'de `None` (bütçesiz), debug'da `Some(2000)`.
- `kisalt`/`cevap_olcusu` silindi; `API_ADRES` ile kendi router'ına yönlendirme.

## 2026-09-02 · 72b7f4a · Merge PR #1 (Speretta/main)
- krxi/discord-bot'a fork'tan merge. Geçmiş tek hatta birleşti.

## Analiz raporu (5 ajan) — özet
Beş paralel ajan kodu taradı; raporların özü `yol-haritasi.md`'deki risk listesinde.
Ana bulgular: global 60 sn timeout stream'i kesebilir (P0), mesgul panic'te sızar,
dosya yazımları atomik değil + ajanlar arası yarış, döngüler panikte sessiz ölür,
kişi anahtarı görünen ad (id değil).

---

## Not — doğrulama komutları
```
cargo fmt && cargo clippy --all-targets && cargo test && cargo build --release
```
`AGENTS.md` kuralı: clippy 0 uyarı beklenir. Tanımlayıcılar Türkçe ama ASCII (`dusunce`, `kisalt`).
