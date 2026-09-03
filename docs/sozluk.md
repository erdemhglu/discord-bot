# Sözlük (Türkçe kalan çalışma zamanı terimleri)

Kod tarafındaki tanımlayıcılar İngilizce (bkz AGENTS.md madde 8). Bu sözlük artık bir kod
sözlüğü değil: botun **Türkçe kalan** yüzeyini (promptlar, `durum/` dosya alanları, Discord'a
çıkan kategori etiketleri, slash komut adları) İngilizce okuyan bir geliştiriciye açıklar.

| Türkçe terim | Anlamı / nerede geçer |
|---|---|
| durum | state — hem paylaşılan `State` yapısının eski adı hem `durum/` klasörünün adı |
| hafiza | memory — hem `src/memory.rs`'in eski adı hem "hafıza" kavramı |
| sohbet | chat — bir kanaldaki açık konuşma |
| gecmis | history — bir sohbetin mesajları (`Chat.history`) |
| sayac | counter — botun o sohbette yazdığı mesaj sayısı |
| hackli | "hacked" — hack şakasında kalan tur sayısı |
| mesgul | busy — kanalda şu an cevap üretiliyor |
| yasakli | banned — (artık kullanılmıyor, kanal yasağı kaldırıldı) |
| profil | group profile — profiler ajanının ürettiği `profil.md` |
| huy | temperament — coach ajanının ürettiği `huy.md`, botun gelişen kişiliği |
| duzeltmeler | corrections — critic ajanının `duzeltmeler.md`'si |
| kendim | "myself" — diarist ajanının `kendim.md`'si, botun kendi anlık hali |
| gundem | agenda — wanderer ajanının internette gezip yazdığı görüşler (`gundem.md`) |
| planlar / uyuyor | sleep plans / is asleep |
| son_yol_mesaji / duyurulan_seyahat | last on-the-road message day / announced trip (eski alan adları, şimdi `last_road_message`/`announced_trip`) |
| gezgin | wanderer — internette gezen ajan (kod adı artık `wander`/`wanderer_cycle`) |
| haberci | news agent — kod adı artık `news_agent` |
| profilci | profiler — kategori etiketi olarak Discord'a çıkan `!durum` kırılımında hâlâ görünür |
| gunlukcu | diarist — sohbeti hafızaya işleyen ajan; kategori etiketi olarak kalır |
| hoca | coach — kişiliği şekillendiren ajan; kategori etiketi olarak kalır |
| elestirmen | critic — kategori etiketi olarak kalır |
| ozetleyici | summarizer/compactor ajan |
| resimci | image commenter ajan; kategori etiketi olarak kalır |
| kisi / kisiler | person / people — `kisiler/<id>.md` |
| konu / konular | topic(s) — `konular/<slug>.md` |
| olay / olaylar | event(s) — `olaylar/YYYY-AA.md` |
| arsiv / arsivle | archive |
| kanaat / puan / not | opinion / score / note — kişi dosyası alanları |
| bilgiler / etiket | facts / tags — kişi dosyası alanları |
| tarih / ay / saat | date / month / time |
| uyku / uyanik_mi / uykusuz | sleep / awake? / sleepless |
| gergin | tense — kişiliğe bağlı uykusuzluk tetikleyicisi |
| seyahat / yolda / gidiyorum | travel / on the road / I'm leaving |
| etkinlik | event (takvim) |
| yer / sebep | place / reason |
| favori | favorite — hep sevilen kullanıcı |
| ayar | setting — env değişkeni ya da `/ayarlar` paneli |
| talimat | instruction — bir çağrının görev metni |
| kaynak | source |
| gelisim / evre / hak edilen | growth / stage / earned stage |
| dogum | birth — ilk çalıştığı an (unix zaman damgası) |

## Model JSON çıktısındaki Türkçe alan adları (bilerek çevrilmedi)
Bu alanlar Rust struct'larında da aynen Türkçe: promptlar Türkçe olduğu için model bu adlarla
JSON üretir, serde alan adı eşleşmesine bakar — çevrilseydi sessizce boş/0 dönerdi.
- İsteklilik (`WILLINGNESS`): `puan` (0-10 puan), `sebep` (tek cümle gerekçe)
- Hedef seçimi (`TARGET_PICK`): `hedef` (kişi adı)
- Ruh hali (`MOOD`): `durum` (ör. "kafa karışıklığı"), `yogunluk` (1-10)
- Uyanış (`WAKING`): `ilgi` (0-10), `konu`
- Günlükçü (`DIARIST`): `olay`, `kisiler[].isim/puan_degisimi/not/bilgiler/etiketler`,
  `konular[].ad/not`, `kendim`

## Discord'a çıkan Türkçe yüzey
Slash komut adları (`durum, yardim, zihin, ayarlar, sifirla, haber, sorun, gez, saka, hack,
ajanlar, uyan, uyu, dusunme, model, debug`), seçenek adları/etiketleri, embed başlık/alan
metinleri, buton/menü etiketleri, ve elbette modelin ürettiği her cevap — hepsi Türkçe kalır
(bkz AGENTS.md madde 8, README.md "commands"). `!durum` kırılımındaki kategori etiketleri de
(`sohbet, isteklilik, profilci, gunlukcu, hoca, elestirmen, ozetleyici_*, haber_sec, hedef_sec,
ruh_hali, uyanis, uyandim, isim_sec, hack_giris, sorun, laf, gozlem`) bilerek Türkçe/kod-içi
tutuldu — hem Discord'a çıkıyorlar hem de değiştirmek geriye dönük metrik kırılımını bozardı.
