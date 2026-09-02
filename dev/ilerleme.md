# İlerleme günlüğü

Kronolojik. En yeni üstte. Her satır: tarih · commit (varsa) · ne+neden · doğrulama.

---

## 2026-09-02 · PR #2 merge'ü + düzeltmeler + sıcak yol tahsis temizliği
- Uzak PR (token optimizasyonu, çok sağlayıcılı genellik, tartışma davranışı, prod-hazırlık:
  kategori metriği, reasoning-mandatory geri dönüşü, cache genelliği, ruh hali, GUILD_ID/KANALLAR,
  taranan.md) yerelle birleştirildi. Çakışmalar el çözüldü: bekçi tek fonksiyonda
  (`dongu_bekle` + KAPANIYOR + her dalda 5 sn uyku), `hafiza::yaz` yerel gövde (kilit + sabit
  `.tmp`; uzaktaki pid+sayaç tahsisli ad alınmadı), ilerleme kronolojisi.
- Düzeltmeler: `cevap_ver = acik && katil` (açık sohbette isteklilik sonucu çöpe gidiyordu —
  PR'ın ana vaadi yarım kalmıştı); `devam_eden_diyalog` adı `kucult` ile karşılaştırılır
  (Türkçe İ); `ruh_hali`'nin gereksiz `sayac == 0 ||` koşulu düştü; `CEVAP_TAVANI` 3000 → 4096
  (reasoning tokenleri bütçeden düşer, dar tavan uzun düşünceyi kırpardı).
- Sıcak yol tahsis temizliği: `soy` dilim döndürür (stream'de her edit'teki tam metin klonu
  kalktı), `bol`/`temizle` ara collect'siz, `kanal_not`/`son_mesajlar`/`dokum` Vec'siz,
  `getir` bütçe sayacı + konu tek okuma, `dizin_yenile` hafif başlık çözümleyici,
  `konu_ekle` tek kilit bölgesi, `sohbet_sistemi` contains tahsissiz.
- Doğrulama: 50 test, clippy 0 uyarı, fmt temiz, release build tamam.

## 2026-09-02 · Hafıza sertleştirme + döngü bekçisi + tarama sırası
- `hafiza::yaz` atomik (geçici + rename) + `YAZMA_KILIDI` ile tek sıradan; `ekle` gerçek append
  (dosyanın tamamı yeniden yazılmıyor). Test: disk round-trip + geçici dosya kalmıyor (43 test).
- Günlükçü JSON'u çözülemezse ham çıktı `arsiv/gunlukcu-<kaynak>.md`'ye kurtarılır.
- `dongu_bekle`: altı döngü bekçiyle başlar, panikte log + 5 sn sonra yeniden; `KAPANIYOR`
  (AtomicBool) ile zarif kapanış — döngüler tik başında döner, bekçi yeniden başlatmaz.
- Süresi dolan haber sohbetleri dakika tikinde temizlenir (`zaman_asimi_kapat`), `haber_bekleyen` şişmez.
- Açılış taraması hafızanın ÖNÜNE eklenir: tarama sürerken gelen canlı mesajlar ezilmez,
  kronoloji korunur; `ad_id` yalnız boşsa dolar (canlı eşleme öncelikli).
- Doğrulama: 43 test, clippy 0 uyarı, fmt temiz.

## 2026-09-02 · HTTP timeout mimarisi + mekanik sertleştirme (d8d7fc8)
- Global 60 sn zaman aşımı kalktı: `connect_timeout` 15 sn + `read_timeout` 120 sn (her okumada
  sıfırlanır → ilk tokeni kapsar); toplam süre sınırı yok, uzun düşünme akışı kesilmez.
- Geçici hatalarda (ağ, 429, 500/502/503/504) yeniden deneme: 2 sn ve 4 sn geri çekilme,
  en çok 2 ek deneme (`sor_ham` + `sor_ham_akis`; akış yalnız açılmadan önce).
- `reasoning_kapat` sağlayıcıya göre: openrouter `reasoning.enabled`, mistral'e parametre yok,
  diğerleri `enable_thinking:false` (ikisini birden yollamak bazı sağlayıcıları bozuyordu).
- `MesgulGuard` (RAII): panik dahil her çıkışta kanalın meşgul bayrağı bırakılır;
  8 dağınık remove tek guard'a indi.
- `soy` char güvenli (bayt dilimi yok) + `kucult` (İ→i̇ birleşik noktası) — 4 yeni test.
- Typing edit döngüsünden çıktı (`yaz_akis`); model çağrısından önce bir kez.
- Doğrulama: 42 test, clippy 0 uyarı.

---

## 2026-09-02 · İlk canlı loglar: iki gerçek üretim hatası
Kullanıcı canlı bottan (`z-ai/glm-5.3-flash`, openrouter) gerçek log yapıştırdı. İki ayrı hata:

- **400 "Reasoning is mandatory ... cannot be disabled"**: `düşünme kapat` kipinde gönderilen
  `reasoning:{enabled:false}` bu GLM varyantında reddediliyordu, sohbet o kanalda hiç cevap
  veremiyordu. `reasoning_kapat` artık alanları eklediyse `true` döner; `sor_ham`/`sor_ham_akis`
  bu spesifik hatayı (`reasoning_zorunlu_hatasi`) tanıyıp alanları kaldırıp bir kez daha dener.
  Not: aynı modelin küçük `max_tokens`'lı mini-çağrılarda (haber_sec=10, hedef_sec/ruh_hali=40,
  isteklilik=80) gizli reasoning'in bütçeyi yiyip boş cevap üretme ihtimali var (profilci
  logunda "boş yanıt" da görüldü) — bu kod tarafında tam çözülebilecek bir şey değil, reasoning
  zorunlu modeller bu mimariyle (küçük bütçeli çok sayıda mini-çağrı) temelde gerilimli.
- **"her mesaja cevap veriyor" (asıl disküsyon şikayeti)**: kod incelenince `acik` (sohbet bu
  kanalda açık mı) tek başına "değerlendirmeye gerek yok, direkt cevapla" anlamına geldiği
  görüldü — sohbet bir kez açılınca kanaldaki HERKESİN mesajı, kime yazdığına bakılmaksızın
  doğrudan cevaplanıyordu (bu, önceki turda düzelttiğim reply-to/etiket meselesinden tamamen
  ayrı bir mekanizma). `devam_eden_diyalog` eklendi: sohbetteki son user mesajının sahibi bu
  mesajı atanla aynıysa (gerçekten kendisiyle konuşuyor) otomatik devam eder; farklı biri
  yazdıysa yine isteklilik değerlendirmesinden geçer (aynı 2 dk rate limit, ek maliyet yok).
- Doğrulama: 43 test, clippy 0 uyarı, `cargo fmt`, release build. `message` handler'daki yeni
  mantık (inline, pure fonksiyon değil) birim testle doğrulanamadı — canlıda izlenmeli.

---

## 2026-09-02 · Ruh hali ajanı + ikinci dayanıklılık turu
Önceki tur devamı: "ikisine de başla" (ruh hali ajanı + backlog).

- **Ruh hali ajanı**: `promptlar/ruh-hali.md`, `Bot::ruh_hali_belirle`, `Sohbet.ruh_hali`. Sohbet
  açılınca ve her 4 turda bir (her mesajda değil) ucuz mini çağrı; taksonomiden tek durum+yoğunluk
  seçer, yoğunluk <3 nötr sayılır. Talimata "ŞU ANKİ RUH HALİN" diye eklenir; kişilik promptunda
  "ilan etme, üsluba yedir" kuralı.
- **`hafiza::yaz` atomik** (geçici dosya + rename) — crash/kill'de yarım dosya kalmaz.
- **`dongu_bekci`**: 6 arka plan döngüsü artık paniklerse loglayıp 5 sn sonra yeniden başlıyor
  (eskiden sessizce bir daha hiç çalışmıyordu).
- **`soy` char-güvenli**: `onek.len()` (bayt, lowercase'den) yerine `.chars().skip(n)`; Türkçe
  büyük İ gibi harflerde panik riski vardı (lowercase bayt uzunluğunu değiştirir).
- **`durum/huy.md` uyku teması temizlendi + `hoca.md` düzeltildi**: kullanıcı şikayeti "!uyan
  attım ama hâlâ yorgun/uykum var diyor" — huy.md'de hoca'nın test sırasındaki sık `!uyan`
  muhabbetini kalıcı TAVIR sanıp yazdığı satırlar ("uykulu", "uyudum amk", "uyandırılmaktan
  bıktım") botun gerçek uyku sistemiyle hiç ilgisiz, salt kelime çakışması kafa karıştırıyordu.
  Ayrıca DOĞALLIK bölümü ters çalışıp ("bırakılacak kalıp" yerine) sabit replik dayatıyordu
  (KALIPLAR icadı). Prompt düzeltildi, mevcut dosya elle temizlendi — ama bu depodaki
  `durum/huy.md` kullanıcının CANLI botunun kullandığı dosya olmayabilir; öyleyse aynı düzeltmeyi
  (dosyayı silip yeniden başlatma ya da elle satır silme) orada da yapması gerekir.
- TOON değerlendirmesi kullanıcıya tekrar soruldu, aynı sonuç: genel taşıma değmez, tek aday
  dizin (INDEX.md), henüz uygulanmadı.
- Doğrulama: bu turun sonunda tek seferlik fmt+clippy+test+release (kullanıcı isteği: "compile
  check'i en sona bırak", ara ara derlemek yerine).

**Kalan (bilerek ertelendi)**: reasoning_kapat hedef adrese göre koşullu (düşük öncelik, Mistral'in
bilinmeyen üst alanı reddettiği doğrulanmadı) · ajan yazımları tek sıra · günlükçü JSON hatası ham
döküm kurtarma · arsivle append · zarif kapanış (watch) · uyanış kanal bazlı · süresi dolan haber
sohbeti temizliği · tarama sırası · typing'i edit döngüsünden çıkarma · hata sınıflandırma+retry.

---

## 2026-09-02 · Token optimizasyonu + prod-hazırlık taraması
Kullanıcı isteği: "proje çok fazla token harcıyor, bir optimizasyon motoru sağlamalıyız" +
sonrasında prod-hazırlık, çok-sağlayıcılı genellik, disküsyon/reply-to davranışı, kanal
geçmişi tarama hatası ve kapsam daraltma istekleri aynı oturumda geldi. Hepsi tek pakette:

- **isteklilik/hedef_sec cache'e taşındı**: eskiden `analiz()` her mini çağrıda profil+dizin'i
  user mesajına gömüp tam fiyatına yeniden yolluyordu (kanal başına en sık tetiklenen çağrı).
  Artık `sor_bolumlu` doğrudan çağrılır, profil+dizin/talimat sabit (cache_control'lü) blokta.
- **Sohbet cevabına release'de de token tavanı**: `CEVAP_TAVANI=3000` (eskiden `None`, bütçesiz).
- **Token metriği çağrı-tipi kırılımlı**: `Metrik.kategoriler`, `!durum` en çok yakan kalemleri
  döker; `Kullanim.prompt_tokens_details.cached_tokens` okunuyor (önbellek isabetini görmek için).
- **cache_control artık hedef adrese göre koşullu** (`onbellek_destekler`): ilk halde model adına
  bakıyordu (claude/anthropic/gemini), kullanıcı "GLM'i niye desteklemiyorsun" diye sorunca yanlış
  soru olduğu anlaşıldı — OpenRouter'a giden istekte hangi model olursa olsun güvenle eklenebilir
  (OpenRouter kendi şemasının parçası, desteklemeyen modelde yok sayar); asıl risk Mistral native
  API'si ya da özel `API_ADRES` router'ı. Artık yalnız `openrouter.ai` adresine bakıyor.
- **Reply-to yeniden koşullu hale geldi**: `Sohbet.son_etiketlendi` eklendi; taban `yanit` yalnız
  etiketliyse ya da `bekleyenler.len() > 1` ise `son_mesaj`, aksi halde `None` (düz mesaj). Önceki
  "her cevap yanıt olsun" kararını geri alır (kararlar.md'de iki karar da duruyor, gerekçesiyle).
- **`durum/taranan.md` kalıcı**: "her bağlandığında mesajları en baştan çekiyor" şikayeti — `taranan`
  bellek-içiydi, her süreç yeniden başlayışında 14 günlük tarama tekrarlanıyordu.
- **GUILD_ID/KANALLAR (.env, isteğe bağlı)**: bot'u tek sunucuya/kanal listesine kilitler; boşsa
  eski davranış (her erişilen yerde çalışır) aynen sürer.
- **HTTP client timeout ayrıldı (P0 kapandı)**: `connect_timeout(10sn)` + `timeout(180sn)`,
  eskiden tek `.timeout(60sn)` uzun stream'i ortasında kesebiliyordu.
- **`mesgul` bayrağı RAII (`MesgulKilit`)**: 7 elle `remove` çağrısı yerine `Drop`; aradaki bir
  panik artık kanalı sonsuza dek kilitli bırakmıyor.
- Doğrulama: 40 test, clippy 0 uyarı, `cargo fmt`, release build tamam. Canlı Discord'ta hiçbiri
  görülmedi (token yok, proje genelinde geçerli kısıt).
- Ertelendi (kullanıcıya soruldu/önerildi, henüz kodlanmadı): `durum/` dosyalarını TOON'a çevirme
  (riskli/düşük getiri değerlendirmesi yapıldı, aşağıda), kişilik promptuna insan ruh hali taklidi
  taksonomisi eklenmesi, geniş prod-hazırlık backlog'unun geri kalanı (dev/yol-haritasi.md).

**TOON değerlendirmesi**: `durum/` çoğunlukla serbest metin/nesir (kisilik, huy, profil notları) —
TOON'un asıl kazandığı yer tekdüze tablo/array veri, burada büyük fayda yok; kişi/konu/olay
dosyaları model tarafından promptlarla (15 dosya) doğrudan okunup yazılıyor, TOON'a geçmek hepsini
yeniden yazmak + küçük modellerin (gpt-4o-mini, GLM) nadir görülen bir formatı güvenilir üretip
üretemeyeceği riskini taşımak demek. Tek gerçek aday `INDEX.md` (dizin): tekdüze, her cevapta
gidiyor. Henüz uygulanmadı, kullanıcıya soruldu.

---

## 2026-09-02 · Adım 8 · Modal'lar + /zihin kodlandı
- Yeni `src/modal.rs`: slash komutlar (`/durum` `/yardim` `/zihin`) modal açar, `!` komutları
  paralel düz metin kalır; zihin modalı herkese açık, 5 slot (bot özeti / kişiler iki yarıda /
  konular / olaylar+gündem). `sigdir` 4000 sınırında taşanı son satır/boşluk hizasında keser + not.
- `hafiza.rs` üç yeni döküm yardımcısı: `kisi_dokumleri` (mtime sırası), `konu_dokumleri`,
  `olay_dokumu`. `interaction_create` dallandı: Command→modal, Modal→ephemeral onay,
  Component→`dusunce_dugmesi` (ayrı impl Handler bloğu). ready'de guild slash kaydı (idempotent).
- `komut.rs`: `!zihin` (dizin dökümü + `/zihin` yönlendirmesi), `!durum` artık ortak
  `modal::durum_metni`; YARDIM metnine slash notu.
- serenity 0.12.5 teyitleri: `CreateModal::new(custom_id, title)` sıra, `CreateInputText::new(style,
  label, custom_id)`, `GuildId::set_commands`, interaction varyantı `Interaction::Modal`.
- Doğrulama: 38 test (4 yeni: slot sınırı/kırpma, kısa metin, kişi bölme, durum_metni),
  clippy 0 uyarı, fmt temiz. Kalan risk: canlı Discord'ta modal davranışı henüz görülmedi.

## 2026-09-02 · Adım 8 planlandı: Modal'lar + /zihin (kod henüz yazılmadı)
- Kararlar: slash komutlar modal açar + `!` komutları düz metin paralel kalır; zihin modalı
  herkese açık; 5 slot: Bot özeti / Kişiler I-II / Konular / Olaylar+Gündem.
- `!zihin` mesaj komutu INDEX özeti + `/zihin` yönlendirmesi verecek (kanala 5×4000 dökmek yok).
- serenity 0.12.5 kaynağından teyit: `CreateModal::new(custom_id, title)` (sıra önemli),
  `CreateInputText` create_components.rs'te. Ayrıntılı yapılacak listesi yol-haritasi.md'de.
- Yol haritasındaki Adım 3-6 metin duplikasyonu temizlendi.

## 2026-09-02 · Log gürültüsü kesildi + renkli çıktı + mesaj temizliği
- Konsolu basan serenity/reqwest tracing olayları hedef filtresiyle kesildi: sink yalnız
  `discord_bot*` hedefli kayıtları seviyeye göre geçiriyor; yabancı crate'lerden yalnız
  warn/error görünüyor (gateway hatası vb. kaybolmaz).
- ANSI renk: ERROR kırmızı+kalın, WARN sarı, INFO yeşil, DEBUG/TRACE soluk; zaman damgası
  soluk. TTY algılamalı (dosyaya çıkışta renk yok), `LOG_RENK=on|off` dayatır.
- 10 bağlamsız `ai hatası: {e}` aşamalı oldu (`ai [uret_akis] [{kanal}]:`, `ai [haber_tanit]:`,
  `ai [uyandim]:` ...); `akis yarıda kesildi` uyarısına kanal öneki geldi.
- Doğrulama: 34 test, clippy 0 uyarı, release build.

## 2026-09-02 · Adım 7 · final: docs + doğrulama
- AGENTS.md tazelendi (test sayısı, uyku kuralı, id bazlı kişiler, yeni açık noktalar).
- Tüm adımların docs güncellemeleri tamam (akislar, moduller, sabitler, kararlar, promptlar,
  durum-dosyalari, README).
- Doğrulama: 34 test, clippy 0 uyarı, release build tamam.

## 2026-09-02 · Adım 6 · uyku modu: dinle + biriktir + uyanınca değerlendir
- Uyurken mesajlar ham hafızaya zaten giriyordu; artık `bellek_dongusu` 2 saatte bir gece
  gözlemi yapıp zihne işliyor (`son_gece_gozlem` işaretli).
- Haber turu uyurken haber seçip `stok_haber`'e koyuyor (atmaz); uyanık ilk turda "sabah
  haberi" olarak gider (`haber_gonder` ortak gönderim yolu).
- Uyanış geçişinde: bekleyen etiket varsa `UYANDIM` ile kesin dönüş, hata durumunda liste
  geri konur (kayıp yok). Etiket yoksa `uyanis.md` ajanı gece mesajlarını puanlar
  (`{"ilgi":0-10,"konu"}`); ilgi ≥5 ise `uyanis-cevap.md` ile son kanala sabah sözü.
- Haber seçimine "Nişantaşı Üniversitesi ile ilgili konu önceliklidir" kuralı (haber-sec.md).
- Doğrulama: 34 test, clippy 0 uyarı.

## 2026-09-02 · Adım 5 · hedef seçimi + sil-baştan kalktı
- `Sohbet.son_gelenler` (isim + mesaj id, 20): bot sustuğundan beri yazanlar; bot cevap
  verince boşalır. 2+ farklı kişi yazdıysa `hedef-sec.md` mini çağrısı (40 token) hedefi
  seçer → yanıt o kişinin mesajına bağlanır, talimata "ona seslen" notu girer.
- `AkisSonuc::Eski` kaldırıldı: üretim sırasında yeni mesaj gelse de akış tamamlanır,
  mesaj silinmez; yeni mesaj sıradaki turda ele alınır.
- Ölü alan temizliği: `Sohbet.etiketli`, `AkisBaglam.gelen` düştü.
- Doğrulama: 34 test (hedef_ayikla JSON/düz metin/bilinmeyen ad dahil), clippy 0 uyarı.

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
