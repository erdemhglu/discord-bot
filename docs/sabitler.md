# Sabitler

## src/main.rs
| Sabit | Değer | Anlam |
|---|---|---|
| SURUM / SURUM_COMMIT / SURUM_TARIH | Cargo.toml sürümü / build.rs'in git'ten aldığı commit (+ = derlemede commit'lenmemiş değişiklik vardı) / derleme tarihi | `surum_metni()`: /durum ve açılış duyurusu |
| OPENROUTER_ADRES / OPENROUTER_MODEL | …/api/v1/chat/completions / openai/gpt-4o-mini | varsayılan sağlayıcı |
| MISTRAL_ADRES / MISTRAL_MODEL | api.mistral.ai/v1/chat/completions / mistral-medium-latest | MISTRAL_KEY varsa ya da SAGLAYICI=mistral |
| SANS | 0.35 | yedek zar: isteklilik çağrısı başarısızsa araya girme olasılığı |
| ISTEK_ESIGI / DEGERLENDIRME_ARALIGI | 6 / 2 dk | isteklilik puan eşiği / kanal başına en sık değerlendirme |
| SOHBET_ZAMAN_ASIMI | 30 dk | bu kadar sessiz kalan sohbet vedasız kapanır |
| YORUM_SURESI | 2 saat | haber attıktan sonra yorum bekleme |
| HABER_ARALIGI | 6 saat | haber turu ve 6 saatlik ajanlar |
| DURTME_ARALIGI / DURTME_SANSI | 1 saat / 0.3 | kendiliğinden laf atma |
| SAKA_ARALIGI / SAKA_SANSI / HACK_PAYI / HACK_MESAJI | 3 saat / 0.1 / 0.3 / 3 | görsel ve hack şakası |
| SORUN_PAYI | 0.25 | laf atma turlarının kod derdi olma payı |
| KANAL_GECMIS / SOHBET_TOHUM | 60 / 10 | kanal başına saklanan satır / yeni sohbete tohum |
| GECMIS_GUN | 14 | açılış taramasının derinliği |
| HAFIZA_BOYU | 2000 | ham hafıza satırı |
| SOHBET_BOYU | 20 | modele giden sohbet geçmişi |
| MESAJ_SINIRI | 1900 | Discord 2000 sınırına pay |
| AKIS_DUZENLEME | 1200 ms | stream'de iki düzenleme arası asgari süre (Discord edit sınırı) |
| PATLAMA_SINIRI | 4 | bir turda en çok kaç satır (= ayrı mesaj) gider; fazlası düşer. Ölçüye dayanıyor: gerçek IM'de bir kişinin peş peşe mesaj dizisi ortalama 1.7 mesaj, dizilerin %42'si çok-mesajlı (Baron 2010) — "her cevabı üçe böl" yanlış olur, 4 tavandır, hedef değil |
| YARIM_SATIR_ESIGI | 12 | akış sürerken son (henüz `\n` görmemiş) satır bu kadar karakteri geçmediyse gösterilmez; "tep" yarım hâlde mesaj olup bir sonraki edit'te silinmesin |
| SATIR_GECIKME_TABAN / _HARF / _TAVAN | 300 ms / 15 ms per karakter / 1500 ms | `gonder_satirlar` (stream OLMAYAN yollar) satırlar arası bekleme + typing. Stream yolunda gecikme YOKTUR (akışın kendi temposu yeter). **Bu üç değer ölçülmedi**, insan yazma hızından kabaca seçildi; canlıda ayarlanmak isteyebilir |
| BAGLANTI_ZAMAN_ASIMI / OKUMA_ZAMAN_ASIMI | 15 sn / 120 sn | http: el sıkışma / iki veri arası (ilk tokeni kapsar). Toplam süre sınırı yok, uzun düşünme akışı kesilmez |
| AI_YENIDEN_DENEME | 2 | ağ hatası / 429 / 5xx'te ek deneme sayısı (toplam bu + 1) |
| `cevap_butcesi!()` (makro) / CEVAP_TAVANI | debug `Some(2000)` / release `Some(4096)` | sohbet cevabı token bütçesi; ikisinde de üst sınır var, release'de yalnız tekrar/döngü gibi kaçak durumları keser |
| REASONING_BUTCE_TABANI | 1500 | stream'siz ajan çağrısında reasoning kapatılamayınca yeniden deneme bütçesi: max(2×mevcut, bu) |
| FAVORI | 259669117248864257 | her zaman sevilen kullanıcı id |
| GEZGIN_ARALIGI | 4 saat | gündem gezintisi |
| RESIM_KLASORU / DURUM_KLASORU | resimler / durum | klasörler (çalışma dizinine göre) |

## src/hafiza.rs
KISI_SINIRI 1800 · KISI_HEDEF 1000 · KONU_SINIRI 1500 · KONU_HEDEF 800 · OLAY_SINIRI 6000 ·
BAGLAM_BUTCESI 6000 · DIZIN_KISI 40 · FAVORI_NOTU · DURAK (elenen kelimeler)

## src/gundem.rs
RSS_ADRESI (Sözcü) · GUNDEM_KAYIT 12 · SAYFA_SINIRI 3500

## src/uyku.rs
SAAT_FARKI +3 saat (TR, yaz saati yok) · UYKUSUZLUK_SANSI 0.07 · UYKUSUZLUK_GERGIN 0.20 ·
normal uyku 01:00→09:00 ±45 dk · uykusuz gece 01:00 ayakta, 06:00→13:00 ±45 dk

## src/seyahat.rs
ETKINLIKLER tablosu (yılbaşı 30 Ara 4g, sömestr 24 Oca 7g, ramazan
bayramı 2026: 19 Mar 4g / 2027: 8 Mar, 23 Nisan 3g, 19 Mayıs 3g, kurban 2026: 26 May 5g / 2027:
15 May, yaz 14 Tem 6g, zeytinli rock 21 Ağu 4g, 30 Ağustos 3g, 29 Ekim 3g)

## Kodda gömülü sayılar (sabit olmayan)
`gonder` kendi mesaj tamponu 50 · `bekleyen_etiketler` 20 · `getir` kişi ≤4/1200, konu ≤2/800,
olay 8, ham satır 12/200, anahtar ≤40, ≥2 eşleşme · `gecmisi_oku` sayfa 100 · haberci HN 12 +
RSS 12 · gezgin rss 20, sayfa ≤3 · yoldan mesaj günde 1, %25 · hoca son 200 satır · profilci 600 ·
gözlem 300 · hack giriş max_tokens 150

## `durum/taranan.md`
`gecmisi_oku` (14 günlük geçmiş taraması) daha önce taranmış sunucu id'lerini burada tutar;
her yeniden başlangıçta `Durum::yukle` okur, `guild_create`'te güncellenir. Yoksa her süreç
yeniden başlayışında her sunucunun tüm kanalları baştan taranırdı (API'ye ve zamana yazık).

## Ortam değişkenleri (.env)
DISCORD_TOKEN (discord'a bağlanmak için zorunlu; `cargo run -- sohbet` istemez) ·
OPENROUTER_KEY veya MISTRAL_KEY (biri zorunlu, CLI sohbet modunda da; ikisi de varsa openrouter) ·
SAGLAYICI=mistral (zorlama) · MODEL (model kimliği, sağlayıcının varsayılanını ezer) ·
API_ADRES (openai uyumlu chat/completions adresi; seçilen sağlayıcının adresini ezer) ·
FIRECRAWL_KEY (yoksa düz indirme) · HABER_KANALI (kanal id; yoksa sistem kanalı / ilk metin kanalı) ·
GUILD_ID (tek sunucu id; ayarlıysa bot yalnız bu sunucuda çalışır) ·
KANALLAR (virgüllü kanal id listesi; ayarlıysa bot yalnız bu kanallarda çalışır) ·
DEBUG_KANALI (kanal id; `/debug` açıkken karar izleri buraya, yoksa mesajın kanalına) ·
RESIM_ANALIZI (varsayılan açık; `kapali/off/hayir/0` ekli fotoğrafları okumayı kapatır — yalnız
açılışta okunur, hiçbir slash komutla çalışırken değiştirilemez, `Bot.resim_analizi`) ·
LOG_SEVIYE (error/warn/info/debug/trace, varsayılan info) · LOG_RENK (on/off; varsayılan: terminalde açık, dosyada kapalı)

## src/gelisim.rs
ISIM_EVRESI 2 (yerlesik) · EVRELER: yeni (0 gün, 0 sohbet, sans×0.7, dürtme×0.4) · isinma (3g, 8s, ×0.8, ×0.7) ·
yerlesik (10g, 25s, ×1, ×1) · eski-toprak (30g, 80s, ×1, ×1.2)
