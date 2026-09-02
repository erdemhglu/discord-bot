# Akışlar (olay → sırayla ne olur)

## Bir mesaj geldi
0. Her mesaj (bot dahil, `gonder` üstünden) kanalın geçmişine düşer: `kanal_not` → bellek (60 satır) + `durum/kanallar/<id>.md`. Yeni sohbet açılırken son 10 satır tohum olur (`sohbet_baslat`), böylece sohbet bitmiş ya da bot yeniden başlamış olsa da bağlam kaybolmaz.
0. **Ham** metin (resim işareti eklenmeden önceki `content_safe` çıktısı) `!` ya da `/` ile başlıyorsa `komut.rs::Bot::komut`: sifirla · haber · sorun · gez · saka · hack · ajanlar · uyan · uyu · durum · düşünme · model · yardım/help. Tanınan komut işlenir ve mesaj sohbete girmez; tanınmayan komut normal mesaj sayılır. `model <id>` yalnız FAVORI, OpenRouter listesinde doğrulanır. `düşünme göster/gizle/sessiz/kapat` düşünme kipini değiştirir (`durum/dusunme.md`): göster=spoiler'da, gizle="Düşünüyorum..." sonrası cevap, sessiz=arka planda düşünür ama hiç iz göstermez (placeholder/sayaç/buton yok), kapat=istekler reasoning'siz.
1. `Handler::message`: bot/webhook/DM → çık; `GUILD_ID`/`KANALLAR` ayarlıysa dışarıdaki sunucu/kanal → çık. `content_safe` (mention'lar `@ad`, `@everyone` zararsız).
1b. **Resim eki:** `msg.attachments` içinde `content_type`'ı `image/` ile başlayan ilk ekin URL'i alınır. Erken çıkış artık "metin boş" değil "metin de ek de yok": sırf görsel atılmış mesaj da işlenir. Hafızaya/kanal notuna/sohbet satırına giden metin işaretlenir: metin varsa `[resim] <metin>`, yoksa `[resim attı]`. URL yalnız sohbet geçmişindeki `Mesaj.resim` alanına konur ve **yalnız en son kullanıcı mesajında** kalır (yeni satır eklenirken eskilerin `resim`'i `None` olur: discord cdn linki ömürlü, eski görseli her turda yollamak token yakar). `mesaj_json` bu alanı görürse istek gövdesinde `content` düz metin değil `[{text},{image_url}]` dizisi olur (ajanlar.rs `resimci` ile aynı biçim).
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

## Çıktı protokolü (her kişilikli cevap bundan geçer)
Model düz metin değil **satır bazlı bir protokol** yazar; `cevap_parcala` çözer (`soy` uygulanmış
metin üzerinde, yeniden soyma yok):
- **Her satır ayrı bir discord mesajıdır.** Boş satırlar atılır, en çok `PATLAMA_SINIRI` (4) satır
  gider; fazlası düşer (debug log). 1900'ü aşan satır `bol` ile kendi içinde bölünür.
- **`tepki: 💀`** satırı yazı olarak GİTMEZ, cevaplanan mesaja emoji tepkisi olur. Büyük/küçük harf
  ve "tepki :" boşluğu tolere edilir; iki noktadan sonraki ilk emoji dizisi alınır (harf, boşluk ve
  bilinen emoji bloklarından bir karakter — U+2600–27BF, U+2B00–2BFF, U+1F000–1FAFF, ©/®/™ gibi
  tekiller — + peşindeki varyasyon seçici/ZWJ/keycap, en çok 8 char). Tanım bilerek dar: `—`, `…`,
  `→`, tipografik tırnak emoji değildir, Discord bunlara 400 döner. `:kekw:` gibi özel emoji biçimi
  ve emoji bulunmayan satır sessizce düşer. İlk tepki kazanır.
- **Susma:** tek başına `-` (ya da `"-"`, `'-'`, `[sus]`, `(sus)`) satırı `sus` bayrağını kaldırır ve
  satır olarak gitmez. Yalnız `sus` varsa hiçbir şey gönderilmez.
- **Kırıntı ve slop:** `'` ile başlayan satır (önceki mesajın devamı) atılır; `slop_temizle` baştaki
  `- `/`* `/`• ` madde öneklerini ve `**`/`__` işaretlerini siler (backtick'in kendisi de İÇİ de
  korunur: `` `__init__` `` bozulmaz). `1. `/`2) ` numara öneki yalnız cevapta **≥2 numaralı satır**
  varsa (gerçek liste) silinir — tek satırdaki "3. sınıftayım" Türkçe sıra sayısıdır. Aynı turda
  birebir tekrar eden satır ikinci kez gitmez. **Kısa satır elenmez**: "he", "yok", "la" doğal
  tepkidir.
- Geçmişe ve kanal notuna giren biçim `Cevap::protokol_metni()`: satırlar `\n` ile, varsa sonunda
  `tepki: 💀`. Model bir sonraki turda kendi biçimini görsün diye böyle.

## cevapla (bir sohbet turu, stream)
```
kilit ── meşgul? çık ── sohbet var? ── talimat seç ── meşgul=1 ── kilit bırak
bekle 0,15-0,35 sn ── güncel geçmiş + son mesaj + bekleyenler ── ruh hali (4 turda bir) ── arastir(link/haber/araştır) ── hedef seçimi (2+ yazan varsa) ── soru tavanı (soru_fazla_mi) ── yazıyor…
uret_akis(stream, bütçe: cevap_butcesi!) ── (hata: meşgul=0, çık)
gonder_akis: mesaj ilk ANLAMLI içerikle açılır (yerleşim boş kaldığı sürece "ilk" harcanmaz; akis_kesiti kısa yarım satırı bekletir) ── AKIS_DUZENLEME (1,2 sn) aralıkla düzenlenir ── düşünürken (cevap başlamadı): göster="Düşünüyorum...", gizle=canlı kelime sayacı, sessiz/kapalı=hiçbir şey (mesaj cevap başlayana dek hiç açılmaz) ── cevap başlayınca aynı mesaj düzenlenerek stream ── göster: thinking newline'sız tek satır, hem spoiler hem kod bloğu ── gizle: thinking mesajda yok, cevap sonunda "Düşünce Sürecini Göster" butonu (interaction_create tıklayana ephemeral kod bloğu açar, düşünce deposu 50 mesaj) ── sessiz: reasoning isteniyor (arka planda çalışıyor) ama hiç toplanmıyor/gösterilmiyor, buton da yok ── kapalı: istek reasoning'siz ── discord yanıtı yalnız ilk mesajda
akış SÜRERKEN görünen kısım: tamamlanmış satırlar (ardında \n olan) + son yarım satır ancak YARIM_SATIR_ESIGI (12) karakteri geçtiyse (akis_kesiti) ── böylece "tep" yarım hâlde mesaj olup silinmez
akış BİTİNCE cevap_parcala:
  sus ∧ satır yok ∧ TEPKİ DE YOK → açılan geçici mesajlar silinir, AkisSonuc::Sus (geçmişe hiçbir şey girmez, sayac artmaz, son_aktivite tazelenmez, yedek uret ÇAĞRILMAZ; hackli yine de azalır) ── "-" ile "tepki: 💀" birlikte gelirse susma değil, emoji düşer
  hiçbir şey yok → AkisSonuc::Bos → stream'siz yedek uret + satır bazlı tekrar elemesi + gonder_cevap
  tekrar_mi SATIR BAZLI: son 5 bot satırıyla aynı olanlar düşer; hiç satır kalmaz ve tepki de yoksa bir kez yeniden üret, yine tekrarsa (ya da yeni cevapta ne satır ne tepki varsa) açılanları sil ve Bos
  final yerleşim yaz_akis ile yazılır (fazla mesajlar silinir) ── tepki varsa baglam.tepki_hedefi mesajına create_reaction (hata warn log, akış durmaz; yalnız tepki de geçerli bir cevaptır)
üst üste 2+ farklı kişi yazdıysa HEDEF_SEC mini çağrısı hedef kişiyi seçer; yanıt o kişinin mesajına bağlanır, talimata "ona seslen" notu girer
üretim sırasında yeni mesaj gelse de akış tamamlanır (sil-baştan yok); yeni mesaj sıradaki turda ele alınır
kilit ── meşgul=0 ── her görünen satır ayrı ayrı kendi_mesajlarim'a, hepsi TEK dosya yazımıyla kanal_not_coklu'ya (tepki "bot: tepki: 💀" satırı olarak) ── asistan satırı = protokol_metni ── sayac++ ── hackli-- ── kilit bırak
… yeni mesaj yoksa çık, varsa bir tur daha
```
Talimat önceliği: hack devam > hack çıkış > boş. Üstüne eklenenler: ruh hali, internet bulgusu,
hedef kişi notu, soru tavanı.

## Soru tavanı
`soru_fazla_mi(d, kanal)`: kanal geçmişindeki son 4 bot satırından (`tepki:` satırları sayılmaz)
≥2'si `?` ile bitiyorsa talimata "Bu sefer soru sorma; düz laf et ya da sus." eklenir. Kod ölçer,
uygulamayı model yapar — kesme/kırpma yok. `cevapla` ve CLI sohbet modu ikisi de uygular.

## gonder_satirlar (stream OLMAYAN yollar)
`soy` + `cevap_parcala` → `gonder_cevap` (gövde; elinde çözülmüş `Cevap` olan yollar doğrudan onu
çağırır) → satırlar sırayla ayrı mesaj. Aralarına `300 ms + 15 ms × karakter`
(tavan 1500 ms) gecikme + `broadcast_typing` girer: stream'in kendi temposu burada yok, üç mesaj
aynı anda düşmesin. Discord yanıtı yalnız ilk satıra takılır; ping de öyle ama **protokol
çözüldükten sonra**, gönderim anında ilk satırın başına `<@id> ` diye eklenir — metne baştan
yapıştırılırsa `-` ve `tepki:` satırları tanınmıyordu. Tepki hedefi verildiyse emoji atılır ve
kanal notuna protokol biçimiyle yazılır; **hedef yoksa tepki düşürülür** (kanalda görünmeyecek
tepki "gönderildi" sayılmasın). `sus` ya da gidecek hiçbir şey kalmayan cevapta **hiçbir şey gitmez**,
`None` döner — açılış göndericileri (dürtme, sorun, haber tanıtımı, hoş geldin, uyandım, uyanış
cevabı, yolda, gidiyorum, isim duyurusu) o turu atlar, sohbet açılmaz. Döndürdüğü `protokol_metni`
sohbeti tohumlayan açılış metni olur. `saka_yap` görsel + metni tek mesajda yolladığı için
protokolden yalnız ilk satırı alır.

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

## CLI sohbet (`cargo run -- sohbet`)
Discord'a hiç bağlanmadan protokolü denemek için terminal tezgâhı (`src/sohbet_cli.rs`).
```
main: ilk argüman "sohbet" mi → Bot::kur() (DISCORD_TOKEN İSTEMEZ, yalnız model anahtarı)
  anahtar yoksa → "sohbet modu açılamadı: <sebep>" + çıkış kodu 1
bot_adi boşsa (ready hiç gelmiyor) gelisim.isim, o da yoksa "bot"
sohbet_baslat(ChannelId::new(1)) — gerçek durum/ dosyalarından tohumlanır, kişilik gerçekçi
döngü: stdin satırı "isim: metin" (iki nokta yoksa ya da bir yanı boşsa yazan "emin") · !cik ya da EOF → çık
  hatirla + kanal geçmişi (yalnız BELLEKTE, gecmise_ekle) + sohbet geçmişine kullanici satırı
  soru tavanı talimatı ── uret (stream YOK) ── soy ── cevap_parcala
  çıktı: her satır "bot_adi: satır" · tepki "[tepki 💀]" · sus "(sustu)" · hiçbir şey yoksa "(boş)" · model hatası "(hata: …)" ve döngü sürer
  geçmişe protokol_metni iter, sayac++
```
Durum içeriğine hiçbir şey yazılmaz: `kanal_not` yerine bellek içi `gecmise_ekle` kullanılır, ajanlar ve
döngüler bu kipte hiç çalışmaz. (Tek istisna: `Bot::kur()` canlı yolla ortak olduğu için boş
`durum/{kisiler,konular,olaylar,arsiv,kanallar}` ve `resimler/` klasörlerini oluşturur.) **Doğrulanmadı:** gerçek model anahtarı bu makinede yok, canlı
cevap alışverişi görülmedi (bkz. AGENTS.md "Bilinen açıklar").

## Komut arayüzü (slash → embed kart → detay modalı)
`ready` → her sunucuya `/durum` `/yardim` `/zihin` kaydı (idempotent) → kullanıcı slash çalıştırır →
`interaction_create(Command)` → ephemeral **embed kart** (`durum_mesaji`/`yardim_mesaji`/`zihin_mesaji`),
yalnız çağırana görünür.
`/zihin` kartı: üç sütun (Kişiler/Konular/Olaylar) + üstte kişi select menüsü, altta Konular/Olaylar/Bot özeti butonları.
Menüden kişi seç ya da butona bas → `interaction_create(Component)` → ilgili **detay modalı**
(`modal_kisi` / `modal_konular` / `modal_olaylar` / `modal_ozet`); her bölüm kendi etiketli alanında, tek kutuya boca yok.
Kullanıcı modal'ı gönderirse → `interaction_create(Modal)` → kısa ephemeral onay; girdi toplanmaz.
Paralel düz metin: `!durum` ortak `durum_metni`; `!yardım` aynen. `!zihin` artık kart değil **görsel** atar (aşağı bak).

## !zihin görsel yolu (panel ekran görüntüsü)
`!zihin` → `zihin_gorsel::zihin_verisi(&durum())` kilit altındaki alanları kopyalar, **guard satır
sonunda düşer** → `spawn_blocking`: `dosyalari_oku` (durum/ okumaları) → `zihin_svg` (yerleşim, metin
sarma, XML kaçışı) → `resvg` ile 2x PNG → `durum/zihin.png` (her seferinde üstüne) → `gonder(...,
dosya: Some(&yol), ...)` tek satır başlıkla ("zihnim, {tarih}").
Üretim ya da yazma patlarsa: hata `warn`'lanır ve eski `zihin_embedleri` kartı gönderilir — `!zihin`
boş dönmez. Etkileşimli detay (kişi menüsü, bölüm butonları, modallar) `/zihin`'de kalır.
Aynı görsel Discord'suz da üretilir: `cargo run -- zihin`.

## Sunucuya bağlanınca
`guild_create` (sunucu başına bir kez) → arka planda: 14 gün geriye tarama (izinli kanallar, 100'lük sayfalar) → ham hafıza son 2000 → profilci → hoca (huy boşsa). Yeniden bağlanmada tekrar taranmaz.
`guild_create` ayrıca süreç başına bir kez (`Handler.duyuruldu`) varsayılan kanala tek satır sürüm duyurusu atar: `geldim · v0.2.0 (69e2851, 2026-09-02) · model … · düşünme …` — hafızaya/kanal notuna yazılmaz (bot bunu kendi lafı sanmasın). `ready`'de değil, çünkü sunucu önbelleği orada henüz dolu değil.

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

## Zihin zinciri (sohbet → günlükçü) ve teşhis
`zaman_asimi_kapat` → info log `zihin: sohbet kapandı [kanal] (30 dk sessiz) → kuyruk (n), günlükçü 10 dk içinde`
→ `bellek_dongusu` (10 dk) → `gunlukcu` → info `zihin: günlükçü [kaynak]: k kişi, m konu, o olay yazıldı`
ya da warn `zihin: günlükçü başarısız [kaynak]: <sebep>`. `gunlukcu` artık `Result<GunlukcuOzet, Hata>` döner.
`!zihin test`: kanalın son 30 satırını hemen günlükçüye verir, sonucu tek mesajla yazar (40 dk beklemeden).
Reasoning zorunlu modelde (glm-5.3-flash) `sor_ham`: 400 "mandatory" → alanlar kaldırılır + openrouter'da
`reasoning.effort=low` + bütçe max(2×, 1500); 200 ama content boş → JSON bekleyen kategorilerde
(gunlukcu, isteklilik, hedef_sec, ruh_hali, uyanis) düşünce alanındaki `{…}` içerik sayılır (warn log),
düzyazı çağrısında sayılmaz; yine boşsa bütçe büyütülüp bir kez daha denenir; hata mesajı kategori/model/
bütçe/düşünce uzunluğunu içerir.

## Debug modu (`!debug`, ayar paneli)
`Durum.debug` açıkken `debug_not` tek satır (⚙ …, ≤300 kr) DEBUG_KANALI'na, yoksa mesajın kanalına yazar
ve info loglar; hafızaya/kanal notuna girmez. İzler: mesaj kararı (`etiket` / `diyalog sürüyor` /
`isteklilik p/eşik · sebep: … → cevap|sus` / `2 dk sınırı` / `yedek zar`), cevapla turu (`ruh hali`,
`hedef`, `soru tavanı`, `n satır gönderildi · tepki X` / `sus (-)` / `akış boş → yedek uret`),
`sohbet kapandı (30 dk sessiz)`.

## Ayar paneli (`!ayarlar`, `/ayarlar`)
Embed (sürüm, model, düşünme, debug, uyku, seyahat) + butonlar: düşünme göster/gizle/sessiz/kapat
(etkin olan Primary), debug aç/kapat, uyandır / uyut (8 saat). Buton → `interaction_create(Component)`
`ayar_*` → `Handler::ayar_dugmesi`: komutlarla aynı yollar (`DusunmeKip` + dusunme.md, `debug_ayarla`,
`uyandir`/`uyut` + `uyku_gecisi`) → `UpdateMessage` ile panel yerinde yenilenir. `/ayarlar` ephemeral,
`!ayarlar` kanalda herkese görünür.
