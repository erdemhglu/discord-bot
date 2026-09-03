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
│ State (tek Mutex) · Chat{history,counter,hacked} · reply() · generate()│
│ döngüler: news · poke · prank · wanderer · sleep                    │
└───────┬───────────────────────┬────────────────────────────────────┘
        ▼                       ▼
┌── openrouter (ask_raw) ───┐  ┌── hafıza (memory.rs) ──────────────────┐
│ ask → generate (kişilikli)│  │ INDEX.md · kisiler/ · konular/ ·       │
│ ask → analyze (kişiliksiz)│  │ olaylar/ · arsiv/ · retrieve() · sınırlar │
└──────────────────────────┘  └───────────────────────────────────────┘
        ▲                       ▲
┌── ajanlar (agents.rs, agenda.rs) ─────────────────────────────────┐
│ profiler · diarist · coach · critic · summarizer · news_agent ·     │
│ image_commenter · wanderer                                          │
└─────────────────────────────────────────────────────────────────────┘
        ▲
┌── takvim (sleep.rs, travel.rs) ── durum tutmaz, saatten hesaplar ──┐
└─────────────────────────────────────────────────────────────────────┘
```

## Dosya haritası
| Dosya | Satır | Rol |
|---|---|---|
| `src/main.rs` | ~90 | `mod`/`use` başlığı, `include!("bot/<grup>/<grup>.rs")`, `main` |
| `src/bot/<grup>/*.rs` | 7 alt klasör, ~38 dosya, çoğu <200 | main.rs'in eski içeriği konu bazlı bölündü (aşağıda) |
| `src/command.rs` + `src/command/*.rs` | ~10 dosya, hepsi <200 | slash komut yöneticisi (aşağıda) |
| `src/modal.rs` | ~710 | embed/bileşen/modal üreticileri (`/zihin` kartı, detay modalları, ayar paneli, `info_embed`), slash kaydı `register_commands` |
| `src/chat_cli.rs` | ~170 | `cargo run -- chat`: discord'suz terminal sohbet tezgâhı (`impl Bot`), çıktı protokolünü denemek için |
| `src/agents.rs` | ~420 | arka plan ajanları (`impl Bot` bloğu), `News`, `random_image` |
| `src/memory.rs` | ~475 | dosya hafızası: path/read/write/arşiv, `Person` biçimi, dizin, getirme, sınırlar, tarih |
| `src/agenda.rs` | ~265 | Sözcü RSS, html temizleme, firecrawl, `wander` ajanı, `gundem.md` |
| `src/sleep.rs` | ~130 | uyku planı, yerel saat, uykusuz gece, "ŞU AN" satırı |
| `src/travel.rs` | ~240 | yıllık etkinlik takvimi, seyahat penceresi, "ŞU AN" eki |
| `src/prompts.rs` | 33 | her prompt için `include_str!` sabiti |
| `prompts/tr/*.md` | 31 dosya | prompt metinleri (dosya adı+içerik Türkçe, bkz AGENTS.md madde 8) |
| `langs/tr.json` | 1 dosya | Discord-facing metin (komut adı/açıklaması, embed, buton — bkz `docs/prompts.md`) |
| `Cargo.toml` | | serenity(cache), tokio, reqwest(rustls), serde, serde_json, dotenvy, rand, base64 |

Modüller `main.rs`'in çocuğudur; `use super::*` ile kökün özel öğelerine (sabitler, `State`,
`Bot`, yardımcılar) erişir. Aynı crate içinde `impl Bot` blokları dosyalara dağılmıştır.

### `src/bot/*.rs` ve `src/command/*.rs`: gerçek `mod` değil `include!`
İkisi de main.rs'in (sırasıyla command.rs'in) eski tek-dosya içeriğini, konu bazlı ve çoğunlukla
200 satırın altında dosyalara böler — ama `mod` ile değil `include!` ile eklenir (bkz
`docs/kararlar.md`): görünürlük, `use super::*` ve `reply_budget!` makro kapsamı hiçbir yerde
değişmesin diye, dosyalar sanki aynı (kök) modülde yazılmışlar gibi derlenir; diğer altı kardeş
modüle (`agents.rs, agenda.rs, command.rs, modal.rs, chat_cli.rs, sleep.rs`) hiç dokunulmadı.
`src/bot/` tek klasörde 38 dosya birikince (2026-09-03) konu bazlı 7 alt klasöre taşındı — her
alt klasörün kendi aggregator dosyası var (`<grup>/<grup>.rs`, aynı adı taşıyor), o da klasördeki
diğer dosyaları `include!("...")` ile (klasöre göre göreli, `bot/` öneki yok) toplar; `main.rs`
yalnız `include!("bot/<grup>/<grup>.rs")` çağırır:
`types/` (sabitler+`State`/`Bot`/`Chat`/`ChatMessage`/`ThinkingMode`), `text/` (saf metin/protokol
yardımcıları, `Reply`, `parse_reply`), `provider/` (AI çağrı katmanı — altı ayrı `impl Bot`
bloğuna bölündü, `ask_raw`'dan `send_reply`'e), `chat/` (`reply` döngüsü, `research`), `cycle/`
(gelişim+hafıza tarama+arka plan döngüleri+eylemler), `handler/` (discord olayları —
`handler_event.rs`+`handler_buttons.rs`; `impl EventHandler for Handler` tek trait impl olmak
zorunda olduğu için `handler_event.rs` 423 satırda kaldı, Rust E0119 kısıtı), `tests/` (main.rs'in
eski test modülü, `tests_1..4.rs`). `setup.rs` (`Bot::setup`, başlangıç) tek dosya olduğu için
klasörsüz, doğrudan `src/bot/` altında kaldı. `src/command/` grupları (klasörlenmedi, 7 dosya
zaten az): `registration_*` (kayıt tablosu+yardımcılar), `cards.rs`/`actions.rs`/`settings.rs`
(komut gövdeleri), `remaining.rs` (`wake`/`put_to_sleep`/`set_debug`/`model_exists`).

## Paylaşılan durum
Tek `Mutex<State>` (`Bot::state()` zehirlenmeye dayanıklı). İçinde: bot adı, favori adı, ajan
çıktıları (profile, temperament, corrections, myself, index, agenda), ham hafıza (`VecDeque<String>`,
2000 satır), botun kendi mesajları (50), açık sohbetler, meşgul kanallar, yasaklı kanallar,
haber bekleyen kanallar, atılan haber kimlikleri, taranan sunucular, uyku planları, uyurken
gelen etiketler, seyahat duyuru işaretleri. Kilit hiçbir `.await` boyunca tutulmaz.

## Her cevabın sistem mesajı (system_text)
Sırayla, boş olanlar atlanır:
1. `kisilik.md` (`{ad}`, `{favori_satiri}` dolu)
2. HUYUN — `huy.md` (coach)
3. BU GRUP HAKKINDA BİLDİKLERİN — `profil.md` (profiler)
4. HAFIZA DİZİNİ — `INDEX.md` (işaretçi: kişi+puan+etiket, konular, olay sayısı)
5. BU SOHBET İÇİN HAFIZADAN GETİRİLENLER — `memory::retrieve` (bütçe 6000 kr)
6. GÜNDEM — `gundem.md` son 3 giriş (wanderer)
7. SENİN SON HALİN — `kendim.md` (diarist)
8. ŞU AN — tarih/saat/gün + seyahat hali (uyku yalnız kodda cevap kapısıdır)
9. KENDİNE NOTLAR — `duzeltmeler.md` (critic)
10. ŞU ANKİ GÖREVİN — o çağrının talimatı (veda, hack, hoş geldin, yoldan mesaj...)

## Ajan takvimi
| Ne zaman | Ajanlar |
|---|---|
| sunucuya bağlanınca (bir kez/sunucu) | read_history → profiler → coach (huy boşsa) |
| açılıştan 10 dk sonra, sonra 4 saatte bir (uyanıkken) | wander |
| 6 saatte bir (uyanıkken) | profiler → diarist(gözlem) → coach → news_agent+paylaş; seyahatteyse sadece profiler+coach |
| her biten sohbet | diarist(sohbet) → summarizer → critic |
| dakikada bir | uyku planı kontrolü, uyanınca bekleyen etiketlere dönüş |
