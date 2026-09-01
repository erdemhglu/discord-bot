# Akışlar (olay → sırayla ne olur)

## Bir mesaj geldi
0. Her mesaj (bot dahil, `gonder` üstünden) kanalın geçmişine düşer: `kanal_not` → bellek (60 satır) + `durum/kanallar/<id>.md`. Yeni sohbet açılırken son 10 satır tohum olur (`sohbet_baslat`), böylece sohbet bitmiş ya da bot yeniden başlamış olsa da bağlam kaybolmaz.
0. Metin `!` ya da `/` ile başlıyorsa `Bot::komut`: sifirla · haber · sorun · gez · saka · hack · ajanlar · uyan · uyu · durum · model. Tanınan komut işlenir ve mesaj sohbete girmez; tanınmayan komut normal mesaj sayılır. `model <id>` yalnız FAVORI, OpenRouter listesinde doğrulanır.
1. `Handler::message`: bot/webhook/DM → çık. `content_safe` (mention'lar `@ad`, `@everyone` zararsız).
2. Kilit içinde: etiketlendi mi? (mention listesi ∪ yanıtlanan mesaj botun ∪ metinde bot adı)
3. `hatirla` (ham hafıza), `son_kanal`, favori adı.
4. Haber attıysa ve 2 saat dolduysa o sohbeti sessizce kapat (yasak yok).
5. **Uyuyorsa:** etiketlendiyse kuyruğa (≤20), cevap yok, çık.
6. Sohbet açık değilse: etiketlendi → aç (yasak olsa da); değilse yasaklı değil ∧ rastgele < SANS (seyahatte ×0.3) → aç.
7. Sohbet açıksa kullanıcı satırını geçmişe ekle (son 20).
8. Kilit dışı: `cevapla`.

## cevapla (bir sohbet turu)
```
kilit ── meşgul? çık ── sohbet var? ── talimat seç ── meşgul=1 ── kilit bırak
bekle 0,15-0,35 sn ── güncel geçmiş + son mesajı al ── arastir(link/haber/araştır) ── yazıyor… ── uret(70/100/140 token) ── kisalt(1/2/3 cümle) ── tekrar_mi? bir kez yeniden üret, yine tekrarsa sus
üretim sırasında yeni mesaj geldiyse eski cevabı göndermeden başa dön; gelmediyse hemen son kullanıcı mesajına Discord yanıtı olarak gonder ve kişiyi etiketle
… bitti değilse: yeni mesaj yoksa çık
kilit ── meşgul=0 ── asistan satırı ekle ── sayac++ ── hackli-- ── sayac≥12 → sohbet_bitir ── kilit bırak
bitti ise: gunlukcu → ozetleyici → elestirmen
```
Talimat önceliği: hack devam > hack çıkış > son mesaj (sayac ≥ 11) > veda (sayac ≥ 9) > boş.

## Sohbet yaşam döngüsü
- Başlangıç kaynakları: rastgele araya girme, etiket, hoş geldin, haber paylaşımı, dürtme, şaka, uyanınca dönüş, yoldan mesaj, gidiyorum duyurusu. Açılışlı olanlar `sayac=1` ile başlar.
- 9. bottan sonra "toparla", 12.'de "vedalaş"; 12'ye ulaşınca sohbet silinir, kanal 3 saat yasaklı.
- Yasak yalnız *araya girmeyi* engeller; etiket her zaman cevap alır.
- Model çağrısı hata verirse sayaç ilerlemez, sohbet açık kalır.

## uret (her kişilikli çağrı)
1. Geçmişteki `user` satırlarından `isim` (": " öncesi) ve metin ayrıştırılır.
2. `anahtarlar(metinler)` → ≤40 kelime.
3. Kilit: `getir(katilimcilar, anahtar, ham hafıza, 20)` → bütçeli bağlam; `sistem_metni`.
4. `sor` → `temizle` (ad öneki, tırnak, 1900).

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
`guncelle`: dün+bugün için plan yoksa kur (gergin ise %20, değilse %7 uykusuz gece). Uyanık→uyudu / uyudu→uyandı geçişi loglanır. Uyanınca bekleyen etiket varsa son etiketin kanalına `UYANDIM` ile tek mesaj + sohbet. Uyurken: cevap yok, döngüler geçer, hafıza kaydı devam eder. Uyku hali konuşma promptuna karakter bahanesi olarak girmez.

## Seyahat (takvimden)
`seyahat::simdi()` bugünü tabloya bakarak bulur. Etkisi: "ŞU AN" satırı, araya girme ×0.3, haber/şaka yok, dürtme yerine günde ≤1 yoldan mesaj, bir gün önce `GIDIYORUM`. Durum tutulmaz; yalnız `son_yol_mesaji` ve `duyurulan_seyahat` işaretleri.

## Gelişim
Her biten sohbet `gelisim.sohbet++`, her mesaj `gelisim.mesaj++`. `gelisim_kontrol`: gün ve sohbet
eşiklerine göre evre yalnız ileri atlar (yeni → isinma → yerlesik → eski-toprak). Evre: sistem
mesajında "GELİŞİM EVREN" bölümü, araya girme şansı × evre.sans, dürtme × evre.durtme. Yerleşik
evresine girince bir kez isim seçer: model tek kelime verir, takma ad her sunucuda değişir,
`bot_adi` olur, gruba duyurulur. Sayaçlar `durum/gelisim.md`'de; yeniden başlatma sıfırlamaz.

## Biten sohbet → hafıza
`gunlukcu`: JSON → `olaylar/AA.md` satırı, kişi dosyaları (puan, not, bilgiler, etiket, olay), konu dosyaları, `kendim.md`, `INDEX.md` → `ozetleyici` sınır aşanları küçültür (arşivle) → `elestirmen` → `duzeltmeler.md`.
