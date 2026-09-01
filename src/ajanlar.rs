// Botun arka planda çalışan ajanları. Hiçbiri kişilikle konuşmaz; hepsi düz analiz yapar
// ve sonucu duruma yazar. Sohbet eden taraf (main.rs) sadece bu sonuçları okur.
//
//   profilci   grubu tanır                      -> profil
//   kanaatci   insanlar hakkında ne düşünüyor   -> kanaatler
//   hoca       nasıl biri olmalı                -> huy
//   elestirmen son sohbette nerede hata yaptı   -> duzeltmeler
//   haberci    hacker news'ten ne atmalı        -> seçilen haber
//   resimci    ekteki görsele ne demeli         -> tek satır yorum

use super::*;
use crate::promptlar::*;
use base64::Engine;
use serde::Deserialize;
use std::path::PathBuf;

impl Bot {
    pub async fn profilci(&self) {
        let ornek = son_mesajlar(&self.durum(), 600);
        if ornek.is_empty() {
            return;
        }
        match self.analiz(&ornek, PROFIL_CIKAR, 1200).await {
            Ok(yeni) => {
                kaydet("profil.txt", &yeni).await;
                self.durum().profil = yeni;
                println!("profilci: profil güncellendi");
            }
            Err(e) => eprintln!("profilci: {e}"),
        }
    }

    pub async fn kanaatci(&self, dokum: String) {
        if dokum.trim().is_empty() {
            return;
        }
        let (talimat, favori) = {
            let d = self.durum();
            let mevcut = serde_json::to_string_pretty(&d.kanaatler).unwrap_or_default();
            let favori = d.favori_adi.clone();
            let t = KANAAT_GUNCELLE
                .replace("{ad}", &d.bot_adi)
                .replace("{mevcut}", &mevcut)
                .replace("{favori}", favori.as_deref().unwrap_or("kimse"));
            (t, favori)
        };

        let cevap = match self.analiz(&dokum, &talimat, 1500).await {
            Ok(c) => c,
            Err(e) => return eprintln!("kanaatci: {e}"),
        };
        let mut yeni: Kanaatler = match serde_json::from_str(json_ayikla(&cevap)) {
            Ok(k) => k,
            Err(e) => return eprintln!("kanaatci: json çözülemedi: {e}"),
        };

        // model ne derse desin sınırlar bizde
        for k in &mut yeni.kisiler {
            k.puan = k.puan.clamp(-10, 10);
        }
        yeni.kisiler.truncate(30);
        if let Some(f) = &favori {
            yeni.kisiler.retain(|k| &k.isim != f);
            yeni.kisiler.insert(
                0,
                Kanaat {
                    isim: f.clone(),
                    puan: 10,
                    not: "canın ciğerin, ne yaparsa yapsın arkasındasın".into(),
                },
            );
        }

        kaydet(
            "kanaatler.json",
            &serde_json::to_string_pretty(&yeni).unwrap_or_default(),
        )
        .await;
        self.durum().kanaatler = yeni;
        println!("kanaatci: kanaatler güncellendi");
    }

    pub async fn hoca(&self) {
        let (metin, talimat) = {
            let d = self.durum();
            if d.hafiza.is_empty() {
                return;
            }
            let kendi = d
                .kendi_mesajlarim
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join("\n");
            let metin = format!(
                "GRUP PROFİLİ\n{}\n\nKANAATLER\n{}\n\nŞU ANKİ HUYUN\n{}\n\nSON KONUŞMALAR\n{}\n\nBOTUN KENDİ SON MESAJLARI\n{}",
                d.profil,
                serde_json::to_string_pretty(&d.kanaatler).unwrap_or_default(),
                if d.huy.is_empty() { "(henüz yok, ilk kez yazıyorsun)" } else { &d.huy },
                son_mesajlar(&d, 200),
                if kendi.is_empty() { "(henüz konuşmadı)" } else { &kendi },
            );
            (metin, HOCA.replace("{ad}", &d.bot_adi))
        };
        match self.analiz(&metin, &talimat, 800).await {
            Ok(huy) => {
                kaydet("huy.txt", &huy).await;
                self.durum().huy = huy;
                println!("hoca: huy güncellendi");
            }
            Err(e) => eprintln!("hoca: {e}"),
        }
    }

    pub async fn elestirmen(&self, dokum: String) {
        if dokum.trim().is_empty() {
            return;
        }
        let talimat = {
            let d = self.durum();
            ELESTIRMEN
                .replace("{ad}", &d.bot_adi)
                .replace("{mevcut}", &d.duzeltmeler)
        };
        match self.analiz(&dokum, &talimat, 400).await {
            Ok(notlar) => {
                kaydet("duzeltmeler.txt", &notlar).await;
                self.durum().duzeltmeler = notlar;
                println!("elestirmen: notlar güncellendi");
            }
            Err(e) => eprintln!("elestirmen: {e}"),
        }
    }

    pub async fn haberci(&self) -> Result<Haber, Hata> {
        let idler: Vec<u64> = self
            .http
            .get("https://hacker-news.firebaseio.com/v0/topstories.json")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let atilan = self.durum().atilan_haberler.clone();

        let mut haberler: Vec<Haber> = Vec::new();
        let mut liste = String::new();
        for id in idler.into_iter().filter(|id| !atilan.contains(id)).take(15) {
            let adres = format!("https://hacker-news.firebaseio.com/v0/item/{id}.json");
            let h: Haber = match self.http.get(&adres).send().await {
                Ok(r) => r.json().await.unwrap_or_default(),
                Err(_) => continue,
            };
            if h.title.is_empty() {
                continue;
            }
            liste += &format!("{}. {} ({} puan)\n", haberler.len(), h.title, h.score);
            haberler.push(h);
        }
        if haberler.is_empty() {
            return Err("haber bulunamadı".into());
        }

        let profil = self.durum().profil.clone();
        let secim = self
            .analiz(&liste, &HABER_SEC.replace("{profil}", &profil), 10)
            .await?;
        let n: usize = secim
            .chars()
            .filter(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .unwrap_or(0);
        let n = if n < haberler.len() { n } else { 0 };
        Ok(haberler.swap_remove(n))
    }

    // görseli modele gösterip kişilikle tek satır yorum alır; model görsel desteklemiyorsa körlemesine yazar
    pub async fn resimci(&self, yol: &PathBuf) -> Result<String, Hata> {
        let (sistem, bot_adi) = {
            let d = self.durum();
            (sistem_metni(&d, RESIM_AT), d.bot_adi.clone())
        };
        let veri = tokio::fs::read(yol).await?;
        let tur = match yol
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase()
            .as_str()
        {
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            _ => "image/png",
        };
        let b64 = base64::engine::general_purpose::STANDARD.encode(veri);
        let govde = serde_json::json!({
            "model": MODEL,
            "max_tokens": 120,
            "messages": [
                {"role": "system", "content": sistem},
                {"role": "user", "content": [
                    {"type": "text", "text": "görsel ekte"},
                    {"type": "image_url", "image_url": {"url": format!("data:{tur};base64,{b64}")}}
                ]}
            ]
        });
        let cevap = match self.sor_ham(govde).await {
            Ok(c) => c,
            Err(_) => {
                self.uret(
                    &[kullanici(
                        "bir görsel atıyorsun ama ne olduğunu hatırlamıyorsun",
                    )],
                    RESIM_AT,
                    120,
                )
                .await?
            }
        };
        Ok(temizle(cevap, &bot_adi))
    }
}

#[derive(Deserialize, Default)]
pub struct Haber {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub score: i64,
}

// resimler/ klasöründen rastgele bir görsel
pub fn rastgele_resim() -> Option<PathBuf> {
    let dosyalar: Vec<PathBuf> = std::fs::read_dir(RESIM_KLASORU)
        .ok()?
        .flatten()
        .map(|d| d.path())
        .filter(|p| {
            let uz = p
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            matches!(uz.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp")
        })
        .collect();
    if dosyalar.is_empty() {
        return None;
    }
    let i = rand::random::<usize>() % dosyalar.len();
    Some(dosyalar[i].clone())
}
