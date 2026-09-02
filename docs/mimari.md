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
| `src/main.rs` | ~90 | `mod`/`use` başlığı, `include!("bot/*.rs")`, `main` |
| `src/bot/*.rs` | ~50 dosya, çoğu <200 | main.rs'in eski içeriği konu bazlı bölündü (aşağıda) |
| `src/komut.rs` + `src/komut/*.rs` | ~10 dosya, hepsi <200 | slash komut yöneticisi (aşağıda) |
| `src/modal.rs` | ~710 | embed/bileşen/modal üreticileri (`/zihin` kartı, detay modalları, ayar paneli, `bilgi_embed`), slash kaydı `komutlari_kayit` |
| `src/sohbet_cli.rs` | ~170 | `cargo run -- sohbet`: discord'suz terminal sohbet tezgâhı (`impl Bot`), çıktı protokolünü denemek için |
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

### `src/bot/*.rs` ve `src/komut/*.rs`: gerçek `mod` değil `include!`
İkisi de main.rs'in (sırasıyla komut.rs'in) eski tek-dosya içeriğini, konu bazlı ve çoğunlukla
200 satırın altında dosyalara böler — ama `mod` ile değil `include!` ile eklenir (bkz
`docs/kararlar.md`): görünürlük, `use super::*` ve `cevap_butcesi!` makro kapsamı hiçbir yerde
değişmesin diye, dosyalar sanki aynı (kök) modülde yazılmışlar gibi derlenir; diğer altı kardeş
modüle (`ajanlar.rs, gundem.rs, komut.rs, modal.rs, sohbet_cli.rs, uyku.rs`) hiç dokunulmadı.
`src/bot/` grupları: `tipler_*` (sabitler+`Durum`/`Bot`/`Sohbet`/`Mesaj`/`DusunmeKip`),
`metin_*` (saf metin/protokol yardımcıları, `Cevap`, `cevap_parcala`), `saglayici_*` (AI çağrı
katmanı — altı ayrı `impl Bot` bloğuna bölündü, `sor_ham`'dan `gonder_cevap`'a), `sohbet_*`
(`cevapla` döngüsü, `arastir`), `dongu_*` (gelişim+hafıza tarama+arka plan döngüleri+eylemler),
`handler_event.rs`+`handler_dugmeler.rs` (discord olayları — `impl EventHandler for Handler` tek
trait impl olmak zorunda olduğu için 423 satırda kaldı, Rust E0119 kısıtı), `kurulum.rs`
(`Bot::kur`, başlangıç), `testler_*` (main.rs'in eski test modülü). `src/komut/` grupları:
`kayit_*` (kayıt tablosu+yardımcılar), `kartlar.rs`/`eylemler.rs`/`ayarlar.rs` (komut gövdeleri),
`kalan.rs` (`uyandir`/`uyut`/`debug_ayarla`/`model_var_mi`).

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
