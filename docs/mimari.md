# Mimari

## Tek cümle
Discord olayları → sohbet motoru (12 mesajlık kısa sohbetler) → her cevapta "sistem mesajı"
= çekirdek kişilik + ajanların öğrettikleri + hafızadan o sohbet için getirilenler + görev;
biten sohbet → ajanlar hafızayı ve kişiliği günceller → dosyalar (`durum/`).

## Katmanlar

```
┌──────────────────────── discord (serenity) ────────────────────────┐
│ ready · guild_create · guild_member_addition · message              │
└───────────────┬────────────────────────────────────────────────────┘
                ▼
┌──────────────────────── sohbet motoru (main.rs) ───────────────────┐
│ Durum (tek Mutex) · Sohbet{gecmis,sayac,hackli} · cevapla() · uret()│
│ döngüler: haber · durtme · saka · gezgin · uyku                     │
└───────┬───────────────────────┬────────────────────────────────────┘
        ▼                       ▼
┌── openrouter (sor_ham) ──┐  ┌── hafıza (hafiza.rs) ─────────────────┐
│ sor → uret (kişilikli)   │  │ INDEX.md · kisiler/ · konular/ ·       │
│ sor → analiz (kişiliksiz)│  │ olaylar/ · arsiv/ · getir() · sınırlar │
└──────────────────────────┘  └───────────────────────────────────────┘
        ▲                       ▲
┌── ajanlar (ajanlar.rs, gundem.rs) ────────────────────────────────┐
│ profilci · gunlukcu · hoca · elestirmen · ozetleyici · haberci ·    │
│ resimci · gezgin                                                     │
└─────────────────────────────────────────────────────────────────────┘
        ▲
┌── takvim (uyku.rs, seyahat.rs) ── durum tutmaz, saatten hesaplar ──┐
└─────────────────────────────────────────────────────────────────────┘
```

## Dosya haritası
| Dosya | Satır | Rol |
|---|---|---|
| `src/main.rs` | ~1000 | sabitler, `Durum`, `Bot`, OpenRouter çağrıları, sohbet motoru, döngüler, discord olayları, `main` |
| `src/ajanlar.rs` | ~420 | arka plan ajanları (`impl Bot` bloğu), `Haber`, `rastgele_resim` |
| `src/hafiza.rs` | ~475 | dosya hafızası: yol/oku/yaz/arşiv, `Kisi` biçimi, dizin, getirme, sınırlar, tarih |
| `src/gundem.rs` | ~265 | Sözcü RSS, html temizleme, firecrawl, `gezgin` ajanı, `gundem.md` |
| `src/uyku.rs` | ~130 | uyku planı, yerel saat, uykusuz gece, "ŞU AN" satırı |
| `src/seyahat.rs` | ~240 | yıllık etkinlik takvimi, seyahat penceresi, "ŞU AN" eki |
| `src/promptlar.rs` | 28 | her prompt için `include_str!` sabiti |
| `promptlar/*.md` | 25 dosya | prompt metinleri |
| `Cargo.toml` | | serenity(cache), tokio, reqwest(rustls), serde, serde_json, dotenvy, rand, base64 |

Modüller `main.rs`'in çocuğudur; `use super::*` ile kökün özel öğelerine (sabitler, `Durum`,
`Bot`, yardımcılar) erişir. Aynı crate içinde `impl Bot` blokları dosyalara dağılmıştır.

## Paylaşılan durum
Tek `Mutex<Durum>` (`Bot::durum()` zehirlenmeye dayanıklı). İçinde: bot adı, favori adı, ajan
çıktıları (profil, huy, duzeltmeler, kendim, dizin, gundem), ham hafıza (`VecDeque<String>`,
2000 satır), botun kendi mesajları (50), açık sohbetler, meşgul kanallar, yasaklı kanallar,
haber bekleyen kanallar, atılan haber kimlikleri, taranan sunucular, uyku planları, uyurken
gelen etiketler, seyahat duyuru işaretleri. Kilit hiçbir `.await` boyunca tutulmaz.

## Her cevabın sistem mesajı (sistem_metni)
Sırayla, boş olanlar atlanır:
1. `kisilik.md` (`{ad}`, `{favori_satiri}` dolu)
2. HUYUN — `huy.md` (hoca)
3. BU GRUP HAKKINDA BİLDİKLERİN — `profil.md` (profilci)
4. HAFIZA DİZİNİ — `INDEX.md` (işaretçi: kişi+puan+etiket, konular, olay sayısı)
5. BU SOHBET İÇİN HAFIZADAN GETİRİLENLER — `hafiza::getir` (bütçe 6000 kr)
6. GÜNDEM — `gundem.md` son 3 giriş (gezgin)
7. SENİN SON HALİN — `kendim.md` (gunlukcu)
8. ŞU AN — tarih/saat/gün + seyahat hali (uyku yalnız kodda cevap kapısıdır)
9. KENDİNE NOTLAR — `duzeltmeler.md` (elestirmen)
10. ŞU ANKİ GÖREVİN — o çağrının talimatı (veda, hack, hoş geldin, yoldan mesaj...)

## Ajan takvimi
| Ne zaman | Ajanlar |
|---|---|
| sunucuya bağlanınca (bir kez/sunucu) | gecmisi_oku → profilci → hoca (huy boşsa) |
| açılıştan 10 dk sonra, sonra 4 saatte bir (uyanıkken) | gezgin |
| 6 saatte bir (uyanıkken) | profilci → gunlukcu(gözlem) → hoca → haberci+paylaş; seyahatteyse sadece profilci+hoca |
| her biten sohbet | gunlukcu(sohbet) → ozetleyici → elestirmen |
| dakikada bir | uyku planı kontrolü, uyanınca bekleyen etiketlere dönüş |
