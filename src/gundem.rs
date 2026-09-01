// Türkiye gündemi: Sözcü RSS'i okur, sayfayı firecrawl ile (yoksa düz indirip) çeker,
// bot okuduklarından kendi görüşünü günlüğüne yazar (durum/gundem.md). Hoca ve her cevap
// bunu okur; kişilik buradan da beslenir. Haberci de aynı RSS'i haber kaynağı olarak kullanır.

use super::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub const RSS_ADRESI: &str = "https://www.sozcu.com.tr/rss/news.xml";
pub const GUNDEM_KAYIT: usize = 12; // gundem.md'de kaç giriş kalır, eskisi arşive
pub const SAYFA_SINIRI: usize = 3500; // bir sayfadan modele giden karakter

pub struct RssHaber {
    pub baslik: String,
    pub link: String,
    pub ozet: String,
}

// html'den düz metin: script/style atılır, etiketler soyulur, boşluklar toplanır
pub fn temiz_html(ham: &str) -> String {
    let mut s = ham.replace("<![CDATA[", "").replace("]]>", "");
    for etiket in ["script", "style"] {
        while let Some(b) = s.find(&format!("<{etiket}")) {
            match s[b..].find(&format!("</{etiket}>")) {
                Some(e) => s.replace_range(b..b + e + etiket.len() + 3, " "),
                None => break,
            }
        }
    }
    let mut out = String::new();
    let mut icinde = false;
    for c in s.chars() {
        match c {
            '<' => icinde = true,
            '>' => icinde = false,
            c if !icinde => out.push(c),
            _ => {}
        }
    }
    let out = out
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn etiket_ici(parca: &str, etiket: &str) -> Option<String> {
    let bas = parca
        .find(&format!("<{etiket}>"))
        .or_else(|| parca.find(&format!("<{etiket} ")))?;
    let bas = bas + parca[bas..].find('>')? + 1;
    let bit = bas + parca[bas..].find(&format!("</{etiket}>"))?;
    Some(temiz_html(&parca[bas..bit]))
}

pub async fn rss(http: &reqwest::Client) -> Result<Vec<RssHaber>, Hata> {
    let xml = http
        .get(RSS_ADRESI)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let mut v = Vec::new();
    for parca in xml.split("<item").skip(1) {
        let parca = parca.split("</item>").next().unwrap_or("");
        let (Some(baslik), Some(link)) = (etiket_ici(parca, "title"), etiket_ici(parca, "link"))
        else {
            continue;
        };
        if baslik.is_empty() || !link.starts_with("http") {
            continue;
        }
        v.push(RssHaber {
            baslik,
            link,
            ozet: etiket_ici(parca, "description").unwrap_or_default(),
        });
    }
    if v.is_empty() {
        return Err("rss boş geldi".into());
    }
    Ok(v)
}

// atılan haberleri hatırlamak için linkten sayı
pub fn kimlik(link: &str) -> u64 {
    let mut h = DefaultHasher::new();
    link.hash(&mut h);
    h.finish()
}

// gundem.md girişleri: her giriş "## tarih saat" satırıyla başlar
pub fn girisler(metin: &str) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for satir in metin.lines() {
        if satir.starts_with("## ") {
            v.push(satir.to_string());
        } else if let Some(son) = v.last_mut() {
            son.push('\n');
            son.push_str(satir);
        }
    }
    v.into_iter()
        .map(|g| g.trim().to_string())
        .filter(|g| !g.is_empty())
        .collect()
}

pub fn son_gundem(metin: &str) -> String {
    let g = girisler(metin);
    let atla = g.len().saturating_sub(3);
    g[atla..].join("\n\n")
}

impl Bot {
    // sayfa metni: firecrawl anahtarı varsa onunla, yoksa düz indirip etiketleri ayıklayarak
    pub async fn sayfa_oku(&self, url: &str) -> Result<String, Hata> {
        let metin = match &self.firecrawl {
            Some(anahtar) => {
                #[derive(Deserialize)]
                struct Yanit {
                    data: Option<Veri>,
                }
                #[derive(Deserialize)]
                struct Veri {
                    markdown: Option<String>,
                }
                let y: Yanit = self
                    .http
                    .post("https://api.firecrawl.dev/v1/scrape")
                    .bearer_auth(anahtar)
                    .json(&serde_json::json!({ "url": url, "formats": ["markdown"], "onlyMainContent": true }))
                    .send()
                    .await?
                    .error_for_status()?
                    .json()
                    .await?;
                y.data
                    .and_then(|d| d.markdown)
                    .ok_or("firecrawl boş döndü")?
            }
            None => temiz_html(
                &self
                    .http
                    .get(url)
                    .send()
                    .await?
                    .error_for_status()?
                    .text()
                    .await?,
            ),
        };
        Ok(metin.chars().take(SAYFA_SINIRI).collect())
    }

    // firecrawl web araması: başlık, açıklama, adres (anahtar yoksa hata)
    pub async fn firecrawl_ara(&self, sorgu: &str) -> Result<String, Hata> {
        let anahtar = self.firecrawl.as_ref().ok_or("firecrawl anahtarı yok")?;
        #[derive(Deserialize)]
        struct Yanit {
            data: Option<Vec<Sonuc>>,
        }
        #[derive(Deserialize, Default)]
        struct Sonuc {
            #[serde(default)]
            title: String,
            #[serde(default)]
            description: String,
            #[serde(default)]
            url: String,
        }
        let y: Yanit = self
            .http
            .post("https://api.firecrawl.dev/v1/search")
            .bearer_auth(anahtar)
            .json(&serde_json::json!({ "query": sorgu, "limit": 5 }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let liste = y
            .data
            .unwrap_or_default()
            .iter()
            .filter(|s| !s.title.is_empty())
            .map(|s| {
                format!(
                    "- {} — {} ({})",
                    s.title,
                    hafiza::kirp(&s.description, 160),
                    s.url
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        if liste.is_empty() {
            return Err("arama boş döndü".into());
        }
        Ok(liste)
    }

    // arada internette gezer: rss'ten ilgisini çekenleri seçer, okur, kendi görüşünü günlüğüne yazar
    pub async fn gezgin(&self) {
        let haberler = match rss(&self.http).await {
            Ok(h) => h,
            Err(e) => return eprintln!("gezgin: {e}"),
        };
        let liste = haberler
            .iter()
            .take(20)
            .enumerate()
            .map(|(i, h)| format!("{i}. {} — {}", h.baslik, hafiza::kirp(&h.ozet, 120)))
            .collect::<Vec<_>>()
            .join("\n");
        let talimat = {
            let d = self.durum();
            GEZGIN_SEC
                .replace("{ad}", &d.bot_adi)
                .replace("{huy}", &d.huy)
                .replace("{profil}", &d.profil)
        };
        let secim = match self.analiz(&liste, &talimat, 20).await {
            Ok(s) => s,
            Err(e) => return eprintln!("gezgin: {e}"),
        };
        let secilen: Vec<usize> = secim
            .split(|c: char| !c.is_ascii_digit())
            .filter_map(|s| s.parse().ok())
            .filter(|n| *n < haberler.len().min(20))
            .take(3)
            .collect();
        if secilen.is_empty() {
            return eprintln!("gezgin: seçim çözülemedi: {secim}");
        }

        let mut okunan = String::new();
        for n in secilen {
            let h = &haberler[n];
            let icerik = match self.sayfa_oku(&h.link).await {
                Ok(m) if !m.trim().is_empty() => m,
                _ => h.ozet.clone(),
            };
            okunan += &format!("## {}\n{}\n{}\n\n", h.baslik, h.link, icerik);
        }

        let not = match self.uret(&[kullanici(okunan)], GEZGIN_NOT, Some(350)).await {
            Ok(n) => n,
            Err(e) => return eprintln!("gezgin: {e}"),
        };

        let mut g = girisler(&hafiza::oku("gundem.md"));
        g.push(format!(
            "## {} {}\n{}",
            hafiza::tarih(),
            uyku::saat(),
            not.trim()
        ));
        while g.len() > GUNDEM_KAYIT {
            hafiza::arsivle("gundem.md", &g.remove(0));
        }
        let metin = g.join("\n\n");
        hafiza::yaz("gundem.md", &metin);
        self.durum().gundem = son_gundem(&metin);
        println!("gezgin: gündem notu yazıldı");
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn rss_parcalanir() {
        let xml = r#"<rss><channel><atom:link href="x"/><item><title><![CDATA[Başlık &amp; devam]]></title>
        <link>https://ornek.com/a</link><description><![CDATA[<p>özet <b>kalın</b></p>]]></description></item>
        <item><title>ikinci</title><link>https://ornek.com/b</link></item></channel></rss>"#;
        let mut v = Vec::new();
        for parca in xml.split("<item").skip(1) {
            let parca = parca.split("</item>").next().unwrap();
            v.push((
                etiket_ici(parca, "title").unwrap(),
                etiket_ici(parca, "link").unwrap(),
                etiket_ici(parca, "description").unwrap_or_default(),
            ));
        }
        assert_eq!(v[0].0, "Başlık & devam");
        assert_eq!(v[0].1, "https://ornek.com/a");
        assert_eq!(v[0].2, "özet kalın");
        assert_eq!(v[1].0, "ikinci");
    }

    #[test]
    fn html_temizlenir() {
        let h = "<html><script>var a=1;</script><body><h1>Selam</h1>  <p>dünya &amp; ötesi</p></body></html>";
        assert_eq!(temiz_html(h), "Selam dünya & ötesi");
    }

    #[test]
    fn gundem_girisleri() {
        let m = "## 2026-09-01 10:00\nbir\niki\n\n## 2026-09-01 14:00\nüç";
        let g = girisler(m);
        assert_eq!(g.len(), 2);
        assert_eq!(g[0], "## 2026-09-01 10:00\nbir\niki");
        assert_eq!(son_gundem(m), m);
    }
}
