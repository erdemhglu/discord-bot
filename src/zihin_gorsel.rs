// !zihin görseli: botun bildikleri modern bir web paneli gibi çizilir. SVG metin
// olarak kurulur, resvg ile PNG'ye rasterize edilir. Tarayıcı, sunucu, dış servis
// yok — görsel botun içinde doğar, Discord'a ek olarak gider.
//
// SVG metni kendi kendine sarmaz: satır kırma, kısaltma ve bütün yerleşim burada
// elle hesaplanır (bkz. `genislik`). Veri hafiza.rs okuyucularından gelir, burada
// yeniden ayrıştırma yapılmaz.

use super::*;
use resvg::tiny_skia;
use resvg::usvg;
use std::cmp::Ordering;
use std::fmt::Write as _;
use std::sync::OnceLock;

pub const CIKTI_ADI: &str = "zihin.png"; // durum/ altına yazılır, her seferinde üstüne

// ---------- ölçüler ----------

const EN: f32 = 1280.0; // tuval genişliği (css px); rasterize ölçekle büyür
const MIN_BOY: f32 = 720.0;
const MAX_BOY: f32 = 2200.0;
// 2x keskin, ama 8 MB'ı aşarsa sırayla düşülür; satır sayıları sınırlı olduğu
// için pratikte ilkinde biter
const OLCEKLER: [f32; 3] = [2.0, 1.5, 1.0];
const PNG_TAVANI: usize = 7_500_000; // discord sınırı 8 MB, pay bırakılır

const CERCEVE: f32 = 44.0; // üstteki tarayıcı şeridi
const KENAR: f32 = 32.0; // sayfa kenar boşluğu
const BOSLUK: f32 = 24.0; // kartlar ve sütunlar arası
const RADYUS: f32 = 14.0; // kart köşesi
const KART_BASLIK: f32 = 52.0; // kart başlık şeridi
const KART_YAN: f32 = 20.0; // kart iç yan boşluğu
const KART_ALT: f32 = 18.0; // kart alt boşluğu
const CHIP_BOY: f32 = 12.0; // bütün chip'lerin yazı boyu
const CHIP_YAN: f32 = 11.0; // chip iç yan boşluğu
const TARIH_SUTUNU: f32 = 96.0; // olay satırında soldaki tarih alanı

// kaç kayıt/kaç satır gösterilir — kartlar bu sabitlerden okur, dağınık sayı yok
const KISI_SATIRI: usize = 8;
const ETIKET_ADEDI: usize = 4;
const OLAY_SATIRI: usize = 8;
const OLAY_METIN_SATIRI: usize = 2;
const KONU_SATIRI: usize = 6;
const KONU_SON_SATIRI: usize = 2;
const GUNDEM_GIRISI: usize = 2;
const GUNDEM_SATIRI: usize = 3;
const KENDIM_SATIRI: usize = 4;
const HUY_SATIRI: usize = 5;
const HUY_METIN_SATIRI: usize = 2;

// ---------- renkler (discord koyu temasıyla uyumlu) ----------

// r#"..."# ham dizgesi "# ile bittigi icin renkler dizgeye sabitten girer
const C_BEYAZ: &str = "#ffffff";
const C_SIYAH: &str = "#000000";
const C_ARKA: &str = "#0f1115";
const C_SERIT: &str = "#12151b";
const C_KART: &str = "#171a21";
const C_METIN: &str = "#e6e8ee";
const C_IKINCIL: &str = "#9aa3b2";
const C_SILIK: &str = "#6b7484";
const C_VURGU: &str = "#7c9cff";
const C_ARTI: &str = "#3ddc97";
const C_EKSI: &str = "#ff6b6b";
const C_ALTIN: &str = "#ffd166";

// ---------- veri ----------

#[derive(Default, Clone)]
pub struct KisiSatiri {
    pub ad: String,
    pub kullanici_adi: String,
    pub puan: i32,
    pub etiketler: Vec<String>,
    pub not: String,
    pub favori: bool,
}

#[derive(Default, Clone)]
pub struct KonuSatiri {
    pub baslik: String,
    pub son_satir: String,
    pub satir_sayisi: usize,
}

#[derive(Default, Clone)]
pub struct OlaySatiri {
    pub tarih: String,
    pub metin: String,
}

#[derive(Default, Clone)]
pub struct GundemGirisi {
    pub tarih: String,
    pub metin: String,
}

// Panelin çizimi için gereken her şey; Durum'dan koparılmış hali. Send olduğu
// için spawn_blocking'e taşınabilir, çizim sırasında kilit tutulmaz.
#[derive(Default, Clone)]
pub struct ZihinVerisi {
    pub bot_adi: String,
    pub evre: String,
    pub gun: i64,
    pub model: String,
    pub kip: String,
    pub uyanik: bool,
    pub ruh_hali: Option<String>,
    pub kisi_sayisi: usize,
    pub konu_sayisi: usize,
    pub olay_sayisi: usize,
    pub gundem_sayisi: usize,
    pub toplam_token: Option<u64>,
    pub kisiler: Vec<KisiSatiri>,
    pub konular: Vec<KonuSatiri>,
    pub olaylar: Vec<OlaySatiri>,
    pub gundem: Vec<GundemGirisi>,
    pub kendim: Vec<String>,
    pub huy: Vec<String>,
    pub tarih_saat: String,
}

// Birinci aşama: yalnız kilit altındaki alanlar kopyalanır, dosyaya dokunulmaz.
// Listeleri `dosyalari_oku` kilit bırakıldıktan sonra doldurur.
pub fn zihin_verisi(d: &Durum) -> ZihinVerisi {
    let g = &d.gelisim;
    let m = &d.metrik;
    let ad = if !d.bot_adi.trim().is_empty() {
        d.bot_adi.clone()
    } else {
        g.isim.clone().unwrap_or_else(|| "bot".to_string())
    };
    ZihinVerisi {
        bot_adi: ad,
        evre: gelisim::evre(g).ad.to_string(),
        gun: gelisim::gun(g) + 1,
        model: d.model.clone(),
        kip: d.dusunme.ad().to_string(),
        uyanik: uyku::uyanik_mi(d),
        // ruh hali sohbet başına tutulur; panelde dolu olan ilki görünür
        ruh_hali: d
            .sohbetler
            .values()
            .map(|s| s.ruh_hali.trim())
            .find(|r| !r.is_empty())
            .map(|r| r.to_string()),
        toplam_token: (m.cagri > 0).then_some(m.giris_token + m.cikis_token),
        kendim: ilk_satirlar(&d.kendim, KENDIM_SATIRI),
        huy: maddeler(&d.huy, HUY_SATIRI),
        gundem: gundem_girisleri(&d.gundem),
        tarih_saat: hafiza::tarih_saat(),
        ..Default::default()
    }
}

// İkinci aşama: durum/ okumaları. Kilit DIŞINDA çağrılır (bkz. AGENTS.md kural 1).
pub fn dosyalari_oku(v: &mut ZihinVerisi) {
    let mut kisiler = hafiza::kisi_dokumleri();
    v.kisi_sayisi = kisiler.len();
    // favori tepede, sonra puan; eşitlikte ad sırası (aynı veri hep aynı görünsün)
    kisiler.sort_by(|a, b| {
        (b.id == FAVORI)
            .cmp(&(a.id == FAVORI))
            .then(b.puan.cmp(&a.puan))
            .then(a.isim.cmp(&b.isim))
    });
    v.kisiler = kisiler
        .into_iter()
        .take(KISI_SATIRI)
        .map(|k| KisiSatiri {
            favori: k.id == FAVORI,
            ad: k.isim,
            kullanici_adi: k.kullanici_adi,
            puan: k.puan,
            etiketler: k.etiket.into_iter().take(ETIKET_ADEDI).collect(),
            not: k.not,
        })
        .collect();

    let konular = hafiza::konu_dokumleri();
    v.konu_sayisi = konular.len();
    v.konular = konular
        .into_iter()
        .take(KONU_SATIRI)
        .map(|(baslik, son)| KonuSatiri {
            satir_sayisi: konu_satir_sayisi(&baslik),
            baslik,
            son_satir: son,
        })
        .collect();

    let aylar = hafiza::olay_aylari(3);
    v.olay_sayisi = aylar.iter().map(|(_, s)| s.len()).sum();
    // en yeni aydan geriye toplanır, gösterim kronolojik olsun diye ters çevrilir
    let mut secilen: Vec<OlaySatiri> = Vec::new();
    for (_, satirlar) in aylar.iter() {
        for satir in satirlar.iter().rev() {
            if secilen.len() >= OLAY_SATIRI {
                break;
            }
            secilen.push(olay_coz(satir));
        }
    }
    secilen.reverse();
    v.olaylar = secilen;

    v.gundem_sayisi = gundem::girisler(&hafiza::oku("gundem.md")).len();
}

// konu dosyasındaki kayıt satırı sayısı; dosya adı slug'dan kurulur, çözülemezse 0
fn konu_satir_sayisi(baslik: &str) -> usize {
    hafiza::oku(&format!("konular/{}.md", hafiza::slug(baslik)))
        .lines()
        .filter(|l| l.starts_with("- "))
        .count()
}

// "- 2026-09-01 22:14:03 #genel: metin" → tarih + gövde. Zaman damgasındaki
// ':' karakterleri yüzünden ayırma kanal işaretinden sonra aranır.
fn olay_coz(satir: &str) -> OlaySatiri {
    let s = satir.trim_start_matches("- ").trim();
    let basi: Vec<char> = s.chars().take(10).collect();
    let tarih = if basi.len() == 10 && basi[4] == '-' && basi[7] == '-' {
        basi.into_iter().collect()
    } else {
        String::new()
    };
    let metin = match s
        .find('#')
        .and_then(|p| s[p..].find(": ").map(|q| &s[p + q + 2..]))
    {
        Some(m) => m.to_string(),
        None if tarih.is_empty() => s.to_string(),
        // biçim tutmuyorsa tarihi tekrar yazmamak için baştaki damga atılır
        None => s.chars().skip(10).collect::<String>().trim().to_string(),
    };
    OlaySatiri { tarih, metin }
}

// gundem.md girişleri: "## 2026-09-01 14:20" başlığı + altındaki günlük
fn gundem_girisleri(metin: &str) -> Vec<GundemGirisi> {
    let g = gundem::girisler(metin);
    let atla = g.len().saturating_sub(GUNDEM_GIRISI);
    g[atla..]
        .iter()
        .map(|giris| {
            let mut satirlar = giris.lines();
            let baslik = satirlar
                .next()
                .unwrap_or_default()
                .trim_start_matches("## ")
                .trim()
                .to_string();
            GundemGirisi {
                tarih: baslik,
                metin: satirlar.collect::<Vec<_>>().join(" ").trim().to_string(),
            }
        })
        .collect()
}

fn ilk_satirlar(metin: &str, adet: usize) -> Vec<String> {
    metin
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .take(adet)
        .map(|l| l.to_string())
        .collect()
}

// huy.md madde listesidir; işaretli satır yoksa düz satırlara düşülür
fn maddeler(metin: &str, adet: usize) -> Vec<String> {
    let isaretli: Vec<String> = metin
        .lines()
        .map(|l| l.trim())
        .filter_map(|l| l.strip_prefix("- ").or_else(|| l.strip_prefix("* ")))
        .filter(|l| !l.trim().is_empty())
        .take(adet)
        .map(|l| l.trim().to_string())
        .collect();
    if isaretli.is_empty() {
        ilk_satirlar(metin, adet)
    } else {
        isaretli
    }
}

// ---------- metin ölçüsü ve sarma ----------

// Inter'de harf genişlikleri em'in kabaca bu oranları. Tahmin bilerek yukarı
// yuvarlar: fazla sarmak, kartı taşırmaktan iyidir.
fn harf_orani(c: char) -> f32 {
    match c {
        'i' | 'ı' | 'l' | 'j' | 'I' | 'İ' | '.' | ',' | ':' | ';' | '!' | '|' | '\'' | '`' => {
            0.28
        }
        ' ' | 'f' | 't' | 'r' | '(' | ')' | '[' | ']' | '-' | '/' => 0.40,
        'm' | 'w' | 'M' | 'W' | '@' | '%' => 0.82,
        '0'..='9' => 0.58,
        c if c.is_uppercase() => 0.62,
        _ => 0.55,
    }
}

fn genislik(metin: &str, boy: f32) -> f32 {
    metin.chars().map(harf_orani).sum::<f32>() * boy
}

// Inter'de olmayan glifler (emoji, oklar, semboller) çizilemez; atılır. Satır
// sonları ve kontrol karakterleri boşluğa iner, art arda boşluk tekleşir.
fn temizle(metin: &str) -> String {
    let mut s = String::with_capacity(metin.len());
    let mut bosluk = true;
    for c in metin.chars() {
        let c = if (c as u32) >= 0x2190 || c.is_control() {
            ' '
        } else {
            c
        };
        if c.is_whitespace() {
            if !bosluk {
                s.push(' ');
            }
            bosluk = true;
        } else {
            s.push(c);
            bosluk = false;
        }
    }
    s.trim_end().to_string()
}

// kelime sınırından sarar; tek başına sığmayan kelime harften bölünür
fn sar(metin: &str, boy: f32, en: f32, max_satir: usize) -> Vec<String> {
    let mut satirlar: Vec<String> = Vec::new();
    let mut simdiki = String::new();
    for kelime in temizle(metin).split_whitespace() {
        if simdiki.is_empty() {
            simdiki = kelime.to_string();
        } else if genislik(&format!("{simdiki} {kelime}"), boy) <= en {
            simdiki.push(' ');
            simdiki.push_str(kelime);
        } else {
            satirlar.push(std::mem::take(&mut simdiki));
            simdiki = kelime.to_string();
        }
        while genislik(&simdiki, boy) > en && simdiki.chars().count() > 1 {
            let mut kesik = String::new();
            for c in simdiki.chars() {
                if genislik(&kesik, boy) + harf_orani(c) * boy > en {
                    break;
                }
                kesik.push(c);
            }
            if kesik.is_empty() {
                break;
            }
            simdiki = simdiki.chars().skip(kesik.chars().count()).collect();
            satirlar.push(kesik);
        }
    }
    if !simdiki.is_empty() {
        satirlar.push(simdiki);
    }
    if satirlar.len() <= max_satir {
        return satirlar;
    }
    satirlar.truncate(max_satir);
    let son = satirlar.pop().unwrap_or_default();
    satirlar.push(uc_nokta(&son, boy, en));
    satirlar
}

// sarmadan tek satıra sığdırır
fn tek_satir(metin: &str, boy: f32, en: f32) -> String {
    let t = temizle(metin);
    if genislik(&t, boy) <= en {
        t
    } else {
        uc_nokta(&t, boy, en)
    }
}

// sona "…" sığdırır; yer açılana kadar kuyruktan harf atar
fn uc_nokta(satir: &str, boy: f32, en: f32) -> String {
    let mut s = satir.trim_end().to_string();
    while !s.is_empty() && genislik(&format!("{s}…"), boy) > en {
        s.pop();
    }
    format!("{}…", s.trim_end())
}

// durum/ içeriği discord kullanıcı metnidir: kaçırmadan svg'ye konmaz
fn kacir(metin: &str) -> String {
    let mut s = String::with_capacity(metin.len());
    for c in metin.chars() {
        match c {
            '&' => s.push_str("&amp;"),
            '<' => s.push_str("&lt;"),
            '>' => s.push_str("&gt;"),
            '"' => s.push_str("&quot;"),
            '\'' => s.push_str("&apos;"),
            c => s.push(c),
        }
    }
    s
}

// 12400 → "12.4k"; istatistik kutusundaki büyük sayı taşmasın
fn sayi_kisalt(n: u64) -> String {
    match n {
        0..=999 => n.to_string(),
        1_000..=999_999 => format!("{:.1}k", n as f64 / 1000.0),
        _ => format!("{:.1}M", n as f64 / 1_000_000.0),
    }
}

// ---------- çizim ilkelleri ----------

// bir metnin biçimi; uzun argüman listesi yerine tek yerde durur
#[derive(Clone, Copy)]
struct Kalem<'a> {
    boy: f32,
    renk: &'a str,
    kalin: bool,
    italik: bool,
    hiza: &'a str,
}

impl<'a> Kalem<'a> {
    fn yeni(boy: f32, renk: &'a str) -> Self {
        Kalem {
            boy,
            renk,
            kalin: false,
            italik: false,
            hiza: "start",
        }
    }
    fn kalin(mut self) -> Self {
        self.kalin = true;
        self
    }
    fn italik(mut self) -> Self {
        self.italik = true;
        self
    }
    fn saga(mut self) -> Self {
        self.hiza = "end";
        self
    }
    fn ortala(mut self) -> Self {
        self.hiza = "middle";
        self
    }
}

// y taban çizgisidir (svg text davranışı), kutunun üstü değil
fn yazi(cikti: &mut String, x: f32, y: f32, metin: &str, k: Kalem) {
    let m = temizle(metin);
    if m.is_empty() {
        return;
    }
    let _ = write!(
        cikti,
        r#"<text x="{x:.1}" y="{y:.1}" font-family="Inter" font-size="{:.1}" font-weight="{}" {}fill="{}" text-anchor="{}">{}</text>"#,
        k.boy,
        if k.kalin { 600 } else { 400 },
        if k.italik {
            r#"font-style="italic" "#
        } else {
            ""
        },
        k.renk,
        k.hiza,
        kacir(&m)
    );
}

fn dolgu(cikti: &mut String, x: f32, y: f32, en: f32, boy: f32, r: f32, renk: &str) {
    let _ = write!(
        cikti,
        r#"<rect x="{x:.1}" y="{y:.1}" width="{en:.1}" height="{boy:.1}" rx="{r:.1}" fill="{renk}"/>"#
    );
}

// kart zemini: koyu yüzey, ince kenarlık, altında hafif gölge
fn kart_zemin(cikti: &mut String, x: f32, y: f32, en: f32, boy: f32) {
    let _ = write!(
        cikti,
        r#"<rect x="{x:.1}" y="{y:.1}" width="{en:.1}" height="{boy:.1}" rx="{RADYUS}" fill="{C_KART}" stroke="{C_BEYAZ}" stroke-opacity="0.08" filter="url(#golge)"/>"#
    );
}

fn ayrac(cikti: &mut String, x: f32, y: f32, en: f32) {
    let _ = write!(
        cikti,
        r#"<rect x="{x:.1}" y="{y:.1}" width="{en:.1}" height="1" fill="{C_BEYAZ}" fill-opacity="0.06"/>"#
    );
}

// yuvarlak köşeli küçük etiket; çizilen genişliği döner ki yanına bir sonraki konsun
fn chip(cikti: &mut String, x: f32, y: f32, metin: &str, renk: &str, nokta: Option<&str>) -> f32 {
    let m = temizle(metin);
    let yuk = CHIP_BOY + 13.0;
    let nokta_yeri = if nokta.is_some() { 13.0 } else { 0.0 };
    let en = genislik(&m, CHIP_BOY) + CHIP_YAN * 2.0 + nokta_yeri;
    let _ = write!(
        cikti,
        r#"<rect x="{x:.1}" y="{y:.1}" width="{en:.1}" height="{yuk:.1}" rx="{:.1}" fill="{C_BEYAZ}" fill-opacity="0.05" stroke="{C_BEYAZ}" stroke-opacity="0.08"/>"#,
        yuk / 2.0
    );
    if let Some(n) = nokta {
        let _ = write!(
            cikti,
            r#"<circle cx="{:.1}" cy="{:.1}" r="3.5" fill="{n}"/>"#,
            x + CHIP_YAN + 3.5,
            y + yuk / 2.0
        );
    }
    yazi(
        cikti,
        x + CHIP_YAN + nokta_yeri,
        y + yuk / 2.0 + CHIP_BOY * 0.35,
        &m,
        Kalem::yeni(CHIP_BOY, renk),
    );
    en
}

// puan rozeti sağa dayanır; kapladığı genişliği döner
fn puan_rozeti(cikti: &mut String, sag_x: f32, y: f32, puan: i32) -> f32 {
    let metin = if puan == 0 {
        "0".to_string()
    } else {
        format!("{puan:+}")
    };
    let boy = 12.5;
    let en = genislik(&metin, boy) + 22.0;
    let yuk = 24.0;
    let x = sag_x - en;
    // dolu yeşil/kırmızı üstüne koyu metin; nötr puan silik gri kalır
    let (zemin, opaklik, yazi_renk) = match puan.cmp(&0) {
        Ordering::Greater => (C_ARTI, "1", C_ARKA),
        Ordering::Less => (C_EKSI, "1", C_ARKA),
        Ordering::Equal => (C_BEYAZ, "0.08", C_IKINCIL),
    };
    let _ = write!(
        cikti,
        r#"<rect x="{x:.1}" y="{y:.1}" width="{en:.1}" height="{yuk}" rx="{}" fill="{zemin}" fill-opacity="{opaklik}"/>"#,
        yuk / 2.0
    );
    yazi(
        cikti,
        x + en / 2.0,
        y + yuk / 2.0 + boy * 0.35,
        &metin,
        Kalem::yeni(boy, yazi_renk).kalin().ortala(),
    );
    en
}

// beş köşeli yıldız: Inter'de ★ glifi yok, şekil elle çizilir
fn yildiz(cikti: &mut String, cx: f32, cy: f32, r: f32, renk: &str) {
    let mut d = String::new();
    for i in 0..10 {
        let aci = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
        let yaricap = if i % 2 == 0 { r } else { r * 0.42 };
        let _ = write!(
            d,
            "{}{:.2} {:.2} ",
            if i == 0 { "M" } else { "L" },
            cx + yaricap * aci.cos(),
            cy + yaricap * aci.sin()
        );
    }
    d.push('Z');
    let _ = write!(cikti, r#"<path d="{d}" fill="{renk}"/>"#);
}

// ---------- kart iskeleti ----------

// başlığı ve (varsa) sayaç chip'ini basar; içerik KART_BASLIK altından başlar
fn kart_basligi(cikti: &mut String, x: f32, y: f32, en: f32, baslik: &str, sayac: Option<usize>) {
    yazi(
        cikti,
        x + KART_YAN,
        y + 33.0,
        baslik,
        Kalem::yeni(15.0, C_METIN).kalin(),
    );
    if let Some(n) = sayac {
        let metin = n.to_string();
        let chip_en = genislik(&metin, CHIP_BOY) + CHIP_YAN * 2.0;
        chip(
            cikti,
            x + en - KART_YAN - chip_en,
            y + 14.0,
            &metin,
            C_IKINCIL,
            None,
        );
    }
}

// boş kart: başlık durur, gövdede silik bir cümle kalır
fn bos_kart(x: f32, y: f32, en: f32, baslik: &str, cumle: &str) -> (String, f32) {
    let yuk = KART_BASLIK + 40.0;
    let mut s = String::new();
    kart_zemin(&mut s, x, y, en, yuk);
    kart_basligi(&mut s, x, y, en, baslik, None);
    yazi(
        &mut s,
        x + KART_YAN,
        y + KART_BASLIK + 18.0,
        cumle,
        Kalem::yeni(13.0, C_SILIK).italik(),
    );
    (s, yuk)
}

// içeriği ve yüksekliğini alıp zemin+başlıkla sarar (zemin içeriğin altında kalmalı)
fn kart_sar(
    x: f32,
    y: f32,
    en: f32,
    baslik: &str,
    sayac: Option<usize>,
    ic: String,
    ic_yuk: f32,
) -> (String, f32) {
    let yuk = KART_BASLIK + ic_yuk + KART_ALT;
    let mut s = String::new();
    kart_zemin(&mut s, x, y, en, yuk);
    kart_basligi(&mut s, x, y, en, baslik, sayac);
    s.push_str(&ic);
    (s, yuk)
}

// ---------- kartlar ----------

fn kart_kisiler(v: &ZihinVerisi, x: f32, y: f32, en: f32) -> (String, f32) {
    if v.kisiler.is_empty() {
        return bos_kart(x, y, en, "Kişiler", "henüz kimseyi tanımıyorum");
    }
    let ic_en = en - KART_YAN * 2.0;
    let mut ic = String::new();
    let mut imlec = y + KART_BASLIK;
    for (i, k) in v.kisiler.iter().enumerate() {
        if i > 0 {
            ayrac(&mut ic, x + KART_YAN, imlec, ic_en);
            imlec += 1.0;
        }
        let ust = imlec + 10.0;
        let rozet_en = puan_rozeti(&mut ic, x + en - KART_YAN, ust - 2.0, k.puan);

        // ad + @kullanıcı + yıldız aynı satırda; rozete çarpmasın diye bütçe kısıtlı
        let butce = ic_en - rozet_en - 16.0;
        let ad = tek_satir(&k.ad, 15.0, butce * 0.7);
        yazi(
            &mut ic,
            x + KART_YAN,
            ust + 14.0,
            &ad,
            Kalem::yeni(15.0, C_METIN).kalin(),
        );
        let mut kalem_x = x + KART_YAN + genislik(&ad, 15.0) + 8.0;
        if !k.kullanici_adi.trim().is_empty() {
            let kalan = x + KART_YAN + butce - kalem_x - if k.favori { 20.0 } else { 0.0 };
            let kadi = tek_satir(&format!("@{}", k.kullanici_adi), 12.5, kalan.max(0.0));
            yazi(
                &mut ic,
                kalem_x,
                ust + 14.0,
                &kadi,
                Kalem::yeni(12.5, C_IKINCIL),
            );
            kalem_x += genislik(&kadi, 12.5) + 8.0;
        }
        if k.favori {
            yildiz(&mut ic, kalem_x + 6.0, ust + 9.0, 7.0, C_ALTIN);
        }
        imlec = ust + 22.0;

        if !k.etiketler.is_empty() {
            let mut cx = x + KART_YAN;
            for e in &k.etiketler {
                let genis = genislik(&temizle(e), CHIP_BOY) + CHIP_YAN * 2.0;
                if cx + genis > x + KART_YAN + ic_en {
                    break;
                }
                cx += chip(&mut ic, cx, imlec, e, C_VURGU, None) + 6.0;
            }
            imlec += CHIP_BOY + 13.0 + 6.0;
        }
        if !k.not.trim().is_empty() {
            yazi(
                &mut ic,
                x + KART_YAN,
                imlec + 12.0,
                &tek_satir(&k.not, 12.5, ic_en),
                Kalem::yeni(12.5, C_IKINCIL).italik(),
            );
            imlec += 20.0;
        }
        imlec += 10.0;
    }
    kart_sar(
        x,
        y,
        en,
        "Kişiler",
        Some(v.kisi_sayisi),
        ic,
        imlec - y - KART_BASLIK,
    )
}

fn kart_olaylar(v: &ZihinVerisi, x: f32, y: f32, en: f32) -> (String, f32) {
    if v.olaylar.is_empty() {
        return bos_kart(x, y, en, "Olaylar", "olay yok");
    }
    let metin_x = x + KART_YAN + TARIH_SUTUNU + 12.0;
    let metin_en = en - KART_YAN * 2.0 - TARIH_SUTUNU - 12.0;
    let mut ic = String::new();
    let mut imlec = y + KART_BASLIK;
    for (i, o) in v.olaylar.iter().enumerate() {
        if i > 0 {
            ayrac(&mut ic, x + KART_YAN, imlec, en - KART_YAN * 2.0);
            imlec += 1.0;
        }
        let ust = imlec + 9.0;
        yazi(
            &mut ic,
            x + KART_YAN,
            ust + 13.0,
            &o.tarih,
            Kalem::yeni(12.5, C_SILIK),
        );
        let satirlar = sar(&o.metin, 13.0, metin_en, OLAY_METIN_SATIRI);
        for (j, s) in satirlar.iter().enumerate() {
            yazi(
                &mut ic,
                metin_x,
                ust + 13.0 + j as f32 * 18.0,
                s,
                Kalem::yeni(13.0, C_METIN),
            );
        }
        imlec = ust + 13.0 + satirlar.len().max(1) as f32 * 18.0 - 4.0;
    }
    kart_sar(
        x,
        y,
        en,
        "Olaylar",
        Some(v.olay_sayisi),
        ic,
        imlec - y - KART_BASLIK,
    )
}

fn kart_konular(v: &ZihinVerisi, x: f32, y: f32, en: f32) -> (String, f32) {
    if v.konular.is_empty() {
        return bos_kart(x, y, en, "Konular", "konu yok");
    }
    let ic_en = en - KART_YAN * 2.0;
    let mut ic = String::new();
    let mut imlec = y + KART_BASLIK;
    for (i, k) in v.konular.iter().enumerate() {
        if i > 0 {
            ayrac(&mut ic, x + KART_YAN, imlec, ic_en);
            imlec += 1.0;
        }
        let ust = imlec + 9.0;
        let mut baslik_en = ic_en;
        if k.satir_sayisi > 0 {
            let metin = format!("{} satır", k.satir_sayisi);
            let chip_en = genislik(&metin, CHIP_BOY) + CHIP_YAN * 2.0;
            chip(
                &mut ic,
                x + en - KART_YAN - chip_en,
                ust - 2.0,
                &metin,
                C_IKINCIL,
                None,
            );
            baslik_en -= chip_en + 10.0;
        }
        yazi(
            &mut ic,
            x + KART_YAN,
            ust + 13.0,
            &tek_satir(&k.baslik, 14.0, baslik_en),
            Kalem::yeni(14.0, C_METIN).kalin(),
        );
        imlec = ust + 20.0;
        if !k.son_satir.trim().is_empty() {
            let satirlar = sar(&k.son_satir, 12.0, ic_en, KONU_SON_SATIRI);
            for (j, s) in satirlar.iter().enumerate() {
                yazi(
                    &mut ic,
                    x + KART_YAN,
                    imlec + 12.0 + j as f32 * 16.0,
                    s,
                    Kalem::yeni(12.0, C_IKINCIL),
                );
            }
            imlec += satirlar.len() as f32 * 16.0;
        }
        imlec += 12.0;
    }
    kart_sar(
        x,
        y,
        en,
        "Konular",
        Some(v.konu_sayisi),
        ic,
        imlec - y - KART_BASLIK,
    )
}

fn kart_gundem(v: &ZihinVerisi, x: f32, y: f32, en: f32) -> (String, f32) {
    if v.gundem.is_empty() {
        return bos_kart(x, y, en, "Gündem", "gündem boş");
    }
    let ic_en = en - KART_YAN * 2.0;
    let mut ic = String::new();
    let mut imlec = y + KART_BASLIK;
    for (i, g) in v.gundem.iter().enumerate() {
        if i > 0 {
            ayrac(&mut ic, x + KART_YAN, imlec, ic_en);
            imlec += 1.0;
        }
        let ust = imlec + 9.0;
        yazi(
            &mut ic,
            x + KART_YAN,
            ust + 12.0,
            &tek_satir(&g.tarih, 12.5, ic_en),
            Kalem::yeni(12.5, C_VURGU).kalin(),
        );
        imlec = ust + 18.0;
        let satirlar = sar(&g.metin, 13.0, ic_en, GUNDEM_SATIRI);
        for (j, s) in satirlar.iter().enumerate() {
            yazi(
                &mut ic,
                x + KART_YAN,
                imlec + 13.0 + j as f32 * 18.0,
                s,
                Kalem::yeni(13.0, C_METIN),
            );
        }
        imlec += satirlar.len() as f32 * 18.0 + 12.0;
    }
    kart_sar(
        x,
        y,
        en,
        "Gündem",
        Some(v.gundem_sayisi),
        ic,
        imlec - y - KART_BASLIK,
    )
}

fn kart_kendim(v: &ZihinVerisi, x: f32, y: f32, en: f32) -> (String, f32) {
    if v.kendim.is_empty() {
        return bos_kart(x, y, en, "Kendim", "kendimi henüz yazmadım");
    }
    let ic_en = en - KART_YAN * 2.0;
    let mut ic = String::new();
    let mut imlec = y + KART_BASLIK;
    // dosya bazen tek uzun paragraf, bazen kısa satırlar: birleştirip sarmak ikisinde de çalışır
    let satirlar = sar(&v.kendim.join(" "), 13.0, ic_en, KENDIM_SATIRI);
    for (j, s) in satirlar.iter().enumerate() {
        yazi(
            &mut ic,
            x + KART_YAN,
            imlec + 13.0 + j as f32 * 20.0,
            s,
            Kalem::yeni(13.0, C_METIN),
        );
    }
    imlec += satirlar.len() as f32 * 20.0;
    kart_sar(x, y, en, "Kendim", None, ic, imlec - y - KART_BASLIK + 4.0)
}

fn kart_huy(v: &ZihinVerisi, x: f32, y: f32, en: f32) -> (String, f32) {
    if v.huy.is_empty() {
        return bos_kart(x, y, en, "Huyum", "huyum daha oturmadı");
    }
    let metin_x = x + KART_YAN + 14.0;
    let metin_en = en - KART_YAN * 2.0 - 14.0;
    let mut ic = String::new();
    let mut imlec = y + KART_BASLIK;
    for m in v.huy.iter() {
        let satirlar = sar(m, 12.5, metin_en, HUY_METIN_SATIRI);
        let _ = write!(
            ic,
            r#"<circle cx="{:.1}" cy="{:.1}" r="3" fill="{C_VURGU}"/>"#,
            x + KART_YAN + 3.0,
            imlec + 9.0
        );
        for (j, s) in satirlar.iter().enumerate() {
            yazi(
                &mut ic,
                metin_x,
                imlec + 13.0 + j as f32 * 17.0,
                s,
                Kalem::yeni(12.5, C_METIN),
            );
        }
        imlec += satirlar.len().max(1) as f32 * 17.0 + 7.0;
    }
    kart_sar(x, y, en, "Huyum", None, ic, imlec - y - KART_BASLIK)
}

// ---------- üst bölümler ----------

// macOS pencere düğmeleri + soluk adres pili: "tarayıcıda açılmış sayfa" hissi
fn tarayici_serit(cikti: &mut String, v: &ZihinVerisi) {
    dolgu(cikti, 0.0, 0.0, EN, CERCEVE, 0.0, C_SERIT);
    let _ = write!(
        cikti,
        r#"<rect x="0" y="{:.1}" width="{EN}" height="1" fill="{C_BEYAZ}" fill-opacity="0.08"/>"#,
        CERCEVE - 1.0
    );
    for (i, renk) in ["#ff5f57", "#febc2e", "#28c840"].iter().enumerate() {
        let _ = write!(
            cikti,
            r#"<circle cx="{:.1}" cy="22" r="6" fill="{renk}" fill-opacity="0.85"/>"#,
            24.0 + i as f32 * 20.0
        );
    }
    let metin = format!("zihin · {}", v.bot_adi);
    let pil_en = (genislik(&metin, 12.0) + 60.0).max(300.0);
    let pil_x = (EN - pil_en) / 2.0;
    let _ = write!(
        cikti,
        r#"<rect x="{pil_x:.1}" y="10" width="{pil_en:.1}" height="24" rx="12" fill="{C_ARKA}" stroke="{C_BEYAZ}" stroke-opacity="0.07"/>"#
    );
    yazi(
        cikti,
        EN / 2.0,
        26.5,
        &metin,
        Kalem::yeni(12.0, C_SILIK).ortala(),
    );
}

// bot adı + durum chip'leri; chip'ler sığmazsa alt satıra sarar. Yüksekliği döner.
fn baslik_blogu(cikti: &mut String, v: &ZihinVerisi, y: f32) -> f32 {
    let ic_en = EN - KENAR * 2.0;
    yazi(
        cikti,
        KENAR,
        y + 26.0,
        &tek_satir(&v.bot_adi, 28.0, ic_en * 0.6),
        Kalem::yeni(28.0, C_METIN).kalin(),
    );
    yazi(
        cikti,
        EN - KENAR,
        y + 24.0,
        &v.tarih_saat,
        Kalem::yeni(13.0, C_IKINCIL).saga(),
    );

    let mut etiketler: Vec<(String, &str, Option<&str>)> = vec![
        (v.evre.clone(), C_VURGU, None),
        (format!("{}. gün", v.gun), C_IKINCIL, None),
        (v.model.clone(), C_IKINCIL, None),
        (format!("düşünme: {}", v.kip), C_IKINCIL, None),
    ];
    if v.uyanik {
        etiketler.push(("uyanık".to_string(), C_METIN, Some(C_ARTI)));
    } else {
        etiketler.push(("uyuyor".to_string(), C_IKINCIL, Some(C_SILIK)));
    }
    if let Some(r) = &v.ruh_hali {
        etiketler.push((r.clone(), C_ALTIN, None));
    }

    let chip_yuk = CHIP_BOY + 13.0;
    let mut cx = KENAR;
    let mut cy = y + 44.0;
    for (metin, renk, nokta) in &etiketler {
        let genis = genislik(&temizle(metin), CHIP_BOY)
            + CHIP_YAN * 2.0
            + if nokta.is_some() { 13.0 } else { 0.0 };
        if cx > KENAR && cx + genis > EN - KENAR {
            cx = KENAR;
            cy += chip_yuk + 8.0;
        }
        cx += chip(cikti, cx, cy, metin, renk, *nokta) + 8.0;
    }
    cy + chip_yuk - y
}

// beş kutuluk sayaç şeridi
fn istatistik(cikti: &mut String, v: &ZihinVerisi, y: f32) -> f32 {
    let kutular = [
        ("Kişiler", v.kisi_sayisi.to_string()),
        ("Konular", v.konu_sayisi.to_string()),
        ("Olaylar", v.olay_sayisi.to_string()),
        ("Gündem", v.gundem_sayisi.to_string()),
        (
            "Token",
            v.toplam_token
                .map(sayi_kisalt)
                .unwrap_or_else(|| "—".into()),
        ),
    ];
    let ara = 16.0;
    let en = (EN - KENAR * 2.0 - ara * (kutular.len() as f32 - 1.0)) / kutular.len() as f32;
    let yuk = 84.0;
    for (i, (etiket, deger)) in kutular.iter().enumerate() {
        let x = KENAR + i as f32 * (en + ara);
        kart_zemin(cikti, x, y, en, yuk);
        yazi(
            cikti,
            x + 18.0,
            y + 42.0,
            deger,
            Kalem::yeni(26.0, C_METIN).kalin(),
        );
        yazi(
            cikti,
            x + 18.0,
            y + 64.0,
            etiket,
            Kalem::yeni(12.0, C_IKINCIL),
        );
    }
    yuk
}

// ---------- svg ----------

pub fn zihin_svg(v: &ZihinVerisi) -> String {
    let mut govde = String::new();
    let mut y = CERCEVE + 28.0;
    y += baslik_blogu(&mut govde, v, y) + BOSLUK;
    y += istatistik(&mut govde, v, y) + BOSLUK;

    // 7/12 - 5/12 ızgara: solda geniş kartlar (kişiler, olaylar), sağda dar olanlar
    let ic_en = EN - KENAR * 2.0;
    let sol_en = ((ic_en - BOSLUK) * 7.0 / 12.0).round();
    let sag_en = ic_en - BOSLUK - sol_en;
    let sag_x = KENAR + sol_en + BOSLUK;

    let mut sol_y = y;
    for kart in [kart_kisiler, kart_olaylar] {
        let (s, h) = kart(v, KENAR, sol_y, sol_en);
        govde.push_str(&s);
        sol_y += h + BOSLUK;
    }
    let mut sag_y = y;
    for kart in [kart_konular, kart_gundem, kart_kendim, kart_huy] {
        let (s, h) = kart(v, sag_x, sag_y, sag_en);
        govde.push_str(&s);
        sag_y += h + BOSLUK;
    }

    // satır sayıları sabitlerle kısıtlı; MAX_BOY yine de güvenlik freni
    let boy = (sol_y.max(sag_y) + 30.0).clamp(MIN_BOY, MAX_BOY);
    let mut alt = String::new();
    yazi(
        &mut alt,
        KENAR,
        boy - 20.0,
        &format!("{} · zihin · {}", v.bot_adi, v.tarih_saat),
        Kalem::yeni(12.0, C_SILIK),
    );

    let mut serit = String::new();
    tarayici_serit(&mut serit, v);

    // gölge tek filtrede tanımlanır, bütün kartlar onu kullanır
    let golge = format!(
        r#"<defs><filter id="golge" x="-25%" y="-25%" width="150%" height="150%"><feDropShadow dx="0" dy="6" stdDeviation="9" flood-color="{C_SIYAH}" flood-opacity="0.35"/></filter></defs>"#
    );
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{EN:.0}" height="{boy:.0}" viewBox="0 0 {EN:.0} {boy:.0}">{golge}<rect width="{EN:.0}" height="{boy:.0}" fill="{C_ARKA}"/>{serit}{govde}{alt}</svg>"#
    )
}

// ---------- rasterize ----------

// gömülü Inter; süreç başına bir kez kurulur
fn fontlar() -> Arc<usvg::fontdb::Database> {
    static DB: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = usvg::fontdb::Database::new();
        db.load_font_data(include_bytes!("../fonts/Inter-Regular.ttf").to_vec());
        db.load_font_data(include_bytes!("../fonts/Inter-SemiBold.ttf").to_vec());
        db.load_font_data(include_bytes!("../fonts/Inter-Italic.ttf").to_vec());
        // gömme bozulursa panel yazısız kalmasın
        if db.is_empty() {
            log::warn!("gömülü Inter yüklenemedi, sistem fontlarına düşülüyor");
            db.load_system_fonts();
        }
        Arc::new(db)
    })
    .clone()
}

pub fn zihin_png(v: &ZihinVerisi) -> Result<Vec<u8>, Hata> {
    let svg = zihin_svg(v);
    let secenek = usvg::Options {
        font_family: "Inter".to_string(),
        font_size: 14.0,
        fontdb: fontlar(),
        ..usvg::Options::default()
    };
    let agac = usvg::Tree::from_str(&svg, &secenek)?;
    let mut png = Vec::new();
    for olcek in OLCEKLER {
        png = rasterize(&agac, olcek)?;
        if png.len() <= PNG_TAVANI {
            return Ok(png);
        }
    }
    log::warn!("zihin görseli 1x'te bile {} bayt", png.len());
    Ok(png)
}

fn rasterize(agac: &usvg::Tree, olcek: f32) -> Result<Vec<u8>, Hata> {
    let en = (agac.size().width() * olcek).round() as u32;
    let boy = (agac.size().height() * olcek).round() as u32;
    let mut tuval = tiny_skia::Pixmap::new(en, boy).ok_or("pixmap kurulamadı")?;
    resvg::render(
        agac,
        tiny_skia::Transform::from_scale(olcek, olcek),
        &mut tuval.as_mut(),
    );
    Ok(tuval.encode_png()?)
}

// `cargo run -- zihin`: discord'suz üretim — tasarım botu çalıştırmadan görülür
pub fn cli_zihin() -> Result<(), Hata> {
    std::fs::create_dir_all(DURUM_KLASORU)?;
    let mut d = Durum::yukle();
    uyku::guncelle(&mut d);
    let secili = hafiza::oku("model.md");
    d.model = if secili.trim().is_empty() {
        "model yok".to_string()
    } else {
        secili.trim().to_string()
    };
    let mut veri = zihin_verisi(&d);
    dosyalari_oku(&mut veri);
    let png = zihin_png(&veri)?;
    let yol = hafiza::yol(CIKTI_ADI);
    std::fs::write(&yol, &png)?;
    println!("yazıldı: {} ({} bayt)", yol.display(), png.len());
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    fn ornek() -> ZihinVerisi {
        ZihinVerisi {
            bot_adi: "Zeytin".into(),
            evre: "yerlesik".into(),
            gun: 12,
            model: "openai/gpt-4o-mini".into(),
            kip: "göster".into(),
            uyanik: true,
            ruh_hali: Some("kafa karışıklığı (6)".into()),
            kisi_sayisi: 5,
            konu_sayisi: 3,
            olay_sayisi: 8,
            gundem_sayisi: 2,
            toplam_token: Some(128_400),
            kisiler: vec![
                KisiSatiri {
                    ad: "Emin".into(),
                    kullanici_adi: "kaju".into(),
                    puan: 10,
                    etiketler: vec!["rust".into(), "yks".into()],
                    not: "canın ciğerin".into(),
                    favori: true,
                },
                KisiSatiri {
                    ad: "Şükrü Işıldağ".into(),
                    kullanici_adi: "sukru".into(),
                    puan: -5,
                    etiketler: vec!["troll".into()],
                    not: "sürekli <script> yazıp \"hackledim\" diyor & gülüyor".into(),
                    favori: false,
                },
            ],
            konular: vec![KonuSatiri {
                baslik: "otosaray projesi".into(),
                son_satir: "emin veri toplamaya karar verdi".into(),
                satir_sayisi: 4,
            }],
            olaylar: vec![OlaySatiri {
                tarih: "2026-09-01".into(),
                metin: "lng ve emin bota hacklenme şakası yaptırdı".into(),
            }],
            gundem: vec![GundemGirisi {
                tarih: "2026-09-01 14:20".into(),
                metin: "bugün rust'ın borrow checker'ına takıldım".into(),
            }],
            kendim: vec!["biraz yorgunum".into()],
            huy: vec!["kısa cevap veriyorum".into()],
            tarih_saat: "2026-09-02 21:00:00".into(),
        }
    }

    #[test]
    fn metin_sarar() {
        let satirlar = sar(
            "bir iki üç dört beş altı yedi sekiz dokuz on",
            13.0,
            90.0,
            3,
        );
        assert!(satirlar.len() <= 3);
        // hiçbir satır verilen genişliği aşmaz
        for s in &satirlar {
            assert!(genislik(s, 13.0) <= 90.0, "taşan satır: {s}");
        }
        // sığmayan kuyruk üç noktayla biter
        assert!(satirlar.last().unwrap().ends_with('…'));
        // sığan metin bozulmadan tek satır kalır
        assert_eq!(sar("kısa", 13.0, 300.0, 3), vec!["kısa".to_string()]);
        // boşluksuz uzun kelime harften bölünür, sonsuz döngüye girmez
        assert!(sar(&"a".repeat(200), 13.0, 60.0, 2).len() == 2);
    }

    #[test]
    fn xml_kacisi() {
        assert_eq!(
            kacir(r#"<b>a & "b" 'c'</b>"#),
            "&lt;b&gt;a &amp; &quot;b&quot; &apos;c&apos;&lt;/b&gt;"
        );
        // kullanıcı metni svg'ye ham girmez
        let mut s = String::new();
        yazi(
            &mut s,
            0.0,
            0.0,
            "</text><script>",
            Kalem::yeni(12.0, C_METIN),
        );
        assert!(!s.contains("<script>"));
        assert!(s.contains("&lt;script&gt;"));
    }

    #[test]
    fn emoji_atilir_turkce_kalir() {
        // Inter'de emoji glifi yok; atılmazsa tofu kutu çizilir
        assert_eq!(temizle("selam 🙂 dünya"), "selam dünya");
        assert_eq!(temizle("İşğüöç ıŞĞÜÖÇ"), "İşğüöç ıŞĞÜÖÇ");
        assert_eq!(temizle("  çok\n\n boşluk  "), "çok boşluk");
    }

    #[test]
    fn bos_veri_svg_uretir() {
        // hiç dosya yokken de panel çıkmalı: boş durum cümleleri görünür
        let svg = zihin_svg(&ZihinVerisi::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        assert!(svg.contains("henüz kimseyi tanımıyorum"));
        assert!(svg.contains("olay yok"));
        assert!(svg.contains("gündem boş"));
    }

    #[test]
    fn dolu_veri_png_uretir() {
        let png = zihin_png(&ornek()).expect("png üretilemedi");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert!(png.len() < 8 * 1024 * 1024, "discord sınırı: {}", png.len());
        // gömülü font gerçekten yüklendi mi (yoksa yazılar boş çıkar)
        assert!(!fontlar().is_empty());
    }

    #[test]
    fn olay_satiri_cozulur() {
        let o = olay_coz("- 2026-09-01 22:14:03 #genel: bot kaçtı: yine");
        assert_eq!(o.tarih, "2026-09-01");
        assert_eq!(o.metin, "bot kaçtı: yine");
        // biçim tutmayan satır olduğu gibi kalır
        let b = olay_coz("- düz satır");
        assert!(b.tarih.is_empty());
        assert_eq!(b.metin, "düz satır");
    }
}
