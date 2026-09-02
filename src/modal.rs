// Modal gösterimi: slash komutlar (/durum /yardim /zihin) modal açar.
// Zihin modalı 5 bölmeli (Discord sınırı): bot özeti, kişiler ikiye bölünmüş,
// konular, olaylar+gündem. İçerik 4000 karakteri aşarsa kırpılır + not düşülür.

use super::*;

const MODAL_SINIR: usize = 4000; // discord TextInput value üst sınırı
const KIRPMA_NOTU: &str = "\n… (sığmadı, kırpıldı)";

pub struct Bolum {
    pub etiket: &'static str, // slot etiketi (discord sınırı 45)
    pub custom_id: &'static str,
    pub icerik: String,
}

impl Bolum {
    fn new(etiket: &'static str, custom_id: &'static str, icerik: String) -> Self {
        Self {
            etiket,
            custom_id,
            icerik: sigdir(&icerik),
        }
    }
}

// discord'un 4000 sınırı; taşan son satır/boşluk hizasında kesilir + not
fn sigdir(metin: &str) -> String {
    if metin.chars().count() <= MODAL_SINIR {
        return metin.to_string();
    }
    let mut s: String = metin
        .chars()
        .take(MODAL_SINIR - KIRPMA_NOTU.chars().count())
        .collect();
    if let Some(son) = s.rfind(['\n', ' ']) {
        s.truncate(son);
    }
    s.push_str(KIRPMA_NOTU);
    s
}

// !durum mesaj komutu ile /durum modalı ortak kullanır
pub fn durum_metni(d: &Durum) -> String {
    let g = &d.gelisim;
    let m = &d.metrik;
    format!(
        "evre: {} ({}. gün, {} sohbet, {} mesaj) · model: {} · {} · düşünme: {} · seyahat: {} · token: {} çağrı, {} giriş + {} çıkış",
        gelisim::evre(g).ad,
        gelisim::gun(g) + 1,
        g.sohbet,
        g.mesaj,
        d.model,
        if uyku::uyanik_mi(d) { "uyanık" } else { "uyuyor" },
        d.dusunme.ad(),
        seyahat::simdi().map(|s| s.yer).unwrap_or("yok"),
        m.cagri,
        m.giris_token,
        m.cikis_token,
    )
}

// modal gösterimi için tek kişilik blok: ad + puan, etiketler, not, son bilgiler
fn kisi_gosterim(k: &hafiza::Kisi) -> String {
    let mut s = format!("{} ({:+})", k.isim, k.puan);
    if !k.etiket.is_empty() {
        s += &format!(" · {}", k.etiket.join(", "));
    }
    if !k.not.is_empty() {
        s += &format!("\nnot: {}", k.not);
    }
    let n = k.bilgiler.len();
    for b in k.bilgiler.iter().skip(n.saturating_sub(3)) {
        s += &format!("\n- {b}");
    }
    if let Some(o) = k.olaylar.last() {
        s += &format!("\nson olay: {o}");
    }
    s
}

// 5 slotu birleştirir (saf: test edilebilir). kişiler mtime sırasıyla gelir,
// ilk yarı "yakın" slotuna, kalanı "diğer" slotuna düşer
pub fn bolumler(
    ozet: String,
    kisiler: Vec<String>,
    konular: String,
    olaylar: String,
) -> Vec<Bolum> {
    let yarim = kisiler.len().div_ceil(2);
    let kisi1 = kisiler[..yarim].join("\n\n");
    let kisi2 = kisiler[yarim..].join("\n\n");
    vec![
        Bolum::new("Bot özeti", "zihin_ozet", ozet),
        Bolum::new("Kişiler (yakın)", "zihin_kisiler_1", kisi1),
        Bolum::new("Kişiler (diğer)", "zihin_kisiler_2", kisi2),
        Bolum::new("Konular", "zihin_konular", konular),
        Bolum::new("Olaylar + Gündem", "zihin_olaylar", olaylar),
    ]
}

pub fn zihin_bolumleri(d: &Durum) -> Vec<Bolum> {
    let ozet = durum_metni(d)
        + &match d.kendim.trim() {
            "" => String::new(),
            k => format!(
                "\n\nkendim:\n{}",
                k.lines()
                    .rev()
                    .take(4)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
        };
    let kisiler: Vec<String> = hafiza::kisi_dokumleri().iter().map(kisi_gosterim).collect();
    let mut konular = String::new();
    for (ad, son) in hafiza::konu_dokumleri() {
        konular += &format!("- {ad} · son: {son}\n");
    }
    let olay_dokumu = hafiza::olay_dokumu();
    let olay_satirlari: Vec<&str> = olay_dokumu
        .lines()
        .filter(|l| l.starts_with("- "))
        .collect();
    let atla = olay_satirlari.len().saturating_sub(15);
    let mut olaylar = format!(
        "Olaylar ({}):\n{}",
        hafiza::ay(),
        olay_satirlari[atla..].join("\n")
    );
    if !d.gundem.trim().is_empty() {
        olaylar += &format!("\n\nGündem:\n{}", hafiza::kirp(&d.gundem, 1500));
    }
    bolumler(ozet, kisiler, konular, olaylar)
}

// ---------- modal kurulumu ----------

fn modal_olustur(baslik: &str, custom_id: &str, bolumler: Vec<Bolum>) -> CreateModal {
    let satirlar = bolumler
        .into_iter()
        .map(|b| {
            let deger = if b.icerik.trim().is_empty() {
                "(henüz boş)".to_string()
            } else {
                b.icerik
            };
            CreateActionRow::InputText(
                CreateInputText::new(InputTextStyle::Paragraph, b.etiket, b.custom_id)
                    .value(deger)
                    .required(false),
            )
        })
        .collect();
    CreateModal::new(custom_id, baslik.chars().take(45).collect::<String>()).components(satirlar)
}

pub fn modal_zihin(d: &Durum) -> CreateModal {
    modal_olustur(
        &format!("{} · zihin", d.bot_adi),
        "modal_zihin",
        zihin_bolumleri(d),
    )
}

pub fn modal_durum(d: &Durum) -> CreateModal {
    modal_olustur(
        "durum",
        "modal_durum",
        vec![Bolum::new(
            "Botun şu anki hali",
            "durum_metni",
            durum_metni(d),
        )],
    )
}

pub fn modal_yardim() -> CreateModal {
    modal_olustur(
        "yardım",
        "modal_yardim",
        vec![Bolum::new(
            "Komutlar",
            "yardim_metni",
            format!(
                "{}\n\nslash komutları (/durum, /yardim, /zihin) bu modal'ları açar",
                komut::YARDIM
            ),
        )],
    )
}

// ---------- slash kaydı ----------

// sunucu komutları: her ready'de çağrılır, discord üstüne yazar (idempotent);
// sunucu komutu anında görünür olur, global komut gecikmeli
pub async fn komutlari_kayit(http: &Http, guild: GuildId) -> Result<(), Hata> {
    guild
        .set_commands(
            http,
            vec![
                CreateCommand::new("durum").description("Botun şu anki halini modal'da gösterir"),
                CreateCommand::new("yardim").description("Komut listesini modal'da gösterir"),
                CreateCommand::new("zihin")
                    .description("Botun bildiklerini 5 bölmeli modal'da gösterir"),
            ],
        )
        .await?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn slot_sinirlari_kirpar() {
        let uzun = "a".repeat(10_000);
        let kisiler = vec![uzun.clone(), uzun.clone(), uzun.clone()];
        let b = bolumler(uzun.clone(), kisiler, uzun.clone(), uzun);
        assert_eq!(b.len(), 5);
        for slot in &b {
            assert!(slot.icerik.chars().count() <= MODAL_SINIR);
            assert!(slot.icerik.contains("kırpıldı"));
        }
    }

    #[test]
    fn kisa_metin_kirpilmaz() {
        let b = bolumler(
            "özet".into(),
            vec!["ali (+1)".into()],
            "konu".into(),
            "olay".into(),
        );
        assert_eq!(b.len(), 5);
        assert_eq!(b[0].icerik, "özet");
        assert_eq!(b[1].icerik, "ali (+1)");
        assert!(b[2].icerik.is_empty()); // tek kişi: ikinci slot boş
    }

    #[test]
    fn kisiler_ikiye_bolunur() {
        let kisiler: Vec<String> = (0..5).map(|i| format!("kişi{i}")).collect();
        let b = bolumler(String::new(), kisiler, String::new(), String::new());
        assert_eq!(b[1].icerik, "kişi0\n\nkişi1\n\nkişi2");
        assert_eq!(b[2].icerik, "kişi3\n\nkişi4");
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
}
