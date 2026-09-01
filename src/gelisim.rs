// Gelişim evreleri: bot sunucuda geçirdiği güne ve bitirdiği sohbete göre evre atlar.
// Evre, sistem mesajına "bu evrede nasıl birisin" satırı ekler ve araya girme / laf atma
// şanslarını ölçekler. Yerleşik evresine girerken kendine isim seçer, takma adını değiştirir.
// Sayaçlar durum/gelisim.md'de durur; yeniden başlatınca kaldığı yerden devam eder.

use super::*;

pub struct Evre {
    pub ad: &'static str,
    pub min_gun: i64,    // doğumdan bu yana en az kaç gün
    pub min_sohbet: u32, // en az kaç biten sohbet
    pub sans: f64,       // SANS çarpanı (araya girme)
    pub durtme: f64,     // DURTME_SANSI çarpanı
    pub aciklama: &'static str,
}

pub const ISIM_EVRESI: usize = 2; // bu evreye (yerlesik) girince isim seçer

pub const EVRELER: &[Evre] = &[
    Evre {
        ad: "yeni",
        min_gun: 0,
        min_sohbet: 0,
        sans: 0.7,
        durtme: 0.4,
        aciklama: "Sunucuya yeni geldin. Herkesi tanımıyorsun; az konuş, çok dinle, soru sor, iç şakalara \
                   henüz girme, yanlış anlamaktan çekin. Kendine bir yer arıyorsun, biraz temkinlisin.",
    },
    Evre {
        ad: "isinma",
        min_gun: 3,
        min_sohbet: 8,
        sans: 0.8,
        durtme: 0.7,
        aciklama: "Isınıyorsun: birkaç kişiyi tanıdın, arada laf sokmaya başladın ama sınırları hâlâ \
                   yokluyorsun. İlk iç şakaları kapıyorsun, bazen yanlış yere gülüyorsun.",
    },
    Evre {
        ad: "yerlesik",
        min_gun: 10,
        min_sohbet: 25,
        sans: 1.0,
        durtme: 1.0,
        aciklama: "Artık buranın parçasısın: kendi kalıpların, sevdiklerin ve sevmediklerin belli. \
                   Bu evreye girerken kendine bir isim seçtin; o isimle anılıyorsun.",
    },
    Evre {
        ad: "eski-toprak",
        min_gun: 30,
        min_sohbet: 80,
        sans: 1.0,
        durtme: 1.2,
        aciklama: "Eski toprak: geçmişe gönderme yapan, yeni gelenlere burayı anlatan, gerektiğinde \
                   susmayı bilen biri. Hikâyen var, herkes seni tanıyor, sen de onları.",
    },
];

#[derive(Default, Clone)]
pub struct Gelisim {
    pub dogum: i64, // ilk çalıştığı an (unix)
    pub sohbet: u32,
    pub mesaj: u32,
    pub evre: usize, // EVRELER indeksi, yalnız ileri gider
    pub isim: Option<String>,
}

pub fn yukle() -> Gelisim {
    let mut g = Gelisim {
        dogum: simdi_unix(),
        ..Default::default()
    };
    for satir in hafiza::oku("gelisim.md").lines() {
        let Some((k, v)) = satir.split_once(':') else {
            continue;
        };
        let v = v.trim();
        match k.trim() {
            "dogum" => g.dogum = v.parse().unwrap_or(g.dogum),
            "sohbet" => g.sohbet = v.parse().unwrap_or(0),
            "mesaj" => g.mesaj = v.parse().unwrap_or(0),
            "evre" => g.evre = v.parse::<usize>().unwrap_or(0).min(EVRELER.len() - 1),
            "isim" if !v.is_empty() => g.isim = Some(v.to_string()),
            _ => {}
        }
    }
    g
}

pub fn kaydet(g: &Gelisim) {
    hafiza::yaz(
        "gelisim.md",
        &format!(
            "dogum: {}\nsohbet: {}\nmesaj: {}\nevre: {}\nisim: {}\n",
            g.dogum,
            g.sohbet,
            g.mesaj,
            g.evre,
            g.isim.as_deref().unwrap_or("")
        ),
    );
}

pub fn gun(g: &Gelisim) -> i64 {
    (simdi_unix() - g.dogum) / 86400
}

// hak edilen evre: hem gün hem sohbet eşiğini geçen en yüksek evre
pub fn hak_edilen(g: &Gelisim) -> usize {
    let gun = gun(g);
    EVRELER
        .iter()
        .enumerate()
        .filter(|(_, e)| gun >= e.min_gun && g.sohbet >= e.min_sohbet)
        .map(|(i, _)| i)
        .max()
        .unwrap_or(0)
}

pub fn evre(g: &Gelisim) -> &'static Evre {
    &EVRELER[g.evre.min(EVRELER.len() - 1)]
}

// sistem mesajına giden bölüm
pub fn evre_metni(g: &Gelisim) -> String {
    let e = evre(g);
    format!(
        "{} evresi ({}. gün, {} sohbet). {}",
        e.ad,
        gun(g) + 1,
        g.sohbet,
        e.aciklama
    )
}

// modelin verdiği isim önerisini tek kelimeye indirir; olmadıysa None
pub fn isim_temizle(metin: &str) -> Option<String> {
    let aday: String = metin
        .split_whitespace()
        .next()?
        .chars()
        .filter(|c| c.is_alphanumeric())
        .take(20)
        .collect();
    (aday.chars().count() >= 2).then_some(aday)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn evre_esikleri() {
        let mut g = Gelisim {
            dogum: simdi_unix() - 11 * 86400,
            sohbet: 30,
            ..Default::default()
        };
        assert_eq!(EVRELER[hak_edilen(&g)].ad, "yerlesik");
        g.sohbet = 5;
        assert_eq!(EVRELER[hak_edilen(&g)].ad, "yeni");
        g.sohbet = 100;
        assert_eq!(EVRELER[hak_edilen(&g)].ad, "yerlesik"); // gün yetmiyor
    }

    #[test]
    fn isim_temizlenir() {
        assert_eq!(isim_temizle("\"Kaju\"").as_deref(), Some("Kaju"));
        assert_eq!(
            isim_temizle("Bundan sonra Zeytin de").as_deref(),
            Some("Bundan")
        );
        assert_eq!(isim_temizle("!").is_none(), true);
    }
}
