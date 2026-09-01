// Uyku düzeni: bizim gibi. Normalde gece 01'de uyur 09'da kalkar (±45 dk oynar).
// Nadiren uykusuz gece: o gece 01-06 arası ayakta, sonra 06-13 uyur. Uykusuzluk
// şansı botun haline göre: kırgınsa, sinirliyse, takıntılıysa daha sık.
// Uyurken yazmaz, araya girmez, haber atmaz; etiketlenirse uyanınca döner.

use super::*;

pub const SAAT_FARKI: i64 = 3 * 3600; // türkiye, utc+3
pub const UYKUSUZLUK_SANSI: f64 = 0.07; // sıradan bir gece
pub const UYKUSUZLUK_GERGIN: f64 = 0.20; // hali bozuksa

#[derive(Clone, Copy)]
pub struct Plan {
    pub gun: i64,                 // hangi günün gecesi (yerel gün numarası)
    pub uykusuz_bas: Option<i64>, // uykusuz geceyse ayakta kaldığı saat (unix)
    pub bas: i64,                 // uyku başı (unix)
    pub bit: i64,                 // uyku sonu (unix)
}

pub fn yerel(unix: i64) -> (i64, i64) {
    let y = unix + SAAT_FARKI;
    (y.div_euclid(86400), y.rem_euclid(86400))
}

pub fn saat() -> String {
    let (_, sn) = yerel(simdi_unix());
    format!("{:02}:{:02}", sn / 3600, (sn % 3600) / 60)
}

pub fn saat_metni() -> String {
    let (gun, _) = yerel(simdi_unix());
    let adlar = [
        "perşembe",
        "cuma",
        "cumartesi",
        "pazar",
        "pazartesi",
        "salı",
        "çarşamba",
    ];
    format!(
        "{} {} {}",
        hafiza::tarih_unix(simdi_unix() + SAAT_FARKI),
        adlar[gun.rem_euclid(7) as usize],
        saat()
    )
}

fn oynama() -> i64 {
    (rand::random::<u32>() % 5400) as i64 - 2700 // ±45 dk
}

fn gergin_mi(d: &Durum) -> bool {
    let m = format!("{} {}", d.kendim, d.huy).to_lowercase();
    [
        "kırgın",
        "sinir",
        "gergin",
        "takıntı",
        "uyku",
        "kafayı",
        "bunalt",
    ]
    .iter()
    .any(|k| m.contains(k))
}

fn plan_kur(gun: i64, gergin: bool) -> Plan {
    let gece = (gun + 1) * 86400 - SAAT_FARKI; // ertesi gün 00:00, unix
    let sans = if gergin {
        UYKUSUZLUK_GERGIN
    } else {
        UYKUSUZLUK_SANSI
    };
    if rand::random::<f64>() < sans {
        Plan {
            gun,
            uykusuz_bas: Some(gece + 3600),
            bas: gece + 6 * 3600 + oynama(),
            bit: gece + 13 * 3600 + oynama(),
        }
    } else {
        Plan {
            gun,
            uykusuz_bas: None,
            bas: gece + 3600 + oynama(),
            bit: gece + 9 * 3600 + oynama(),
        }
    }
}

// bugün ve dünün gecesi için plan yoksa kurar, biteni atar
pub fn guncelle(d: &mut Durum) {
    let simdi = simdi_unix();
    let (gun, _) = yerel(simdi);
    for g in [gun - 1, gun] {
        if !d.planlar.iter().any(|p| p.gun == g) {
            let p = plan_kur(g, gergin_mi(d));
            if p.uykusuz_bas.is_some() {
                println!(
                    "uyku: {} gecesi uykusuz geçecek",
                    hafiza::tarih_unix(g * 86400)
                );
            }
            d.planlar.push(p);
        }
    }
    d.planlar.retain(|p| p.bit > simdi);
}

pub fn uyanik_mi(d: &Durum) -> bool {
    let s = simdi_unix();
    if s < d.uyanik_zorla {
        return true; // !uyan denmiş
    }
    !d.planlar.iter().any(|p| p.bas <= s && s < p.bit)
}

pub fn uykusuz_mu(d: &Durum) -> bool {
    let s = simdi_unix();
    d.planlar
        .iter()
        .any(|p| p.uykusuz_bas.is_some_and(|u| u <= s && s < p.bas))
}

// sistem mesajına giden "şu an" satırı
pub fn durum_metni(d: &Durum) -> String {
    let mut s = saat_metni();
    if uykusuz_mu(d) {
        s += ". Uyuyamadın, gecenin bu saatinde ayaktasın; o modasın, yarın işin var ama kafan durmuyor.";
    } else if !uyanik_mi(d) {
        s += ". Uyuyorsun.";
    }
    s
}
