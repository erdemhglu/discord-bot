// Dosya tabanlı hafıza, ikinci beyin mantığı:
//   - INDEX.md      ne bildiğinin listesi; her cevapta gider (işaretçi, veri değil)
//   - kisiler/      kişi başına bir dosya; yalnız o sohbette konuşan kişilerinki getirilir
//   - konular/      konu başına bir dosya; sohbetteki anahtar kelimelere göre getirilir
//   - olaylar/      biten her sohbetten tek satır, aylık dosya
//   - arsiv/        sınırı aşan dosyalardan özetlenip çıkarılan ham parçalar (silme yok)
// Sınır dolunca özetleyici ajan dosyayı küçültür (ajanlar.rs). Bağlam penceresi
// büyümez: her cevaba dizin + o sohbet için getirilenler gider, bütçesi sabittir.

use super::*;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

// yazımlar tek sıradan ve atomik geçer: iki ajan aynı dosyaya yürüse ya da süreç
// ortasında düşse dosya yarım/bozuk kalmaz (geçici + rename, aynı dosya sistemi atomik)
static YAZMA_KILIDI: Mutex<()> = Mutex::new(());

pub const KISI_SINIRI: usize = 1800; // kişi dosyası bunu aşınca özetlenir
pub const KISI_HEDEF: usize = 1000; // özetlenince hedef boy
pub const KONU_SINIRI: usize = 1500;
pub const KONU_HEDEF: usize = 800;
pub const OLAY_SINIRI: usize = 6000; // aylık olay dosyası bunu aşınca eski yarısı özetlenir
pub const BAGLAM_BUTCESI: usize = 6000; // bir cevaba getirilen hafızanın toplam karakteri
pub const DIZIN_KISI: usize = 40; // dizinde en fazla kaç kişi görünür
pub const FAVORI_NOTU: &str = "canın ciğerin, ne yaparsa yapsın arkasındasın";

// ---------- dosya işleri ----------

pub fn yol(parca: &str) -> PathBuf {
    Path::new(DURUM_KLASORU).join(parca)
}

pub fn oku(parca: &str) -> String {
    fs::read_to_string(yol(parca)).unwrap_or_default()
}

pub fn yaz(parca: &str, icerik: &str) {
    let _kilit = YAZMA_KILIDI.lock().unwrap_or_else(|e| e.into_inner());
    let p = yol(parca);
    if let Some(ust) = p.parent() {
        let _ = fs::create_dir_all(ust);
    }
    // geçici dosyaya yaz, sonra rename: yarım dosya hiç görünmez
    let gecici = p.with_extension("tmp");
    let sonuc = fs::write(&gecici, icerik).and_then(|_| fs::rename(&gecici, &p));
    if let Err(e) = sonuc {
        let _ = fs::remove_file(&gecici);
        log::error!("{} yazılamadı: {e}", p.display());
    }
}

// oku+yaz ile bütün dosyayı yeniden yazmak yerine gerçek append
fn ekle(parca: &str, satir: &str) {
    let _kilit = YAZMA_KILIDI.lock().unwrap_or_else(|e| e.into_inner());
    let p = yol(parca);
    if let Some(ust) = p.parent() {
        let _ = fs::create_dir_all(ust);
    }
    let sonuc = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&p)
        .and_then(|mut f| f.write_all(format!("{satir}\n").as_bytes()));
    if let Err(e) = sonuc {
        log::error!("{} eklenemedi: {e}", p.display());
    }
}

// özetlenip atılan ham parça arşive gider, hiçbir şey silinmez
pub fn arsivle(parca: &str, icerik: &str) {
    ekle(
        &format!("arsiv/{parca}"),
        &format!("\n## {} öncesi\n{}", tarih_saat(), icerik.trim_end()),
    );
}

pub fn slug(isim: &str) -> String {
    let mut s = String::new();
    for c in isim.chars().flat_map(|c| c.to_lowercase()) {
        let c = match c {
            'ç' => 'c',
            'ğ' => 'g',
            'ı' => 'i',
            'ö' => 'o',
            'ş' => 's',
            'ü' => 'u',
            'â' => 'a',
            'î' => 'i',
            'û' => 'u',
            c => c,
        };
        if c.is_ascii_alphanumeric() {
            s.push(c);
        } else if !s.is_empty() && !s.ends_with('-') {
            s.push('-');
        }
    }
    let s = s.trim_end_matches('-').to_string();
    if s.is_empty() {
        "bilinmeyen".to_string()
    } else {
        s
    }
}

// YYYY-AA-GG, dış kütüphanesiz
pub fn tarih() -> String {
    tarih_unix(simdi_unix())
}

// tüm kayıtlar saniyeli zaman damgasıyla düşer: YYYY-AA-GG SS:DD:SS
pub fn tarih_saat() -> String {
    format!("{} {}", tarih(), crate::uyku::saat_saniye())
}

pub fn tarih_unix(unix: i64) -> String {
    let z = unix.div_euclid(86400) + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let g = doy - (153 * mp + 2) / 5 + 1;
    let a = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if a <= 2 { 1 } else { 0 };
    format!("{y:04}-{a:02}-{g:02}")
}

pub fn ay() -> String {
    tarih()[..7].to_string()
}

// ---------- kişi dosyası ----------

#[derive(Default, Clone)]
pub struct Kisi {
    pub id: u64,                 // dosya anahtarı; discord kullanıcı id'si
    pub isim: String,            // görünen ad (global_name ya da kullanıcı adı)
    pub kullanici_adi: String,   // discord kullanıcı adı
    pub eski_adlar: Vec<String>, // önceki görünen adlar
    pub puan: i32,
    pub etiket: Vec<String>,
    pub not: String,
    pub bilgiler: Vec<String>,
    pub olaylar: Vec<String>,
}

impl Kisi {
    pub fn coz(id: u64, metin: &str) -> Kisi {
        let mut k = Kisi {
            id,
            ..Default::default()
        };
        let mut bolum = "";
        for satir in metin.lines() {
            let s = satir.trim();
            if let Some(b) = s.strip_prefix("# ") {
                k.isim = b.trim().to_string();
            } else if let Some(b) = s.strip_prefix("## ") {
                bolum = if b.starts_with("Bildik") {
                    "bilgi"
                } else if b.starts_with("Son") {
                    "olay"
                } else {
                    ""
                };
            } else if let Some(v) = s.strip_prefix("id:") {
                k.id = v.trim().parse().unwrap_or(k.id);
            } else if let Some(v) = s.strip_prefix("kullanici_adi:") {
                k.kullanici_adi = v.trim().to_string();
            } else if let Some(v) = s.strip_prefix("eski_adlar:") {
                k.eski_adlar = v
                    .split(',')
                    .map(|e| e.trim().to_string())
                    .filter(|e| !e.is_empty())
                    .collect();
            } else if let Some(v) = s.strip_prefix("puan:") {
                k.puan = v.trim().parse().unwrap_or(0);
            } else if let Some(v) = s.strip_prefix("etiket:") {
                k.etiket = v
                    .split(',')
                    .map(|e| e.trim().to_string())
                    .filter(|e| !e.is_empty())
                    .collect();
            } else if let Some(v) = s.strip_prefix("not:") {
                k.not = v.trim().to_string();
            } else if let Some(v) = s.strip_prefix("- ") {
                match bolum {
                    "bilgi" => k.bilgiler.push(v.to_string()),
                    "olay" => k.olaylar.push(v.to_string()),
                    _ => {}
                }
            }
        }
        k
    }

    pub fn metin(&self) -> String {
        let mut s = format!(
            "# {}\nid: {}\nkullanici_adi: {}\neski_adlar: {}\npuan: {:+}\netiket: {}\nnot: {}\n\n## Bildiklerin\n",
            self.isim,
            self.id,
            self.kullanici_adi,
            self.eski_adlar.join(", "),
            self.puan,
            self.etiket.join(", "),
            self.not
        );
        for b in &self.bilgiler {
            s += &format!("- {b}\n");
        }
        s += "\n## Son olaylar\n";
        for o in &self.olaylar {
            s += &format!("- {o}\n");
        }
        s
    }
}

pub fn kisi_oku(id: u64) -> Kisi {
    let m = oku(&format!("kisiler/{id}.md"));
    if m.is_empty() {
        Kisi {
            id,
            ..Default::default()
        }
    } else {
        Kisi::coz(id, &m)
    }
}

pub fn kisi_yaz(k: &Kisi) {
    yaz(&format!("kisiler/{}.md", k.id), &k.metin());
}

// ---------- konu ve olay ----------

pub fn konu_ekle(ad: &str, not: &str) {
    let parca = format!("konular/{}.md", slug(ad));
    if oku(&parca).is_empty() {
        yaz(&parca, &format!("# {ad}\netiket: \n\n"));
    }
    ekle(&parca, &format!("- {}: {}", tarih_saat(), not.trim()));
}

pub fn olay_ekle(kanal: &str, olay: &str) {
    ekle(
        &format!("olaylar/{}.md", ay()),
        &format!("- {} #{}: {}", tarih_saat(), kanal, olay.trim()),
    );
}

// modal gösterimi için dökümler (mtime sırası, en son değişen önce)

pub fn kisi_dokumleri() -> Vec<Kisi> {
    let mut v = Vec::new();
    for p in dosyalar("kisiler") {
        // id bazlı dosya adı; eski slug dosyaları (id çözülemez) atlanır
        let Some(id) = p
            .file_stem()
            .and_then(|f| f.to_str())
            .and_then(|f| f.parse::<u64>().ok())
        else {
            continue;
        };
        let k = Kisi::coz(id, &fs::read_to_string(&p).unwrap_or_default());
        if k.isim.is_empty() {
            continue;
        }
        v.push(k);
    }
    v
}

// (konu adı, son not)
pub fn konu_dokumleri() -> Vec<(String, String)> {
    dosyalar("konular")
        .into_iter()
        .take(30)
        .map(|p| {
            let icerik = fs::read_to_string(&p).unwrap_or_default();
            let son = icerik
                .lines()
                .rev()
                .find(|l| l.starts_with("- "))
                .map(|l| l.trim_start_matches("- ").to_string())
                .unwrap_or_default();
            (ilk_satir(&p), son)
        })
        .collect()
}

pub fn olay_dokumu() -> String {
    oku(&format!("olaylar/{}.md", ay()))
}

// ---------- kanal geçmişi ----------

// durum/kanallar/<id>.md dosyalarını okur: (kanal id, son satırlar)
pub fn kanal_gecmisi_yukle() -> Vec<(u64, VecDeque<String>)> {
    let mut v = Vec::new();
    for p in dosyalar("kanallar") {
        let Some(id) = p
            .file_stem()
            .and_then(|f| f.to_str())
            .and_then(|f| f.parse::<u64>().ok())
        else {
            continue;
        };
        let satirlar: VecDeque<String> = fs::read_to_string(&p)
            .unwrap_or_default()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.to_string())
            .collect();
        v.push((id, satirlar));
    }
    v
}

// ---------- dizin ----------

fn dosyalar(klasor: &str) -> Vec<PathBuf> {
    let mut v: Vec<(std::time::SystemTime, PathBuf)> = fs::read_dir(yol(klasor))
        .map(|d| {
            d.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|u| u == "md"))
                .filter_map(|p| Some((fs::metadata(&p).ok()?.modified().ok()?, p)))
                .collect()
        })
        .unwrap_or_default();
    v.sort_by_key(|e| std::cmp::Reverse(e.0)); // en son değişen önce
    v.into_iter().map(|(_, p)| p).collect()
}

fn ilk_satir(p: &Path) -> String {
    fs::read_to_string(p)
        .unwrap_or_default()
        .lines()
        .next()
        .unwrap_or("")
        .trim_start_matches("# ")
        .to_string()
}

// INDEX.md'yi yeniden üretir ve döndürür: her cevaba giden "ne biliyorum" listesi
pub fn dizin_yenile() -> String {
    let mut s = String::from("## Kişiler\n");
    for p in dosyalar("kisiler").into_iter().take(DIZIN_KISI) {
        // id bazlı dosya adı; eski slug dosyaları (id çözülemez) atlanır
        let Some(id) = p
            .file_stem()
            .and_then(|f| f.to_str())
            .and_then(|f| f.parse::<u64>().ok())
        else {
            continue;
        };
        let k = Kisi::coz(id, &fs::read_to_string(&p).unwrap_or_default());
        if k.isim.is_empty() {
            continue;
        }
        s += &format!("- {} ({:+})", k.isim, k.puan);
        if !k.etiket.is_empty() {
            s += &format!(" · {}", k.etiket.join(", "));
        }
        if !k.not.is_empty() {
            s += &format!(" · {}", k.not);
        }
        s += "\n";
    }
    s += "\n## Konular\n";
    for p in dosyalar("konular").into_iter().take(30) {
        let icerik = fs::read_to_string(&p).unwrap_or_default();
        let son = icerik
            .lines()
            .rev()
            .find(|l| l.starts_with("- "))
            .and_then(|l| l.get(2..12))
            .unwrap_or("");
        s += &format!("- {} · son: {}\n", ilk_satir(&p), son);
    }
    s += "\n## Olaylar\n";
    for p in dosyalar("olaylar").into_iter().take(3) {
        let n = fs::read_to_string(&p)
            .unwrap_or_default()
            .lines()
            .filter(|l| l.starts_with("- "))
            .count();
        s += &format!(
            "- {} · {n} kayıt\n",
            p.file_stem().and_then(|f| f.to_str()).unwrap_or("")
        );
    }
    yaz("INDEX.md", &s);
    s
}

// ---------- getirme ----------

const DURAK: &[&str] = &[
    "için", "gibi", "değil", "bence", "yani", "falan", "filan", "diye", "olan", "daha", "böyle",
    "şöyle", "nasıl", "neden", "niye", "sonra", "önce", "şimdi", "bugün", "yarın", "zaten", "hala",
    "hâlâ", "bile", "kadar", "biraz", "bayağı", "aynen", "tamam", "evet", "hayır", "olur", "oldu",
    "olsun", "yapsın", "yaptı", "bunu", "şunu", "onun", "bunun", "bana", "sana", "beni", "seni",
    "bizi", "sizi", "kendi", "hangi", "nerede", "burada", "orada", "http", "https", "that", "this",
    "with", "have", "what", "just", "like", "abi", "lan", "amk", "aga", "reis",
];

// sohbet metninden arama anahtarları: 4+ harfli, sık kelimeler elenmiş
pub fn anahtarlar(metinler: &[String]) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for m in metinler {
        for k in m.split(|c: char| !c.is_alphanumeric()) {
            let k = k.to_lowercase();
            if k.chars().count() >= 4 && !DURAK.contains(&k.as_str()) && !v.contains(&k) {
                v.push(k);
            }
        }
    }
    v.truncate(40);
    v
}

fn puanla(metin: &str, anahtar: &[String]) -> usize {
    let m = metin.to_lowercase();
    anahtar.iter().filter(|a| m.contains(a.as_str())).count()
}

pub fn kirp(metin: &str, sinir: usize) -> String {
    if metin.chars().count() <= sinir {
        metin.trim().to_string()
    } else {
        format!("{}…", metin.chars().take(sinir).collect::<String>().trim())
    }
}

// bu sohbet için ne getirilecek: konuşanların dosyaları, konuya değen konu dosyaları,
// son olaylar ve ham hafızadan anahtar kelimeye uyan eski satırlar. bütçe sabit.
pub fn getir(
    katilimcilar: &[String],
    ad_id: &std::collections::HashMap<String, u64>,
    anahtar: &[String],
    hafiza: &VecDeque<String>,
    atla_son: usize,
) -> String {
    let mut bolumler: Vec<String> = Vec::new();

    for isim in katilimcilar.iter().take(4) {
        let Some(id) = ad_id.get(&isim.to_lowercase()) else {
            continue;
        };
        let m = oku(&format!("kisiler/{id}.md"));
        if !m.is_empty() {
            bolumler.push(kirp(&m, 1200));
        }
    }

    let mut konular: Vec<(usize, PathBuf)> = dosyalar("konular")
        .into_iter()
        .map(|p| {
            (
                puanla(&fs::read_to_string(&p).unwrap_or_default(), anahtar),
                p,
            )
        })
        .filter(|(puan, _)| *puan >= 1)
        .collect();
    konular.sort_by_key(|k| std::cmp::Reverse(k.0));
    for (_, p) in konular.into_iter().take(2) {
        bolumler.push(kirp(&fs::read_to_string(&p).unwrap_or_default(), 800));
    }

    let olaylar = oku(&format!("olaylar/{}.md", ay()));
    let son_olaylar: Vec<&str> = olaylar.lines().filter(|l| l.starts_with("- ")).collect();
    if !son_olaylar.is_empty() {
        let atla = son_olaylar.len().saturating_sub(8);
        bolumler.push(format!("Son olaylar:\n{}", son_olaylar[atla..].join("\n")));
    }

    // ham bağlam penceresi: son sohbette olmayan ama konuya değen eski satırlar
    if !anahtar.is_empty() {
        let n = hafiza.len().saturating_sub(atla_son);
        let mut eski: Vec<(usize, usize, &String)> = hafiza
            .iter()
            .take(n)
            .enumerate()
            .map(|(i, s)| (puanla(s, anahtar), i, s))
            .filter(|(p, _, _)| *p >= 2)
            .collect();
        eski.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
        eski.truncate(12);
        eski.sort_by_key(|e| e.1);
        if !eski.is_empty() {
            let satirlar: Vec<String> = eski.iter().map(|(_, _, s)| kirp(s, 200)).collect();
            bolumler.push(format!(
                "Hafızadan, konuya değen eski mesajlar:\n{}",
                satirlar.join("\n")
            ));
        }
    }

    // bütçe: sırayla ekle, dolunca dur
    let mut sonuc = String::new();
    for b in bolumler {
        if sonuc.chars().count() + b.chars().count() > BAGLAM_BUTCESI {
            break;
        }
        if !sonuc.is_empty() {
            sonuc += "\n\n";
        }
        sonuc += &b;
    }
    sonuc
}

// sınırı aşan dosyalar: (tür, dosya yolu)
pub fn sinir_asanlar() -> Vec<(&'static str, PathBuf)> {
    let mut v = Vec::new();
    for p in dosyalar("kisiler") {
        if fs::metadata(&p)
            .map(|m| m.len() as usize > KISI_SINIRI)
            .unwrap_or(false)
        {
            v.push(("kisi", p));
        }
    }
    for p in dosyalar("konular") {
        if fs::metadata(&p)
            .map(|m| m.len() as usize > KONU_SINIRI)
            .unwrap_or(false)
        {
            v.push(("konu", p));
        }
    }
    let olay = yol(&format!("olaylar/{}.md", ay()));
    if fs::metadata(&olay)
        .map(|m| m.len() as usize > OLAY_SINIRI)
        .unwrap_or(false)
    {
        v.push(("olay", olay));
    }
    v
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn tarih_dogru() {
        assert_eq!(tarih_unix(0), "1970-01-01");
        assert_eq!(tarih_unix(1788220800), "2026-09-01");
        assert_eq!(tarih_unix(1788220799), "2026-08-31");
    }

    #[test]
    fn slug_turkce() {
        assert_eq!(slug("Emin Şeyrek"), "emin-seyrek");
        assert_eq!(slug("LNG deniz altı"), "lng-deniz-alti");
        assert_eq!(slug("!!!"), "bilinmeyen");
    }

    #[test]
    fn kisi_gidip_gelir() {
        let k = Kisi {
            id: 259669117248864257,
            isim: "Emin".into(),
            kullanici_adi: "kaju".into(),
            eski_adlar: vec!["eski ad".into()],
            puan: -3,
            etiket: vec!["rust".into(), "oyun".into()],
            not: "laf soktu".into(),
            bilgiler: vec!["yks'ye hazırlanıyor".into()],
            olaylar: vec!["2026-09-01 14:03:22: tartıştık".into()],
        };
        let g = Kisi::coz(k.id, &k.metin());
        assert_eq!(g.id, k.id);
        assert_eq!(g.isim, "Emin");
        assert_eq!(g.kullanici_adi, "kaju");
        assert_eq!(g.eski_adlar, k.eski_adlar);
        assert_eq!(g.puan, -3);
        assert_eq!(g.etiket, k.etiket);
        assert_eq!(g.not, k.not);
        assert_eq!(g.bilgiler, k.bilgiler);
        assert_eq!(g.olaylar, k.olaylar);
    }

    #[test]
    fn anahtar_eler() {
        let a = anahtarlar(&["rust ile bot yazdım abi, bence güzel oldu".to_string()]);
        assert_eq!(a, vec!["rust", "yazdım", "güzel"]);
    }

    #[test]
    fn tarih_saat_bicimi() {
        // YYYY-AA-GG SS:DD:SS (19 karakter, saniyeli)
        let s = tarih_saat();
        assert_eq!(s.chars().count(), 19);
        assert_eq!(&s[4..5], "-");
        assert_eq!(&s[10..11], " ");
        assert_eq!(&s[13..14], ":");
        assert_eq!(&s[16..17], ":");
    }

    #[test]
    fn yaz_ekle_diskte_donup_durur() {
        let parca = "test-gecici.md";
        yaz(parca, "ilk\n");
        ekle(parca, "ikinci");
        let icerik = oku(parca);
        let gecici_kaldi = yol(parca).with_extension("tmp").exists();
        let _ = fs::remove_file(yol(parca));
        assert_eq!(icerik, "ilk\nikinci\n");
        assert!(!gecici_kaldi); // rename sonrası geçici dosya kalmamalı
    }

    #[test]
    fn hafizadan_ceker() {
        let mut h = VecDeque::new();
        h.push_back("emin: rust derleme süresi çok uzun".to_string());
        h.push_back("lng: bugün hava güzel".to_string());
        h.push_back("emin: son mesaj, sohbette zaten var".to_string());
        let mut ad_id = std::collections::HashMap::new();
        ad_id.insert("emin".to_string(), 1u64);
        let g = getir(&[], &ad_id, &["rust".into(), "derleme".into()], &h, 1);
        assert!(g.contains("rust derleme süresi"));
        assert!(!g.contains("hava güzel"));
        assert!(!g.contains("son mesaj"));
    }
}
