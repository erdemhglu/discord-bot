// Botun arka planda çalışan ajanları. Hiçbiri kişilikle konuşmaz; hepsi düz analiz yapar
// ve sonucu durum/ klasörüne yazar. Sohbet eden taraf (main.rs) sadece bu sonuçları okur.
//
//   profilci    grubu tanır                       -> profil.md
//   gunlukcu    biten sohbetten hafıza kaydı       -> kisiler/, konular/, olaylar/, kendim.md
//   hoca        nasıl biri olmalı                 -> huy.md
//   elestirmen  son sohbette nerede hata yaptı    -> duzeltmeler.md
//   ozetleyici  sınırı aşan dosyayı küçültür      -> dosya küçülür, taşan arsiv/'e gider
//   haberci     hacker news'ten ne atmalı         -> seçilen haber
//   resimci     ekteki görsele ne demeli          -> tek satır yorum
//   gezgin      (gundem.rs) internette gezer, görüşünü yazar -> gundem.md

use super::*;
use crate::hafiza::{self, Kisi};
use crate::promptlar::*;
use base64::Engine;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Default)]
struct Kayit {
    #[serde(default)]
    olay: String,
    #[serde(default)]
    kisiler: Vec<KisiKaydi>,
    #[serde(default)]
    konular: Vec<KonuKaydi>,
    #[serde(default)]
    kendim: String,
}
#[derive(Deserialize, Default)]
struct KisiKaydi {
    #[serde(default)]
    isim: String,
    #[serde(default)]
    puan_degisimi: i32,
    #[serde(default)]
    not: String,
    #[serde(default)]
    bilgiler: Vec<String>,
    #[serde(default)]
    etiketler: Vec<String>,
}
#[derive(Deserialize, Default)]
struct KonuKaydi {
    #[serde(default)]
    ad: String,
    #[serde(default)]
    not: String,
}

impl Bot {
    pub async fn profilci(&self) {
        let ornek = son_mesajlar(&self.durum(), 600);
        if ornek.is_empty() {
            return;
        }
        match self.analiz(&ornek, PROFIL_CIKAR, 1200).await {
            Ok(yeni) => {
                hafiza::yaz("profil.md", &yeni);
                self.durum().profil = yeni;
                println!("profilci: profil güncellendi");
            }
            Err(e) => eprintln!("profilci: {e}"),
        }
    }

    // biten sohbetten (ya da 6 saatlik gözlemden) hafızaya yazılacakları çıkarır ve dosyalara işler
    pub async fn gunlukcu(&self, dokum: String, kaynak: &str, kanal: &str) {
        if dokum.trim().is_empty() {
            return;
        }
        let (talimat, favori, bot_adi) = {
            let d = self.durum();
            let t = GUNLUKCU
                .replace("{ad}", &d.bot_adi)
                .replace("{kaynak}", kaynak)
                .replace("{favori}", d.favori_adi.as_deref().unwrap_or("kimse"));
            (t, d.favori_adi.clone(), d.bot_adi.clone())
        };
        let cevap = match self.analiz(&dokum, &talimat, 1200).await {
            Ok(c) => c,
            Err(e) => return eprintln!("gunlukcu: {e}"),
        };
        let kayit: Kayit = match serde_json::from_str(json_ayikla(&cevap)) {
            Ok(k) => k,
            Err(e) => return eprintln!("gunlukcu: json çözülemedi: {e}"),
        };

        if !kayit.olay.is_empty() {
            hafiza::olay_ekle(kanal, &kayit.olay);
        }
        for kk in kayit.kisiler {
            if kk.isim.is_empty() || kk.isim.eq_ignore_ascii_case(&bot_adi) {
                continue;
            }
            let mut k: Kisi = hafiza::kisi_oku(&kk.isim);
            // model ne derse desin sınırlar bizde
            k.puan = (k.puan + kk.puan_degisimi.clamp(-3, 3)).clamp(-10, 10);
            if !kk.not.trim().is_empty() {
                k.not = kk.not.trim().to_string();
            }
            for b in kk.bilgiler {
                let b = b.trim().to_string();
                if !b.is_empty() && !k.bilgiler.contains(&b) {
                    k.bilgiler.push(b);
                }
            }
            for e in kk.etiketler {
                let e = e.trim().to_lowercase();
                if !e.is_empty() && !k.etiket.contains(&e) {
                    k.etiket.push(e);
                }
            }
            k.etiket.truncate(6);
            if !kayit.olay.is_empty() {
                k.olaylar
                    .push(format!("{}: {}", hafiza::tarih(), kayit.olay));
            }
            if favori.as_deref() == Some(k.isim.as_str()) {
                k.puan = 10;
                k.not = hafiza::FAVORI_NOTU.to_string();
            }
            hafiza::kisi_yaz(&k);
        }
        for kn in kayit.konular {
            if !kn.ad.trim().is_empty() && !kn.not.trim().is_empty() {
                hafiza::konu_ekle(kn.ad.trim(), &kn.not);
            }
        }
        if !kayit.kendim.trim().is_empty() {
            hafiza::yaz("kendim.md", kayit.kendim.trim());
            self.durum().kendim = kayit.kendim.trim().to_string();
        }
        self.durum().dizin = hafiza::dizin_yenile();
        println!("gunlukcu: {kaynak} kaydedildi");

        self.ozetleyici().await;
    }

    // sınırı aşan dosyaları küçültür; çıkan ham parça arşive gider
    pub async fn ozetleyici(&self) {
        for (tur, yol) in hafiza::sinir_asanlar() {
            let eski = std::fs::read_to_string(&yol).unwrap_or_default();
            let parca = yol
                .strip_prefix(DURUM_KLASORU)
                .unwrap_or(&yol)
                .to_string_lossy()
                .to_string();

            let sonuc = match tur {
                "kisi" => {
                    self.analiz(
                        &eski,
                        &OZETLEYICI_KISI.replace("{sinir}", &hafiza::KISI_HEDEF.to_string()),
                        700,
                    )
                    .await
                }
                "konu" => {
                    self.analiz(
                        &eski,
                        &OZETLEYICI_KONU.replace("{sinir}", &hafiza::KONU_HEDEF.to_string()),
                        600,
                    )
                    .await
                }
                _ => {
                    // olay dosyası: eski yarısı özetlenir, yeni yarısı olduğu gibi kalır
                    let ozetler: Vec<&str> =
                        eski.lines().filter(|l| !l.starts_with("- ")).collect();
                    let satirlar: Vec<&str> =
                        eski.lines().filter(|l| l.starts_with("- ")).collect();
                    let kes = satirlar.len() * 6 / 10;
                    let (eskiler, yeniler) = satirlar.split_at(kes);
                    match self
                        .analiz(&eskiler.join("\n"), OZETLEYICI_OLAYLAR, 400)
                        .await
                    {
                        Ok(ozet) => {
                            hafiza::arsivle(&parca, &eskiler.join("\n"));
                            Ok(format!(
                                "{}\n{}\n\n{}\n",
                                ozetler.join("\n").trim(),
                                ozet.trim(),
                                yeniler.join("\n")
                            ))
                        }
                        Err(e) => Err(e),
                    }
                }
            };

            match sonuc {
                Ok(yeni) if !yeni.trim().is_empty() && yeni.len() < eski.len() => {
                    if tur != "olay" {
                        hafiza::arsivle(&parca, &eski);
                    }
                    hafiza::yaz(&parca, yeni.trim_end());
                    println!(
                        "ozetleyici: {parca} {} -> {} karakter",
                        eski.len(),
                        yeni.len()
                    );
                }
                Ok(_) => eprintln!("ozetleyici: {parca} küçülmedi, bırakıldı"),
                Err(e) => eprintln!("ozetleyici: {parca}: {e}"),
            }
        }
        self.durum().dizin = hafiza::dizin_yenile();
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
                "GRUP PROFİLİ\n{}\n\nKİŞİ DİZİNİ\n{}\n\nGÜNDEM NOTLARI (internette okuyup düşündükleri)\n{}\n\nBOTUN SON HALİ\n{}\n\nŞU ANKİ HUYUN\n{}\n\nSON KONUŞMALAR\n{}\n\nBOTUN KENDİ SON MESAJLARI\n{}",
                d.profil,
                d.dizin,
                if d.gundem.is_empty() { "(henüz gezmedi)" } else { &d.gundem },
                if d.kendim.is_empty() { "(bir şey yok)" } else { &d.kendim },
                if d.huy.is_empty() { "(henüz yok, ilk kez yazıyorsun)" } else { &d.huy },
                son_mesajlar(&d, 200),
                if kendi.is_empty() { "(henüz konuşmadı)" } else { &kendi },
            );
            (metin, HOCA.replace("{ad}", &d.bot_adi))
        };
        match self.analiz(&metin, &talimat, 800).await {
            Ok(huy) => {
                hafiza::yaz("huy.md", &huy);
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
                hafiza::yaz("duzeltmeler.md", &notlar);
                self.durum().duzeltmeler = notlar;
                println!("elestirmen: notlar güncellendi");
            }
            Err(e) => eprintln!("elestirmen: {e}"),
        }
    }

    // iki kaynaktan haber toplar (hacker news + Türkiye gündemi), profile göre birini seçer
    pub async fn haberci(&self) -> Result<Haber, Hata> {
        let atilan = self.durum().atilan_haberler.clone();
        let mut haberler: Vec<Haber> = Vec::new();

        let idler: Vec<u64> = self
            .http
            .get("https://hacker-news.firebaseio.com/v0/topstories.json")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .unwrap_or_default();
        for id in idler.into_iter().filter(|id| !atilan.contains(id)).take(12) {
            let adres = format!("https://hacker-news.firebaseio.com/v0/item/{id}.json");
            let mut h: Haber = match self.http.get(&adres).send().await {
                Ok(r) => r.json().await.unwrap_or_default(),
                Err(_) => continue,
            };
            if h.title.is_empty() {
                continue;
            }
            h.kaynak = "hn";
            haberler.push(h);
        }

        match gundem::rss(&self.http).await {
            Ok(rss) => {
                for r in rss.into_iter().take(12) {
                    let id = gundem::kimlik(&r.link);
                    if !atilan.contains(&id) {
                        haberler.push(Haber {
                            id,
                            title: r.baslik,
                            url: r.link,
                            score: 0,
                            kaynak: "gündem",
                        });
                    }
                }
            }
            Err(e) => eprintln!("haberci: rss: {e}"),
        }
        if haberler.is_empty() {
            return Err("haber bulunamadı".into());
        }

        let liste = haberler
            .iter()
            .enumerate()
            .map(|(i, h)| format!("{i}. [{}] {}", h.kaynak, h.title))
            .collect::<Vec<_>>()
            .join("\n");
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
            let (sabit, degisken) = sistem_metni(&d, RESIM_AT, "");
            (format!("{sabit}\n\n{degisken}"), d.bot_adi.clone())
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
            "model": self.durum().model.clone(),
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
                    Some(120),
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
    #[allow(dead_code)]
    pub score: i64,
    #[serde(skip)]
    pub kaynak: &'static str,
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
