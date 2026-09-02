#[derive(Default, Debug, PartialEq, Eq)]
struct Cevap {
    satirlar: Vec<String>,
    tepki: Option<String>,
    sus: bool,
}

impl Cevap {
    // cevaptan kullanılır hiçbir şey çıkmadı mı (ne söz, ne tepki, ne susma kararı)
    fn bos(&self) -> bool {
        self.satirlar.is_empty() && self.tepki.is_none() && !self.sus
    }

    // sohbet geçmişine ve kanal notuna giren biçim: model kendi protokolünü geri görsün,
    // sonraki turda aynı biçimde yazsın
    fn protokol_metni(&self) -> String {
        let mut s = self.satirlar.join("\n");
        if let Some(t) = &self.tepki {
            if !s.is_empty() {
                s.push('\n');
            }
            s.push_str("tepki: ");
            s.push_str(t);
        }
        s
    }
}

// satır "tepki:" ile mi başlıyor (büyük/küçük harf farketmez, "tepki :" de olur);
// öyleyse iki noktadan sonrası döner
fn tepki_govdesi(satir: &str) -> Option<&str> {
    let (bas, kalan) = satir.split_once(':')?;
    (kucult(bas.trim()) == "tepki").then_some(kalan)
}

// karakter gerçekten emoji mi. "harf değilse emojidir" demek yetmiyor: "—", "…", "→",
// tipografik tırnak da o testi geçiyor ve discord'a Unicode tepki olarak gidince istek
// 400 ile dönüyor. Bilinen emoji blokları sayılır, gerisi tepki olmaz.
fn emoji_basi(c: char) -> bool {
    matches!(c as u32,
        0xA9 | 0xAE | 0x2122 | 0x3030 | 0x303D | 0x3297 | 0x3299
        | 0x2600..=0x27BF   // çeşitli semboller + dingbat
        | 0x2B00..=0x2BFF   // ok/yıldız/kare sembolleri (⭐ gibi)
        | 0x1F000..=0x1FAFF // asıl emoji blokları (bayraklar, ten tonu dahil)
    )
}

// dizinin devamına takılabilecekler: varyasyon seçici, ZWJ, keycap
fn emoji_devami(c: char) -> bool {
    emoji_basi(c) || matches!(c as u32, 0xFE0F | 0xFE0E | 0x200D | 0x20E3)
}

// metindeki ilk emoji dizisi; peşine takılan varyasyon seçici / ZWJ de alınır.
// ":kekw:" gibi özel emoji biçiminde ve emoji hiç yoksa None.
fn emoji_ayikla(metin: &str) -> Option<String> {
    let (bas, _) = metin.char_indices().find(|(_, c)| emoji_basi(*c))?;
    Some(
        metin[bas..]
            .chars()
            .take_while(|c| emoji_devami(*c))
            .take(8)
            .collect(),
    )
}

// satır tek başına susma işareti mi
fn sus_isareti(satir: &str) -> bool {
    matches!(satir, "-" | "\"-\"" | "'-'") || matches!(kucult(satir).as_str(), "[sus]" | "(sus)")
}

// "1. " / "2) " gibi numara önekinden sonrası (rakamlar tek baytlık, dilim güvenli).
// Tek başına elenmez: "3. sınıftayım", "2. el araba" Türkçe'de sıra sayısıdır, madde değil.
fn numara_oneki(s: &str) -> Option<&str> {
    let basamak = s.chars().take_while(char::is_ascii_digit).count();
    if basamak == 0 {
        return None;
    }
    let kalan = &s[basamak..];
    kalan
        .strip_prefix(". ")
        .or_else(|| kalan.strip_prefix(") "))
        .map(str::trim_start)
}

// "yapay zeka yazmış" izlerini siler: baştaki madde öneki ve kalın/altı çizili markdown
// işaretleri. Numara öneki burada değil `cevap_parcala`'da elenir (gerçek liste mi sıra
// sayısı mı, ancak cevabın tamamına bakınca anlaşılır). Backtick'in İÇİ de korunur:
// `__init__` gibi kod parçası gerçek bilgi taşır.
fn slop_temizle(satir: &str) -> String {
    let mut s = satir.trim();
    for onek in ["- ", "* ", "• "] {
        if let Some(k) = s.strip_prefix(onek) {
            s = k.trim_start();
            break;
        }
    }
    let mut cikti = String::with_capacity(s.len());
    // backtick ile bölünce tek indeksli parçalar kod içidir, dokunulmaz
    for (i, parca) in s.split('`').enumerate() {
        if i > 0 {
            cikti.push('`');
        }
        if i % 2 == 0 {
            cikti.push_str(&parca.replace("**", "").replace("__", ""));
        } else {
            cikti.push_str(parca);
        }
    }
    cikti.trim().to_string()
}

// Model cevabını protokole göre çözer. Kısa satır elenmez: "he", "yok", "la" da
// doğal bir tepkidir.
