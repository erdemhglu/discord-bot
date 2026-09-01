# durum/ dosyaları

Çalışma zamanı hafızası; git'e girmez. Bot yeniden başlayınca buradan yükler. Tümü UTF-8 düz metin.

| Dosya | Yazan | Okuyan | Sınır / bakım |
|---|---|---|---|
| `INDEX.md` | `dizin_yenile` (gunlukcu, ozetleyici, açılış) | her cevap (sistem mesajı), hoca | ≤40 kişi, ≤30 konu, ≤3 ay; türetilmiş dosya, elle düzenleme |
| `profil.md` | profilci | her cevap, haberci, gezgin, hoca | her 6 saatte yeniden üretilir (max_tokens 1200) |
| `huy.md` | hoca | her cevap, gezgin seçimi, uyku (gerginlik) | 6 saatte bir evrimleşir (800 token) |
| `duzeltmeler.md` | elestirmen | her cevap | her biten sohbette yeniden yazılır (400) |
| `kendim.md` | gunlukcu (`kendim` alanı doluysa) | her cevap, hoca, uyku (gerginlik) | tek parça, üstüne yazılır |
| `gundem.md` | gezgin | her cevap (son 3), hoca (son 3) | 12 giriş; eskisi `arsiv/gundem.md` |
| `kisiler/<slug>.md` | gunlukcu, ozetleyici | `getir` (sohbetteki kişiler), dizin | >1800 kr → özet, hedef 1000; eski hali `arsiv/kisiler/<slug>.md` |
| `konular/<slug>.md` | gunlukcu, ozetleyici | `getir` (anahtar eşleşmesi), dizin | >1500 → özet, hedef 800 |
| `olaylar/YYYY-AA.md` | gunlukcu, ozetleyici | `getir` (son 8), dizin | >6000 → eski %60 satır 3-5 satıra; taşınanlar `arsiv/olaylar/YYYY-AA.md` |
| `arsiv/…` | arsivle | insan | yalnız eklenir, `## tarih öncesi` başlıklı |

## Biçimler

### kisiler/<slug>.md
```
# Emin
puan: +3
etiket: rust, oyun
not: rust'ı övdüm diye üç mesaj laf soktu

## Bildiklerin
- yks'ye hazırlanıyor
- otosaray diye bir projesi var

## Son olaylar
- 2026-09-01: rust vs go tartışması, bot kaçtı
```
`puan` -10..10 (favori sabit +10, not sabit). `etiket` ≤6, küçük harf. `not` tek cümle,
kanaat. `Bildiklerin` tekrar etmeyen kalıcı bilgiler. `Son olaylar` her biten sohbetten bir satır.
Ayrıştırma `Kisi::coz`: `# ` başlık, `puan:` `etiket:` `not:` başlık alanları, `## Bildik…` ve
`## Son…` bölümleri, `- ` satırları. Bilinmeyen satır yok sayılır.

### konular/<slug>.md
```
# otosaray projesi
etiket:

- 2026-09-01: emin model eğitimi için veri toplamaya karar verdi
```
`etiket:` satırı şimdilik boş bırakılır; `getir` içeriğin tamamında anahtar arar.

### olaylar/YYYY-AA.md
```
- 2026-09-01 #genel: lng ve emin bota hacklenme şakası yaptırdı, bot 12 mesajda kaçtı
```
Özetlenince dosya başına `- ` ile başlamayan özet satırları gelir; `- ` satırları ham kayıttır
(`dizin_yenile` yalnız `- ` satırlarını sayar).

### gundem.md
```
## 2026-09-01 14:20
(botun kendi ağzından 10 satıra kadar günlük)
```

## Slug kuralı
`hafiza::slug`: küçük harf; ç→c ğ→g ı→i ö→o ş→s ü→u â→a î→i û→u; alfanümerik dışı tek `-`;
boşsa `bilinmeyen`. Kişi anahtarı görünen addır (`ad(&User)`), kullanıcı id değil.

## Bütçe (bir cevaba giden hafıza)
`getir`: kişi dosyaları (≤4 × ≤1200) → konu (≤2 × ≤800) → son 8 olay → ham hafızadan ≤12 satır
(≥2 anahtar, ≤200 kr). Toplam ≤6000 karakter; sığmayan bölümde durur. Dizin ve profil bütçeye
dahil değildir (sistem mesajının sabit kısmı).
