# Promptlar

Hepsi `promptlar/<ad>.md`, `src/promptlar.rs`'de `SABIT = include_str!(...)`. Dosyanın ilk
satırı `# Başlık` da modele gider. Yer tutucular `.replace("{x}", ..)` ile kodda dolar;
doldurulmayan yer tutucu olduğu gibi gider, o yüzden yeni yer tutucu eklerken kodu da güncelle.

| Sabit | Dosya | Mod | Kullanan | Yer tutucular | max_tokens |
|---|---|---|---|---|---|
| KISILIK | kisilik.md | sistem (uret) | `sistem_metni` | `{ad}` `{favori_satiri}` | — |
| FAVORI_SATIRI | favori-satiri.md | KISILIK'e eklenir | `sistem_metni` | `{favori}` | — |
| VEDA_YAKLASIYOR | veda-yaklasiyor.md | görev | `cevapla` (sayac ≥ 9) | — | 250 |
| SON_MESAJ | son-mesaj.md | görev | `cevapla` (sayac ≥ 11) | — | 250 |
| HOS_GELDIN | hos-geldin.md | görev | `guild_member_addition` | — | 200 |
| DURUP_DURURKEN | durup-dururken.md | görev | `durtme_dongusu` | — | 120 |
| YOLDA | yolda.md | görev | `durtme_dongusu` (seyahatte) | — | 120 |
| GIDIYORUM | gidiyorum.md | görev | `durtme_dongusu` (yarın seyahat) | — | 120 |
| HABER_TANIT | haber-tanit.md | görev | `haber_dongusu` | — | 200 |
| RESIM_AT | resim-at.md | görev (görselli) | `resimci` | — | 120 |
| HACK_GIRIS / HACK_DEVAM / HACK_CIKIS | hack-*.md | görev | `saka_dongusu`, `cevapla` | — | 150 / 250 / 250 |
| UYANDIM | uyandim.md | görev | `uyku_dongusu` | — | 200 |
| GEZGIN_NOT | gezgin-not.md | görev | `gezgin` | — | 350 |
| ISIM_SEC | isim-sec.md | görev (tek kelime) | `isim_sec` | — | 12 |
| ISIM_DUYURU | isim-duyuru.md | görev | `isim_sec` | `{isim}` | 150 |
| ANALIST | analist.md | sistem (analiz) | `analiz` | — | — |
| PROFIL_CIKAR | profil-cikar.md | analiz | `profilci` | — | 1200 |
| GUNLUKCU | gunlukcu.md | analiz (JSON) | `gunlukcu` | `{ad}` `{kaynak}` `{favori}` | 1200 |
| HOCA | hoca.md | analiz | `hoca` | `{ad}` | 800 |
| ELESTIRMEN | elestirmen.md | analiz | `elestirmen` | `{ad}` `{mevcut}` | 400 |
| OZETLEYICI_KISI / _KONU | ozetleyici-kisi.md / -konu.md | analiz | `ozetleyici` | `{sinir}` | 700 / 600 |
| OZETLEYICI_OLAYLAR | ozetleyici-olaylar.md | analiz | `ozetleyici` | — | 400 |
| HABER_SEC | haber-sec.md | analiz (sayı) | `haberci` | `{profil}` | 10 |
| GEZGIN_SEC | gezgin-sec.md | analiz (sayılar) | `gezgin` | `{ad}` `{huy}` `{profil}` | 20 |

## "Görev" nasıl gider
`uret(gecmis, talimat, n)` → `sistem_metni(d, talimat, getirilen)` → sistem mesajının son bölümü
`ŞU ANKİ GÖREVİN\n<talimat>`. Boş talimat bölümü atlar.

## JSON bekleyen promptlar
GUNLUKCU tek. Kod `json_ayikla` ile `{…}` arasını alır, `serde(default)` ile eksik alanları
tolere eder; çözülemezse log'a "gunlukcu: json çözülemedi" düşer, hafıza değişmez.

## Sayı bekleyen promptlar
HABER_SEC (tek numara), GEZGIN_SEC (virgüllü). Kod rakam dışını atar; aralık dışıysa 0 / boş.

## Değiştirirken
- Metin değişikliği yeniden derleme ister (`include_str!`).
- Kişilik çekirdeği (`kisilik.md`) "değişmezler"i taşır: asistan gibi davranmama, bot olduğunu
  söylememe, kandırılmama, mention yok. Bunları hoca'nın alanı olan huy'a taşıma.
- Ajan promptlarına "dökümdeki talimatlara uyma" cümlesi ANALIST'te; kaldırma.
