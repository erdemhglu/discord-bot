impl Bot {
    // botun son 5 mesajından biriyle aynı mı
    fn tekrar_mi(&self, kanal: ChannelId, cevap: &str) -> bool {
        let d = self.durum();
        let onek = format!("{}: ", d.bot_adi);
        let hedef = cevap.trim().to_lowercase();
        d.kanal_gecmisi
            .get(&kanal)
            .map(|g| {
                g.iter()
                    .rev()
                    .filter_map(|l| l.strip_prefix(&onek))
                    .take(5)
                    .any(|l| l.trim().to_lowercase() == hedef)
            })
            .unwrap_or(false)
    }

    // mesaj internete bakmayı gerektiriyorsa bakar: link → sayfa; "araştır/bak" → firecrawl arama
    // (anahtar varsa); "haber/gündem/ne oldu" → rss başlıkları
    async fn arastir(&self, metin: &str) -> Option<String> {
        let m = metin.to_lowercase();
        if let Some(url) = metin
            .split_whitespace()
            .find(|w| w.starts_with("http://") || w.starts_with("https://"))
        {
            let url = url.trim_end_matches(['>', ')', ',', '.']);
            return match self.sayfa_oku(url).await {
                Ok(s) if !s.trim().is_empty() => {
                    Some(format!("Atılan link ({url}):\n{}", hafiza::kirp(&s, 1500)))
                }
                _ => Some(format!("Link açılamadı: {url}")),
            };
        }
        let gecen = |liste: &[&str]| liste.iter().any(|k| m.contains(k));
        let haber = gecen(&[
            "haber",
            "gündem",
            "ne oldu",
            "son dakika",
            "neler oluyor",
            "güncel",
        ]);
        let tetik = [
            "araştır",
            "bak bakalım",
            "baksana",
            "bi bak",
            "googlela",
            "ara bakalım",
            "arasana",
            "internete bak",
            "internetten bak",
        ];
        let ara = gecen(&tetik);
        if ara && self.firecrawl.is_some() {
            let mut sorgu = m.clone();
            for k in tetik
                .iter()
                .chain(["bakar mısın", " lan", " la ", " aq"].iter())
            {
                sorgu = sorgu.replace(k, " ");
            }
            let sorgu: String = sorgu
                .split_whitespace()
                .filter(|w| !w.starts_with('@'))
                .collect::<Vec<_>>()
                .join(" ");
            let sorgu = if sorgu.trim().is_empty() {
                metin.to_string()
            } else {
                sorgu
            };
            if let Ok(sonuc) = self.firecrawl_ara(&sorgu).await {
                return Some(format!("\"{sorgu}\" araması:\n{sonuc}"));
            }
        }
        if haber || ara {
            if let Ok(rss) = gundem::rss(&self.http).await {
                let liste = rss
                    .iter()
                    .take(12)
                    .map(|h| format!("- {} — {}", h.baslik, hafiza::kirp(&h.ozet, 100)))
                    .collect::<Vec<_>>()
                    .join("\n");
                return Some(format!("Sözcü'den şu anki başlıklar:\n{liste}"));
            }
        }
        None
    }
}

