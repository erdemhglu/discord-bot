// Komut arayüzü: slash komutlar embed kartlarıyla açılır (web sayfası gibi
// bölümlü, okunaklı), detaylar etiketli modal alanlarına dağıtılır — tek
// metin kutusuna her şey boca edilmez.

use super::*;

const MODAL_SINIR: usize = 4000; // discord TextInput value üst sınırı
const ALAN_SINIRI: usize = 1024; // discord embed field value üst sınırı
const ETIKET_SINIRI: usize = 45; // modal başlık/etiket, menü seçenek etiketi
const ACIKLAMA_SINIRI: usize = 100; // menü seçenek açıklaması
const KISI_MENU_SINIRI: usize = 25; // discord select menü seçenek üst sınırı
const KIRPMA_NOTU: &str = "\n… (sığmadı, kırpıldı)";

// bileşen kimlikleri
pub const ZIHIN_KISI_SEC: &str = "zihin_kisi_sec";
pub const ZIHIN_KONULAR: &str = "zihin_konular";
pub const ZIHIN_OLAYLAR: &str = "zihin_olaylar";
pub const ZIHIN_OZET: &str = "zihin_ozet";
// ayar paneli: düşünme kipi butonlarının kimliği "ayar_dusunme:<kip dosya değeri>"
pub const AYAR_DUSUNME: &str = "ayar_dusunme:";
pub const AYAR_DEBUG: &str = "ayar_debug";
pub const AYAR_UYAN: &str = "ayar_uyan";
pub const AYAR_UYU: &str = "ayar_uyu";

// embed vurgu renkleri
const RENK_ZIHIN: u32 = 0x5865F2;
const RENK_DURUM: u32 = 0x57F287;
const RENK_YARDIM: u32 = 0xEB459E;
const RENK_AYAR: u32 = 0xFEE75C;

pub struct Bolum {
    pub etiket: String,
    pub custom_id: String,
    pub icerik: String,
}

impl Bolum {
    fn new(etiket: impl Into<String>, custom_id: impl Into<String>, icerik: String) -> Self {
        Self {
            etiket: etiket.into(),
            custom_id: custom_id.into(),
            icerik: sigdir(&icerik, MODAL_SINIR),
        }
    }
}

// sınır aşımı son satır/boşluk hizasında kesilir + not; gövde hep sınıra sığar
fn sigdir(metin: &str, sinir: usize) -> String {
    if metin.chars().count() <= sinir {
        return metin.to_string();
    }
    let mut s: String = metin
        .chars()
        .take(sinir - KIRPMA_NOTU.chars().count())
        .collect();
    if let Some(son) = s.rfind(['\n', ' ']) {
        s.truncate(son);
    }
    s.push_str(KIRPMA_NOTU);
    s
}

// "2026-09" → "Eylül 2026"; çözülemezse girdi aynen döner
fn ay_adi(ay: &str) -> String {
    const AYLAR: [&str; 12] = [
        "Ocak", "Şubat", "Mart", "Nisan", "Mayıs", "Haziran", "Temmuz", "Ağustos", "Eylül", "Ekim",
        "Kasım", "Aralık",
    ];
    let mut parca = ay.splitn(2, '-');
    match (parca.next(), parca.next()) {
        (Some(yil), Some(a)) if (1..=12).contains(&a.parse::<usize>().unwrap_or(0)) => {
            format!("{} {}", AYLAR[a.parse::<usize>().unwrap_or(1) - 1], yil)
        }
        _ => ay.to_string(),
    }
}

// ---------- genel durum satırları (!durum ve /durum ortak) ----------

pub fn durum_metni(d: &Durum) -> String {
    let g = &d.gelisim;
    let m = &d.metrik;
    let ozet = format!(
        "sürüm: {} · evre: {} ({}. gün, {} sohbet, {} mesaj) · model: {} · {} · düşünme: {} · debug: {} · seyahat: {} · token: {} çağrı, {} giriş ({} önbellek) + {} çıkış",
        surum_metni(),
        gelisim::evre(g).ad,
        gelisim::gun(g) + 1,
        g.sohbet,
        g.mesaj,
        d.model,
        if uyku::uyanik_mi(d) { "uyanık" } else { "uyuyor" },
        d.dusunme.ad(),
        if d.debug { "açık" } else { "kapalı" },
        seyahat::simdi().map(|s| s.yer).unwrap_or("yok"),
        m.cagri,
        m.giris_token,
        m.onbellek_token,
        m.cikis_token,
    );
    if m.kategoriler.is_empty() {
        return ozet;
    }
    format!("{ozet}\ntoken kırılımı: {}", token_kirilimi(m))
}

// kategoriler toplam tokene göre sıralı: "sohbet: 120 giriş + 80 çıkış · ..."
fn token_kirilimi(m: &Metrik) -> String {
    use std::fmt::Write as _;
    let mut sirali: Vec<(&'static str, &Kullanim)> =
        m.kategoriler.iter().map(|(ad, k)| (*ad, k)).collect();
    sirali.sort_by_key(|(_, k)| std::cmp::Reverse(k.prompt_tokens + k.completion_tokens));
    let mut satirlar = String::new();
    for (i, (ad, k)) in sirali.iter().enumerate() {
        if i > 0 {
            satirlar.push_str(" · ");
        }
        let _ = write!(
            satirlar,
            "{ad}: {} giriş + {} çıkış",
            k.prompt_tokens, k.completion_tokens
        );
    }
    satirlar
}

// ---------- /zihin: embed kart + menü/buton ----------

// zihin kartı: üç sütun (kişiler / konular / olaylar) + alt bilgide sayaçlar
pub fn zihin_embedleri(d: &Durum) -> Vec<CreateEmbed> {
    let kisiler = hafiza::kisi_dokumleri();
    let konular = hafiza::konu_dokumleri();
    let olaylar = hafiza::olay_aylari(3);
    let olay_sayisi: usize = olaylar.iter().map(|(_, s)| s.len()).sum();

    let mut kisi_satirlari = String::new();
    for k in kisiler.iter().take(8) {
        kisi_satirlari += &format!("**{}** ({:+})", k.isim, k.puan);
        if !k.etiket.is_empty() {
            kisi_satirlari += &format!(" · {}", k.etiket.join(", "));
        }
        kisi_satirlari.push('\n');
    }

    let mut konu_satirlari = String::new();
    for (ad, son) in konular.iter().take(8) {
        konu_satirlari += &format!("**{ad}**");
        if !son.is_empty() {
            konu_satirlari += &format!(" · son: {son}");
        }
        konu_satirlari.push('\n');
    }

    // en yeni olaylar: en yeni aydan geriye doğru her ayın sonu; gösterim
    // kronolojik olsun diye eski aylar öne alınır
    let mut parcalar: Vec<Vec<&str>> = Vec::new();
    let mut toplam = 0usize;
    for (_, s) in olaylar.iter() {
        if toplam >= 5 {
            break;
        }
        let alinacak = (5 - toplam).min(s.len());
        toplam += alinacak;
        let n = s.len();
        parcalar.push(
            s.iter()
                .skip(n.saturating_sub(alinacak))
                .map(|x| x.as_str())
                .collect(),
        );
    }
    parcalar.reverse();
    let mut olay_satirlari = String::new();
    for parca in parcalar {
        for o in parca {
            // "- tarih saat #kanal: metin" → okunur tek satır
            olay_satirlari += &hafiza::kirp(o.trim_start_matches("- "), 90);
            olay_satirlari.push('\n');
        }
    }

    let g = &d.gelisim;
    vec![CreateEmbed::new()
        .title("Zihin")
        .color(RENK_ZIHIN)
        .description(format!(
            "{} · {}. gün · {} · {}",
            gelisim::evre(g).ad,
            gelisim::gun(g) + 1,
            d.model,
            d.dusunme.ad(),
        ))
        .field(
            format!("Kişiler ({})", kisiler.len()),
            bos_yoksa(&kisi_satirlari),
            true,
        )
        .field(
            format!("Konular ({})", konular.len()),
            bos_yoksa(&konu_satirlari),
            true,
        )
        .field(
            format!("Olaylar ({olay_sayisi})"),
            bos_yoksa(&olay_satirlari),
            true,
        )
        .footer(CreateEmbedFooter::new(format!(
            "{} · {}",
            d.bot_adi,
            hafiza::tarih()
        )))]
}

// boş bölme kartta "—" olarak görünür; silik ama yapı bozulmaz
fn bos_yoksa(s: &str) -> String {
    let t = s.trim();
    if t.is_empty() {
        "—".to_string()
    } else {
        sigdir(t, ALAN_SINIRI)
    }
}

// kişi detay menüsü: son değişenler, etiket ad + puan, açıklama etiketler/not
fn kisi_secenekleri() -> Vec<CreateSelectMenuOption> {
    hafiza::kisi_dokumleri()
        .into_iter()
        .take(KISI_MENU_SINIRI)
        .map(|k| {
            let mut aciklama: Vec<String> = Vec::new();
            if !k.etiket.is_empty() {
                aciklama.push(k.etiket.join(", "));
            }
            if !k.not.is_empty() {
                aciklama.push(k.not.clone());
            }
            CreateSelectMenuOption::new(
                hafiza::kirp(&format!("{} ({:+})", k.isim, k.puan), ETIKET_SINIRI),
                k.id.to_string(),
            )
            .description(hafiza::kirp(&aciklama.join(" · "), ACIKLAMA_SINIRI))
        })
        .collect()
}

pub fn zihin_bilesenleri() -> Vec<CreateActionRow> {
    let mut satirlar: Vec<CreateActionRow> = Vec::new();
    let secenekler = kisi_secenekleri();
    if !secenekler.is_empty() {
        satirlar.push(CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                ZIHIN_KISI_SEC,
                CreateSelectMenuKind::String {
                    options: secenekler,
                },
            )
            .placeholder("Kişi detayı seç…"),
        ));
    }
    satirlar.push(CreateActionRow::Buttons(vec![
        CreateButton::new(ZIHIN_KONULAR)
            .label("Konular")
            .style(ButtonStyle::Secondary),
        CreateButton::new(ZIHIN_OLAYLAR)
            .label("Olaylar")
            .style(ButtonStyle::Secondary),
        CreateButton::new(ZIHIN_OZET)
            .label("Bot özeti")
            .style(ButtonStyle::Secondary),
    ]));
    satirlar
}

pub fn zihin_mesaji(d: &Durum) -> CreateInteractionResponseMessage {
    CreateInteractionResponseMessage::new()
        .ephemeral(true)
        .embeds(zihin_embedleri(d))
        .components(zihin_bilesenleri())
}

// ---------- detay modalları: her konu kendi etiketli alanında ----------

fn modal_olustur(baslik: &str, custom_id: &str, bolumler: Vec<Bolum>) -> CreateModal {
    let dolu: Vec<Bolum> = bolumler
        .into_iter()
        .filter(|b| !b.icerik.trim().is_empty())
        .collect();
    let satirlar = if dolu.is_empty() {
        vec![CreateActionRow::InputText(
            CreateInputText::new(
                InputTextStyle::Paragraph,
                "Durum",
                format!("{custom_id}_bos"),
            )
            .value("(henüz boş)")
            .required(false),
        )]
    } else {
        dolu.into_iter()
            .map(|b| {
                CreateActionRow::InputText(
                    CreateInputText::new(
                        InputTextStyle::Paragraph,
                        hafiza::kirp(&b.etiket, ETIKET_SINIRI),
                        b.custom_id,
                    )
                    .value(b.icerik)
                    .required(false),
                )
            })
            .collect()
    };
    CreateModal::new(
        custom_id,
        baslik.chars().take(ETIKET_SINIRI).collect::<String>(),
    )
    .components(satirlar)
}

// kişi kartı: kimlik / izlenim / etiketler / bildikleri / son olaylar ayrı alanlarda
pub fn modal_kisi(id: u64) -> CreateModal {
    let k = hafiza::kisi_oku(id);
    let baslik = if k.isim.is_empty() {
        "bilinmeyen".to_string()
    } else {
        k.isim.clone()
    };
    let mut bolumler: Vec<Bolum> = Vec::new();

    let mut kimlik = format!("{}\nid: {}", k.isim, k.id);
    if !k.kullanici_adi.is_empty() {
        kimlik += &format!("\nkullanıcı adı: {}", k.kullanici_adi);
    }
    if !k.eski_adlar.is_empty() {
        kimlik += &format!("\nönceki adları: {}", k.eski_adlar.join(", "));
    }
    bolumler.push(Bolum::new("Kimlik", "kisi_kimlik", kimlik));

    let mut izlenim = format!("puan: {:+}", k.puan);
    if !k.not.is_empty() {
        izlenim += &format!("\n{}", k.not);
    }
    bolumler.push(Bolum::new("İzlenim", "kisi_izlenim", izlenim));

    if !k.etiket.is_empty() {
        bolumler.push(Bolum::new(
            "Etiketler",
            "kisi_etiketler",
            k.etiket.join(" · "),
        ));
    }
    if !k.bilgiler.is_empty() {
        let n = k.bilgiler.len();
        let liste: Vec<&str> = k
            .bilgiler
            .iter()
            .skip(n.saturating_sub(8))
            .map(|s| s.as_str())
            .collect();
        bolumler.push(Bolum::new("Bildikleri", "kisi_bilgiler", liste.join("\n")));
    }
    if !k.olaylar.is_empty() {
        let n = k.olaylar.len();
        let liste: Vec<&str> = k
            .olaylar
            .iter()
            .skip(n.saturating_sub(5))
            .map(|s| s.as_str())
            .collect();
        bolumler.push(Bolum::new("Son olaylar", "kisi_olaylar", liste.join("\n")));
    }
    modal_olustur(&baslik, &format!("zihin_kisi_{id}"), bolumler)
}

// konular: son değişenler notlarıyla, kalanı ad listesi
pub fn modal_konular() -> CreateModal {
    let konular = hafiza::konu_dokumleri();
    let mut bolumler: Vec<Bolum> = Vec::new();
    let son: Vec<String> = konular
        .iter()
        .take(15)
        .map(|(ad, not)| {
            if not.is_empty() {
                format!("- {ad}")
            } else {
                format!("- {ad} · son: {not}")
            }
        })
        .collect();
    bolumler.push(Bolum::new("Son değişenler", "konular_son", son.join("\n")));
    if konular.len() > 15 {
        let diger: Vec<&str> = konular[15..].iter().map(|(ad, _)| ad.as_str()).collect();
        bolumler.push(Bolum::new(
            "Diğer konular",
            "konular_diger",
            diger.join(" · "),
        ));
    }
    modal_olustur("Konular", "modal_konular", bolumler)
}

// olaylar: ay başına bir alan, her ayın son kayıtları
pub fn modal_olaylar() -> CreateModal {
    let mut bolumler: Vec<Bolum> = Vec::new();
    for (ay, satirlar) in hafiza::olay_aylari(3) {
        if satirlar.is_empty() {
            continue;
        }
        let n = satirlar.len();
        let goster: Vec<&str> = satirlar
            .iter()
            .skip(n.saturating_sub(10))
            .map(|s| s.as_str())
            .collect();
        bolumler.push(Bolum::new(
            ay_adi(&ay),
            format!("olaylar_{ay}"),
            goster.join("\n"),
        ));
    }
    modal_olustur("Olaylar", "modal_olaylar", bolumler)
}

// bot özeti: durum / token / kendim / gündem ayrı alanlarda
pub fn modal_ozet(d: &Durum) -> CreateModal {
    let g = &d.gelisim;
    let m = &d.metrik;
    let durum = format!(
        "evre: {} ({}. gün)\nsohbet: {} · mesaj: {}\nmodel: {}\nuyku: {} · düşünme: {}\nseyahat: {}",
        gelisim::evre(g).ad,
        gelisim::gun(g) + 1,
        g.sohbet,
        g.mesaj,
        d.model,
        if uyku::uyanik_mi(d) { "uyanık" } else { "uyuyor" },
        d.dusunme.ad(),
        seyahat::simdi().map(|s| s.yer).unwrap_or("yok"),
    );
    let mut token = format!(
        "{} çağrı · {} giriş ({} önbellek) + {} çıkış",
        m.cagri, m.giris_token, m.onbellek_token, m.cikis_token
    );
    if !m.kategoriler.is_empty() {
        token += &format!("\nkırılım: {}", token_kirilimi(m));
    }
    let mut bolumler = vec![
        Bolum::new("Durum", "ozet_durum", durum),
        Bolum::new("Token", "ozet_token", token),
    ];
    if !d.kendim.trim().is_empty() {
        let son: Vec<&str> = d.kendim.lines().rev().take(4).collect();
        bolumler.push(Bolum::new(
            "Kendim",
            "ozet_kendim",
            son.into_iter().rev().collect::<Vec<_>>().join("\n"),
        ));
    }
    if !d.gundem.trim().is_empty() {
        bolumler.push(Bolum::new(
            "Gündem",
            "ozet_gundem",
            hafiza::kirp(&d.gundem, 1000),
        ));
    }
    modal_olustur("Bot özeti", "modal_ozet", bolumler)
}

// ---------- /durum ve /yardim ----------

pub fn durum_mesaji(d: &Durum) -> CreateInteractionResponseMessage {
    let m = &d.metrik;
    let g = &d.gelisim;
    let mut e = CreateEmbed::new()
        .title("Durum")
        .color(RENK_DURUM)
        .field(
            "Genel",
            format!(
                "sürüm: {}\nevre: {} ({}. gün)\nsohbet: {} · mesaj: {}\nmodel: {}",
                surum_metni(),
                gelisim::evre(g).ad,
                gelisim::gun(g) + 1,
                g.sohbet,
                g.mesaj,
                d.model,
            ),
            true,
        )
        .field(
            "Hal",
            format!(
                "uyku: {}\ndüşünme: {}\ndebug: {}\nseyahat: {}",
                if uyku::uyanik_mi(d) {
                    "uyanık"
                } else {
                    "uyuyor"
                },
                d.dusunme.ad(),
                if d.debug { "açık" } else { "kapalı" },
                seyahat::simdi().map(|s| s.yer).unwrap_or("yok"),
            ),
            true,
        )
        .field(
            "Token",
            format!(
                "{} çağrı · {} giriş ({} önbellek) + {} çıkış",
                m.cagri, m.giris_token, m.onbellek_token, m.cikis_token
            ),
            false,
        );
    if !m.kategoriler.is_empty() {
        e = e.field("Kırılım", sigdir(&token_kirilimi(m), ALAN_SINIRI), false);
    }
    CreateInteractionResponseMessage::new()
        .ephemeral(true)
        .embed(e)
}

pub fn yardim_mesaji() -> CreateInteractionResponseMessage {
    CreateInteractionResponseMessage::new()
        .ephemeral(true)
        .embed(
            CreateEmbed::new()
                .title("Yardım")
                .color(RENK_YARDIM)
                .description(komut::YARDIM)
                .field(
                    "Arayüz",
                    "/durum ve /zihin bu kartları açar; /zihin'deki menü ve butonlar detay modallarına götürür.",
                    false,
                ),
        )
}

// ---------- ayar paneli (butonlu) ----------

pub fn ayarlar_embed(d: &Durum) -> CreateEmbed {
    let uyku = if uyku::uyanik_mi(d) {
        if d.uyanik_zorla > simdi_unix() {
            "uyanık (zorla, !uyan)"
        } else {
            "uyanık"
        }
    } else {
        "uyuyor"
    };
    CreateEmbed::new()
        .title("Ayarlar")
        .color(RENK_AYAR)
        .description(format!(
            "sürüm: {}\nmodel: {} (`!model <id>`, yalnız favori)\ndüşünme: **{}**\ndebug: **{}**\nuyku: **{}**\nseyahat: {}",
            surum_metni(),
            d.model,
            d.dusunme.ad(),
            if d.debug { "açık" } else { "kapalı" },
            uyku,
            seyahat::simdi().map(|s| s.yer).unwrap_or("yok"),
        ))
        .footer(CreateEmbedFooter::new(
            "butona bas, panel yerinde yenilenir · göster: düşünce spoiler'da · gizle: düşünüyorum… · sessiz: iz yok · kapat: reasoning'siz",
        ))
}

pub fn ayarlar_bilesenleri(d: &Durum) -> Vec<CreateActionRow> {
    let kipler = [
        (DusunmeKip::Goster, "göster"),
        (DusunmeKip::Gizle, "gizle"),
        (DusunmeKip::Sessiz, "sessiz"),
        (DusunmeKip::Kapali, "kapat"),
    ];
    let dusunme: Vec<CreateButton> = kipler
        .iter()
        .map(|(kip, etiket)| {
            CreateButton::new(format!("{AYAR_DUSUNME}{}", kip.dosya_degeri()))
                .label(format!("düşünme: {etiket}"))
                .style(if *kip == d.dusunme {
                    ButtonStyle::Primary
                } else {
                    ButtonStyle::Secondary
                })
        })
        .collect();
    let uyanik = uyku::uyanik_mi(d);
    vec![
        CreateActionRow::Buttons(dusunme),
        CreateActionRow::Buttons(vec![
            CreateButton::new(AYAR_DEBUG)
                .label(if d.debug {
                    "debug: açık"
                } else {
                    "debug: kapalı"
                })
                .style(if d.debug {
                    ButtonStyle::Success
                } else {
                    ButtonStyle::Secondary
                }),
            CreateButton::new(AYAR_UYAN)
                .label("uyandır")
                .style(ButtonStyle::Secondary)
                .disabled(uyanik),
            CreateButton::new(AYAR_UYU)
                .label("uyut (8 saat)")
                .style(ButtonStyle::Secondary)
                .disabled(!uyanik),
        ]),
    ]
}

// /ayarlar (ephemeral) ve buton sonrası yerinde yenileme (UpdateMessage) aynı gövde
pub fn ayarlar_mesaji(d: &Durum, gizli: bool) -> CreateInteractionResponseMessage {
    CreateInteractionResponseMessage::new()
        .ephemeral(gizli)
        .embed(ayarlar_embed(d))
        .components(ayarlar_bilesenleri(d))
}

// ---------- slash kaydı ----------

// sunucu komutları: her ready'de çağrılır, discord üstüne yazar (idempotent);
// sunucu komutu anında görünür olur, global komut gecikmeli
pub async fn komutlari_kayit(http: &Http, guild: GuildId) -> Result<(), Hata> {
    guild
        .set_commands(
            http,
            vec![
                CreateCommand::new("durum")
                    .description("Botun şu anki halini kart olarak gösterir"),
                CreateCommand::new("yardim").description("Komut listesini kart olarak gösterir"),
                CreateCommand::new("zihin")
                    .description("Botun bildiklerini interaktif kart + menü/butonlarla gösterir"),
                CreateCommand::new("ayarlar")
                    .description("Butonlu ayar paneli: düşünme kipi, debug, uyku"),
            ],
        )
        .await?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn sigdir_siniri_asmaz() {
        let uzun = "kelime ".repeat(1000);
        let s = sigdir(&uzun, 200);
        assert!(s.chars().count() <= 200);
        assert!(s.contains("kırpıldı"));
        // kısa metin aynen kalır
        assert_eq!(sigdir("kısa", 200), "kısa");
    }

    #[test]
    fn ay_adi_cevirir() {
        assert_eq!(ay_adi("2026-09"), "Eylül 2026");
        assert_eq!(ay_adi("2026-01"), "Ocak 2026");
        assert_eq!(ay_adi("bozuk"), "bozuk");
        assert_eq!(ay_adi("2026-13"), "2026-13");
    }

    #[test]
    fn bolumler_bos_icerigi_alan_yapar() {
        // modal_olustur boş bölmeleri atlar; hepsi boşsa tek "(henüz boş)" alanı kalır.
        // CreateModal serileşmezliği yüzünden davranış Bolum filtresinden doğrulanır:
        let dolu: Vec<Bolum> = vec![
            Bolum::new("A", "a", String::new()),
            Bolum::new("B", "b", "veri".into()),
        ]
        .into_iter()
        .filter(|b| !b.icerik.trim().is_empty())
        .collect();
        assert_eq!(dolu.len(), 1);
        assert_eq!(dolu[0].etiket, "B");
    }

    #[test]
    fn durum_metni_sayac_tasir() {
        let d = Durum {
            model: "test-model".into(),
            gelisim: gelisim::Gelisim {
                mesaj: 7,
                ..Default::default()
            },
            ..Default::default()
        };
        let m = durum_metni(&d);
        assert!(m.contains("test-model"));
        assert!(m.contains("7 mesaj"));
    }

    #[test]
    fn token_kirilimi_sirali() {
        let mut m = Metrik::default();
        m.kategoriler.insert(
            "az",
            Kullanim {
                prompt_tokens: 10,
                completion_tokens: 5,
                ..Default::default()
            },
        );
        m.kategoriler.insert(
            "cok",
            Kullanim {
                prompt_tokens: 100,
                completion_tokens: 50,
                ..Default::default()
            },
        );
        let k = token_kirilimi(&m);
        // büyük kategori önde gelir
        assert!(k.find("cok").unwrap() < k.find("az").unwrap());
    }
}
