# durum/ kayıtları

Çalışma zamanı hafızası; git'e girmez. Bot yeniden başlayınca buradan yükler.

**Depolama**: `arsiv/` dışındaki her şey tek bir dosyada, `durum/hafiza.redb`
(redb — saf Rust, ACID transaction; neden rusqlite/düz-JSON değil de bu:
`docs/kararlar.md`). Aşağıdaki tablo hâlâ eski dosya yollarını gösteriyor çünkü tasarım
gereği öyle: her kayıt, redb'de o eski göreli yol string'i anahtar olarak (`"kisiler/1.md"`,
`"profil.md"`, ...), değeri ise dosyanın tutacağı metnin birebir aynısı olacak şekilde saklanır
— alan adları, sınırlar, biçimler aşağıda değişmeden geçerli, yalnız konteyner değişti (bkz
`src/memory.rs`'nin modül yorumu). `arsiv/` tek istisna: hâlâ gerçek `.md` dosyaları, çünkü
yalnız insan içindir, bot bir daha okumaz (bkz aşağıdaki tablo). Eski bir `durum/` ağacından
geçiş `cargo run -- migrate-durum` ile yapılır (`src/migrate.rs`).
(Dosya/dizin adları ve içindeki alan adları bilerek Türkçe bırakıldı, bkz AGENTS.md madde 8.)

| Kayıt | Yazan | Okuyan | Sınır / bakım |
|---|---|---|---|
| `INDEX.md` | `refresh_index` (diarist, summarizer, açılış) | her cevap (sistem mesajı), coach | ≤40 kişi, ≤30 konu, ≤3 ay; türetilmiş dosya, elle düzenleme |
| `profil.md` | profiler | her cevap, news_agent, wanderer, coach | her 6 saatte yeniden üretilir (max_tokens 1200) |
| `huy.md` | coach | her cevap, wanderer seçimi, uyku (gerginlik) | 6 saatte bir evrimleşir (800 token) |
| `duzeltmeler.md` | critic | her cevap | her biten sohbette yeniden yazılır (400) |
| `kendim.md` | diarist (`kendim` alanı doluysa) | her cevap, coach, uyku (gerginlik) | tek parça, üstüne yazılır |
| `gundem.md` | wanderer | her cevap (son 3), coach (son 3) | 12 giriş; eskisi `arsiv/gundem.md` |
| `kisiler/<id>.md` | diarist, summarizer | `retrieve` (sohbetteki kişiler), dizin | >1800 kr → özet, hedef 1000; eski hali `arsiv/kisiler/<id>.md` |
| `konular/<slug>.md` | diarist, summarizer | `retrieve` (anahtar eşleşmesi), dizin | >1500 → özet, hedef 800 |
| `olaylar/YYYY-AA.md` | diarist, summarizer | `retrieve` (son 8), dizin | >6000 → eski %60 satır 3-5 satıra; taşınanlar `arsiv/olaylar/YYYY-AA.md` |
| `arsiv/…` | archive | insan | yalnız eklenir, `## tarih öncesi` başlıklı |
| `kanallar/<id>.md` | `channel_note` (her mesaj, bot dahil) | açılış, `start_chat` tohumu | son 60 satır, her yazımda dosya baştan yazılır |
| `model.md` | `/model` | açılış (`main`, env MODEL'i ezer) | tek satır model kimliği |
| `dusunme.md` | `/dusunme kip:göster/gizle/sessiz/kapat` | açılış (`State::load`; dosya yoksa göster) | `goster`, `gizle`, `sessiz` ya da `kapali` |
| `debug.md` | `/debug durum:aç/kapat`, ayar paneli | açılış (`State::load`; dosya yoksa kapalı) | `acik` ya da `kapali`; açıkken karar izleri kanala düşer |
| `gelisim.md` | check_growth, pick_name | açılış (`growth::load`) | `dogum: unix` `sohbet: n` `mesaj: n` `evre: i` `isim: ad` satırları |

## Biçimler

### kisiler/<id>.md
Dosya adı discord kullanıcı id'si (ad değişikliği bölünme yaratmaz).
```
# Emin
id: 259669117248864257
kullanici_adi: kaju
eski_adlar: önceki görünen ad
puan: +3
etiket: rust, oyun
not: rust'ı övdüm diye üç mesaj laf soktu

## Bildiklerin
- yks'ye hazırlanıyor
- otosaray diye bir projesi var

## Son olaylar
- 2026-09-01 22:14:03: rust vs go tartışması, bot kaçtı
```
`puan` -10..10 (favori sabit +10, not sabit). `etiket` ≤6, küçük harf. `not` tek cümle,
kanaat. `eski_adlar` ≤5. `Bildiklerin` tekrar etmeyen kalıcı bilgiler. `Son olaylar` her biten
sohbetten bir satır, zaman damgaları saniyeli (`date_time`). Ayrıştırma `Person::parse`: `# ` başlık,
`id:` `kullanici_adi:` `eski_adlar:` `puan:` `etiket:` `not:` alanları, `## Bildik…` ve
`## Son…` bölümleri, `- ` satırları. Bilinmeyen satır yok sayılır. İsim→id çözümü `State.name_to_id`;
çözülemeyen kayıt o tur atlanır (loglanır). Eski slug dosyaları okunmaz, zamanla silinebilir.

### konular/<slug>.md
```
# otosaray projesi
etiket:

- 2026-09-01: emin model eğitimi için veri toplamaya karar verdi
```
`etiket:` satırı şimdilik boş bırakılır; `retrieve` içeriğin tamamında anahtar arar.

### olaylar/YYYY-AA.md
```
- 2026-09-01 #genel: lng ve emin bota hacklenme şakası yaptırdı, bot 12 mesajda kaçtı
```
Özetlenince dosya başına `- ` ile başlamayan özet satırları gelir; `- ` satırları ham kayıttır
(`refresh_index` yalnız `- ` satırlarını sayar).

### gundem.md
```
## 2026-09-01 14:20
(botun kendi ağzından 10 satıra kadar günlük)
```

## Slug kuralı
`memory::slug`: küçük harf; ç→c ğ→g ı→i ö→o ş→s ü→u â→a î→i û→u; alfanümerik dışı tek `-`;
boşsa `bilinmeyen`. Kişi anahtarı görünen addır (`display_name(&User)`), kullanıcı id değil.

## Bütçe (bir cevaba giden hafıza)
`retrieve`: kişi dosyaları (≤4 × ≤1200) → konu (≤2 × ≤800) → son 8 olay → ham hafızadan ≤12 satır
(≥2 anahtar, ≤200 kr). Toplam ≤6000 karakter; sığmayan bölümde durur. Dizin ve profil bütçeye
dahil değildir (sistem mesajının sabit kısmı).
