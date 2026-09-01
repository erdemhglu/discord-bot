// Yıl içindeki etkinliklere göre seyahat taklidi: bayramlar, uzun hafta sonları, yaz tatili,
// festival. Seyahatteyken telefondan yazıyormuş gibi seyrek konuşur, araya girme şansı düşer,
// haber ve şaka atmaz, günde bir "yoldan" mesaj atar, gitmeden bir gün önce haber verir.
// Durum tutmaz: hangi gün neredeyse takvimden hesaplanır; yer seçimi yıla göre sabittir.

use super::*;

pub const YOLDA_SANS_CARPANI: f64 = 0.3; // seyahatte araya girme şansı bu kadar düşer

pub struct Seyahat {
    pub yer: &'static str,
    pub sebep: &'static str,
    pub bas: i64, // yerel gün numarası
    pub bit: i64, // son gün (dahil değil)
}

struct Etkinlik {
    yil: Option<i64>, // None: her yıl
    ay: i64,
    gun: i64,
    sure: i64,
    sebep: &'static str,
    yerler: &'static [&'static str],
}

// bayramlar yıla göre kayar, onlar yıl yıl yazılı; gerisi her yıl aynı
const ETKINLIKLER: &[Etkinlik] = &[
    Etkinlik {
        yil: None,
        ay: 12,
        gun: 30,
        sure: 4,
        sebep: "yılbaşı",
        yerler: &["Kartepe'de dağ evi", "Bursa'da arkadaşının evi", "memleket"],
    },
    Etkinlik {
        yil: None,
        ay: 1,
        gun: 24,
        sure: 7,
        sebep: "sömestr",
        yerler: &["memleket", "İzmir'de kuzeninin yanı"],
    },
    Etkinlik {
        yil: Some(2026),
        ay: 3,
        gun: 19,
        sure: 4,
        sebep: "ramazan bayramı",
        yerler: &["memleket, akraba turu"],
    },
    Etkinlik {
        yil: Some(2027),
        ay: 3,
        gun: 8,
        sure: 4,
        sebep: "ramazan bayramı",
        yerler: &["memleket, akraba turu"],
    },
    Etkinlik {
        yil: None,
        ay: 4,
        gun: 23,
        sure: 3,
        sebep: "23 nisan uzatması",
        yerler: &["Bozcaada", "Ayvalık"],
    },
    Etkinlik {
        yil: None,
        ay: 5,
        gun: 19,
        sure: 3,
        sebep: "19 mayıs kaçamağı",
        yerler: &["Datça", "Kaz Dağları'nda kamp"],
    },
    Etkinlik {
        yil: Some(2026),
        ay: 5,
        gun: 26,
        sure: 5,
        sebep: "kurban bayramı",
        yerler: &["memleket"],
    },
    Etkinlik {
        yil: Some(2027),
        ay: 5,
        gun: 15,
        sure: 5,
        sebep: "kurban bayramı",
        yerler: &["memleket"],
    },
    Etkinlik {
        yil: None,
        ay: 7,
        gun: 14,
        sure: 6,
        sebep: "yaz tatili",
        yerler: &["Kaş", "Fethiye", "Marmaris'te arkadaşlarla"],
    },
    Etkinlik {
        yil: None,
        ay: 8,
        gun: 21,
        sure: 4,
        sebep: "zeytinli rock festivali",
        yerler: &["Burhaniye, festival alanında çadır"],
    },
    Etkinlik {
        yil: None,
        ay: 8,
        gun: 30,
        sure: 3,
        sebep: "30 ağustos uzatması",
        yerler: &["Kapadokya", "Eskişehir"],
    },
    Etkinlik {
        yil: None,
        ay: 10,
        gun: 29,
        sure: 3,
        sebep: "29 ekim uzatması",
        yerler: &["Ankara, Anıtkabir sonra arkadaşlar"],
    },
];

// takvim gününden gün numarası (1970-01-01 = 0), dış kütüphanesiz
pub fn gun_no(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn yil(gun: i64) -> i64 {
    hafiza::tarih_unix(gun * 86400)[..4].parse().unwrap_or(1970)
}

// verilen yerel gün için seyahat var mı
pub fn gunde(gun: i64) -> Option<Seyahat> {
    let bu_yil = yil(gun);
    for e in ETKINLIKLER {
        for y in [bu_yil - 1, bu_yil] {
            if e.yil.is_some_and(|ey| ey != y) {
                continue;
            }
            let bas = gun_no(y, e.ay, e.gun);
            if bas <= gun && gun < bas + e.sure {
                let yer = e.yerler[((y + e.ay * 31 + e.gun) as usize) % e.yerler.len()];
                return Some(Seyahat {
                    yer,
                    sebep: e.sebep,
                    bas,
                    bit: bas + e.sure,
                });
            }
        }
    }
    None
}

pub fn bugun() -> i64 {
    uyku::yerel(simdi_unix()).0
}

pub fn simdi() -> Option<Seyahat> {
    gunde(bugun())
}

// yarın başlayan ama bugün olmayan seyahat
pub fn yarin() -> Option<Seyahat> {
    let g = bugun();
    match (gunde(g), gunde(g + 1)) {
        (None, Some(s)) => Some(s),
        _ => None,
    }
}

// sistem mesajına giden satır
pub fn durum_metni() -> String {
    match simdi() {
        Some(s) => format!(
            "Şu an {}'desin ({}); {} gündür oradasın, {} gün sonra dönüyorsun. Telefondan arada bir bakıyorsun; bunu bilerek konuş, ara ara oradan bahset.",
            s.yer,
            s.sebep,
            bugun() - s.bas + 1,
            s.bit - bugun()
        ),
        None => match yarin() {
            Some(s) => format!("Yarın {}'ye gidiyorsun ({}), hazırlık var.", s.yer, s.sebep),
            None => String::new(),
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn gun_no_dogru() {
        assert_eq!(gun_no(1970, 1, 1), 0);
        assert_eq!(gun_no(2026, 9, 1), 1788220800 / 86400);
        assert_eq!(
            hafiza::tarih_unix(gun_no(2026, 12, 31) * 86400),
            "2026-12-31"
        );
    }

    #[test]
    fn yilbasi_yila_sarkar() {
        let s = gunde(gun_no(2027, 1, 2)).expect("2 ocakta yılbaşı seyahati olmalı");
        assert_eq!(s.sebep, "yılbaşı");
        assert!(gunde(gun_no(2027, 1, 3)).is_none());
    }

    #[test]
    fn bayram_yila_gore() {
        assert_eq!(
            gunde(gun_no(2026, 3, 20)).map(|s| s.sebep),
            Some("ramazan bayramı")
        );
        assert!(gunde(gun_no(2025, 3, 20)).is_none());
        assert_eq!(
            gunde(gun_no(2026, 9, 1)).map(|s| s.sebep),
            Some("30 ağustos uzatması")
        );
        assert!(gunde(gun_no(2026, 9, 5)).is_none());
    }

    #[test]
    fn yer_sabit() {
        let a = gunde(gun_no(2026, 7, 15)).unwrap().yer;
        let b = gunde(gun_no(2026, 7, 18)).unwrap().yer;
        assert_eq!(a, b);
    }
}
