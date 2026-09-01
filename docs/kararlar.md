# Kararlar ve gerekçeleri

Tarih sırasıyla. Bir kararı değiştirirken buraya yeni satır ekle, eskisini silme.

- **2026-09-01 · Python → Go → Rust.** Emin'in isteğiyle iki kez dil değişti. Rust'ta kalındı;
  serenity 0.12 + tokio. Go sürümü git geçmişinde (`git log --all -- main.go`).
- **OpenRouter için SDK yok, reqwest ile ham JSON.** Tek `sor_ham` fonksiyonu; görsel girişi
  (`image_url`) için gövdeyi elle kurmak kolay, bağımlılık az, ne gittiği görünür.
- **Promptlar `.md` + `include_str!`.** Emin'in isteği; metin düzenlemek kod düzenlemekten
  ayrı, başlık satırı modele bağlam veriyor. Bedeli: değişiklik yeniden derleme ister.
- **Kişilik statik değil, ajanlar yazar.** Çekirdek kurallar `kisilik.md`'de sabit; huy (hoca),
  düzeltmeler (eleştirmen), kanaatler ve bilgiler (günlükçü), gündem görüşü (gezgin) dosyadan.
  Gerekçe: "bot kendi kişiliğini inşa etsin" isteği; tek promptla kişilik büyümez.
- **Ajanlar kişiliksiz (`analiz`).** Profil çıkarma ve seçim işlerinde persona gürültü yapıyordu.
- **Kanaat JSON'u → kişi dosyaları.** `kanaatler.json` tek dosyaydı, büyüyordu ve her cevapta
  gidiyordu. İkinci-beyin mimarisi: dizin her cevapta, kişi dosyası yalnız o sohbette gerekince.
- **Hiçbir şey silinmez, özetlenir; ham parça arşive.** Emin'in ikinci beynindeki kural.
- **Sınırlar kodda kesilir.** Model puan/uzunluk/format konusunda güvenilmez; clamp, truncate,
  "küçülmediyse dokunma".
- **Aynı kanalda tek cevap üretimi (`mesgul`).** Spam ile API faturası şişmesin, cevaplar
  birbirinin üstüne binmesin. Bu sırada gelen mesajlar geçmişe düşer, sonraki turda görülür.
- **Mention'lar kapalı.** Model `@everyone` yazabilir; tek istisna hoş geldin pingi.
- **Botlara/webhook'lara/DM'e cevap yok.** Bot-bot döngüsü.
- **Kişi anahtarı görünen ad, id değil.** Model dökümde adları görür, id'yi göremez; dosya
  adı okunur olsun. Bedeli: aynı görünen adlı iki kişi çakışır (bilinen açık).
- **Favori kullanıcı kodda sabit (+10).** Emin'in isteği; model ne derse desin.
- **Tarih/saat dış kütüphanesiz.** Hinnant algoritması 15 satır; chrono bağımlılığına değmez.
  TR yaz saati yok, sabit +3.
- **Uyku ve seyahat durum tutmaz.** Takvim ve saat yeterli; yeniden başlatma tutarlılığı bedava.
  Yalnız uyku planı (rastgele ±45 dk ve uykusuzluk zarı) bellekte, yeniden başlatınca yeniden
  atılır (kabul edildi).
- **Uykusuzluk şansı kişiliğe göre.** `kendim`+`huy` içinde gerginlik kelimeleri varsa %7 → %20.
  Modelden zar attırılmadı; model rastgelelikte kötü.
- **Seyahatte ajanlar çalışmaya devam eder, haber/şaka durur.** Öğrenme kesilmesin, ama
  "telefondan bakan" biri haber atmaz.
- **Hack şakası link ve bilgi istemeyi yasaklar.** Şaka gerçek phishing'e benzemesin.
- **Görseller `resimler/` klasöründen, git dışı.** Discord CDN linkleri bir günde ölüyor;
  kişisel ekran görüntüleri public repoya sızmasın.
- **`durum/` git dışı.** Kişisel veri (arkadaşlar hakkında notlar) içerir.
- **Repo public** (Emin kararı).
