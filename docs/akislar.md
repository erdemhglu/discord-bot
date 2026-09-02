# Akışlar (olay → sırayla ne olur)

## Bir mesaj geldi
0. Her mesaj (bot dahil, `gonder` üstünden) kanalın geçmişine düşer: `kanal_not` → bellek (60 satır) + `durum/kanallar/<id>.md`. Yeni sohbet açılırken son 10 satır tohum olur (`sohbet_baslat`), böylece sohbet bitmiş ya da bot yeniden başlamış olsa da bağlam kaybolmaz.
0. Metin `!` ya da `/` ile başlıyorsa `komut.rs::Bot::komut`: sifirla · haber · sorun · gez · saka · hack · ajanlar · uyan · uyu · durum · düşünme · model · yardım/help. Tanınan komut işlenir ve mesaj sohbete girmez; tanınmayan komut normal mesaj sayılır. `model <id>` yalnız FAVORI, OpenRouter listesinde doğrulanır. `düşünme göster/gizle/sessiz/kapat` düşünme kipini değiştirir (`durum/dusunme.md`): göster=spoiler'da, gizle="Düşünüyorum..." sonrası cevap, sessiz=arka planda düşünür ama hiç iz göstermez (placeholder/sayaç/buton yok), kapat=istekler reasoning'siz.
1. `Handler::message`: bot/webhook/DM → çık. `content_safe` (mention'lar `@ad`, `@everyone` zararsız).
2. Kilit içinde: etiketlendi mi? (mention listesi ∪ yanıtlanan mesaj botun ∪ metinde bot adı)
3. `hatirla` (ham hafıza), `son_kanal`, favori adı.
4. Haber attıysa ve 2 saat dolduysa o sohbeti sessizce kapat (yasak yok).
5. **Uyuyorsa:** etiketlendiyse kuyruğa (≤20), cevap yok, çık.
6. Etiketlendiyse ya da **sürmekte olan diyalog**sa (sohbet açık VE sohbetteki son user
   mesajının sahibi bu mesajı atanla aynıysa — yani gerçekten kendisiyle konuşuyor) doğrudan
   cevap. Değilse (sohbet yok, ya da kanalda BAŞKA biri yazdı) **isteklilik değerlendirmesi**:
   kanal başına en sık 2 dakikada bir mini model çağrısı (`isteklilik.md`, ~80 token) son 12
   mesaj + profil + dizin üzerinden `{"puan":0-10}` üretir; eşik (`ISTEK_ESIGI`, evre
   cesaretine göre ±1, seyahatte +2) üstündeyse sohbete girer. Çağrı başarısızsa yedek zar
   (`SANS`). Bu, açık bir sohbette kanaldaki HERKESE otomatik cevap vermeyi engeller — yalnız
   gerçek muhatabına.
7. Sohbet açıksa kullanıcı satırını geçmişe ekle (son 20).
8. Kilit dışı: `cevapla`.

## cevapla (bir sohbet turu, stream)
```
kilit ── meşgul? çık ── sohbet var? ── talimat seç ── meşgul=1 ── kilit bırak
bekle 0,15-0,35 sn ── güncel geçmiş + son mesaj + bekleyenler ── arastir(link/haber/araştır) ── hedef seçimi (2+ yazan varsa) ── yazıyor…
uret_akis(stream, bütçe: cevap_butcesi!; release'de bütçe yok) ── (hata: meşgul=0, çık)
gonder_akis: ilk delta ile mesaj açılır ── AKIS_DUZENLEME (1,2 sn) aralıkla düzenlenir ── düşünürken (cevap başlamadı): göster="Düşünüyorum...", gizle=canlı kelime sayacı, sessiz/kapalı=hiçbir şey (mesaj cevap başlayana dek hiç açılmaz) ── cevap başlayınca aynı mesaj düzenlenerek stream ── göster: thinking newline'sız tek satır, hem spoiler hem kod bloğu ── gizle: thinking mesajda yok, cevap sonunda "Düşünce Sürecini Göster" butonu (interaction_create tıklayana ephemeral kod bloğu açar, düşünce deposu 50 mesaj) ── sessiz: reasoning isteniyor (arka planda çalışıyor) ama hiç toplanmıyor/gösterilmiyor, buton da yok ── kapalı: istek reasoning'siz ── 1900'ü aşan parça yeni mesaj ── discord yanıtı her cevapta ilk mesajda
tekrar_mi? bir kez yeniden üret, yine tekrarsa açılanları sil ve sus
üst üste 2+ farklı kişi yazdıysa HEDEF_SEC mini çağrısı hedef kişiyi seçer; yanıt o kişinin mesajına bağlanır, talimata "ona seslen" notu girer
üretim sırasında yeni mesaj gelse de akış tamamlanır (sil-baştan yok); yeni mesaj sıradaki turda ele alınır
stream hiçbir şey üretmediyse uret ile stream'siz yedek
… bitti değilse: yeni mesaj yoksa çık
kilit ── meşgul=0 ── asistan satırı ekle (yalnız cevap, thinking değil) ── sayac++ ── hackli-- ── sayac≥12 → sohbet_bitir ── kilit bırak
```
Talimat önceliği: hack devam > hack çıkış > boş.

## Sohbet yaşam döngüsü
- Başlangıç kaynakları: rastgele araya girme, etiket, hoş geldin, haber paylaşımı, dürtme, şaka, uyanınca dönüş, yoldan mesaj, gidiyorum duyurusu. Açılışlı olanlar `sayac=1` ile başlar.
- Mesaj sınırı ve veda yok: sohbet son mesajdan 30 dk sonra sessizce kapanır (dakika tikinde `zaman_asimi_kapat`), kanal yasağı yok. Kapanan sohbetin dökümü günlükçüye ve eleştirmene gider.
- Yasak yalnız *araya girmeyi* engeller; etiket her zaman cevap alır.
- Model çağrısı hata verirse sayaç ilerlemez, sohbet açık kalır.

## uret (her kişilikli çağrı)
1. Geçmişteki `user` satırlarından `isim` (": " öncesi) ve metin ayrıştırılır.
2. `anahtarlar(metinler)` → ≤40 kelime.
3. Kilit: `getir(katilimcilar, ad_id, anahtar, ham hafıza, 20)` → bütçeli bağlam; `sistem_metni`.
4. `sor` → `temizle` (ad öneki, tırnak, 1900).
Sohbet cevapları bunu kullanmaz; `uret_akis` aynı sistemi kurup stream açar (`gonder_akis` yazar), kırpma yoktur.

## Komut arayüzü (slash → embed kart → detay modalı)
`ready` → her sunucuya `/durum` `/yardim` `/zihin` kaydı (idempotent) → kullanıcı slash çalıştırır →
`interaction_create(Command)` → ephemeral **embed kart** (`durum_mesaji`/`yardim_mesaji`/`zihin_mesaji`),
yalnız çağırana görünür.
`/zihin` kartı: üç sütun (Kişiler/Konular/Olaylar) + üstte kişi select menüsü, altta Konular/Olaylar/Bot özeti butonları.
Menüden kişi seç ya da butona bas → `interaction_create(Component)` → ilgili **detay modalı**
(`modal_kisi` / `modal_konular` / `modal_olaylar` / `modal_ozet`); her bölüm kendi etiketli alanında, tek kutuya boca yok.
Kullanıcı modal'ı gönderirse → `interaction_create(Modal)` → kısa ephemeral onay; girdi toplanmaz.
Paralel düz metin: `!durum` ortak `durum_metni`; `!zihin` aynı embed kartını kanala yollar + `/zihin` yönlendirmesi; `!yardım` aynen.

## Sunucuya bağlanınca
`guild_create` (sunucu başına bir kez) → arka planda: 14 gün geriye tarama (izinli kanallar, 100'lük sayfalar) → ham hafıza son 2000 → profilci → hoca (huy boşsa). Yeniden bağlanmada tekrar taranmaz.

## 6 saatlik tur (haber_dongusu)
uyanık değil → geç · seyahatte → profilci, hoca, geç · profilci → gunlukcu("gözlem", son 300) → hoca → kanalda sohbet açıksa geç → haberci (HN 12 + Sözcü 12, atılmamışlar) → seçim → tanıtım (`uret`) → gönder → sohbet aç, 2 saat yorum bekle, haberi "atıldı" say.

## Dürtme (saatte bir)
%25 (`SORUN_PAYI`): varsayılan kanala `sorun_at` (uydurma kod derdi + soru), sohbet açılır. Aksi halde aşağıdaki akış.
uyanık değil → geç · seyahatte: bugün yazdıysa geç, %25 → `YOLDA` · yarın seyahat: bir kez `GIDIYORUM` · değilse %30 → `DURUP_DURURKEN` · `bos_kanal` yoksa geç → `uret(son 40 satır)` → gönder → sohbet aç.

## Şaka (3 saatte bir, %10)
uyanık ∧ seyahatte değil ∧ boş kanal ∧ `resimler/` dolu → %30 hack (`HACK_GIRIS` metni + görsel, sohbet `hackli=3`: 2 tur `HACK_DEVAM`, 1 tur `HACK_CIKIS`) · %70 düz görsel (`resimci`: model görseli görür).

## Gündem gezintisi (10 dk sonra, sonra 4 saatte bir)
rss 20 → seçim (`GEZGIN_SEC`, huy+profil) → ≤3 sayfa (`sayfa_oku`: firecrawl ya da düz) → `uret(GEZGIN_NOT)` botun kendi günlüğü → `gundem.md` (12 giriş, eskisi arşiv) → `Durum.gundem` son 3 → her cevabın "GÜNDEM" bölümü ve hoca girdisi.

## Uyku (dakikada bir)
`!uyan`: aktif planın bitişine kadar `uyanik_zorla` (planı silmek işe yaramaz, dakika sonra yeniden kurulup uyutur). `!uyu [saat]`: geçici plan, zorlama sıfırlanır.
`guncelle`: dün+bugün için plan yoksa kur (gergin ise %20, değilse %7 uykusuz gece). Uyanık→uyudu / uyudu→uyandı geçişi loglanır. Uyku hali konuşma promptuna karakter bahanesi olarak girmez.
**Uyurken dinleme sürer:** mesajlar ham hafızaya girer; `bellek_dongusu` 2 saatte bir gece gözlemi yapıp zihne işler; haber turu uyurken haber seçer ama atmaz, `stok_haber`'e koyar.
**Uyanınca:** bekleyen etiket varsa `UYANDIM` ile kesin dönüş (hata durumunda liste geri konur, kaybolmaz). Etiket yoksa `uyanis.md` ajanı gece mesajlarını değerlendirir (`{"ilgi":0-10,"konu"}`); ilgi ≥5 ise `uyanis-cevap.md` ile son konuşulan kanala sabah sözü. Stok haber uyanık ilk turda "sabah haberi" olarak atılır.

## Seyahat (takvimden)
`seyahat::simdi()` bugünü tabloya bakarak bulur. Etkisi: "ŞU AN" satırı, araya girme ×0.3, haber/şaka yok, dürtme yerine günde ≤1 yoldan mesaj, bir gün önce `GIDIYORUM`. Durum tutulmaz; yalnız `son_yol_mesaji` ve `duyurulan_seyahat` işaretleri.

## Gelişim
Her biten sohbet `gelisim.sohbet++`, her mesaj `gelisim.mesaj++`. `gelisim_kontrol`: gün ve sohbet
eşiklerine göre evre yalnız ileri atlar (yeni → isinma → yerlesik → eski-toprak). Evre: sistem
mesajında "GELİŞİM EVREN" bölümü, araya girme şansı × evre.sans, dürtme × evre.durtme. Yerleşik
evresine girince bir kez isim seçer: model tek kelime verir, takma ad her sunucuda değişir,
`bot_adi` olur, gruba duyurulur. Sayaçlar `durum/gelisim.md`'de; yeniden başlatma sıfırlamaz.

## Biten sohbet → hafıza
Sohbet 30 dk sessiz kalınca `zaman_asimi_kapat` kapatır; döküm `Durum.bellek_kuyruk`'a düşer.
`bellek_dongusu` (10 dk'da bir, uykudan bağımsız) kuyruğu işler: `gunlukcu` JSON → `olaylar/AA.md`
satırı (saniyeli), kişi dosyaları id bazlı (isim `ad_id` ile çevrilir; puan, not, bilgiler,
etiket, olay), konu dosyaları, `kendim.md`, `INDEX.md` → `ozetleyici` sınır aşanları küçültür
(arşivle) → biten sohbette ayrıca `elestirmen` → `duzeltmeler.md`. 6 saatlik turun gözlemi de
aynı kuyruktan geçer.
