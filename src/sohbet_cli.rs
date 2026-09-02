// Terminalden sohbet tezgâhı: discord'a bağlanmadan cevap protokolünü (satır = mesaj,
// `tepki:` emojisi, `-` susma) denemek için. `cargo run -- sohbet` ile açılır; her girdi
// satırı "isim: metin", iki nokta yoksa yazan `emin` sayılır, `!cik` ya da EOF çıkarır.
// Kişilik gerçekçi olsun diye durum/ dosyaları normal şekilde yüklenir, ama buradan
// hiçbir şey diske YAZILMAZ: kanal geçmişi ve hafıza yalnız bellekte tutulur.

use super::*;
use std::io::Write;

// CLI'nin sahte kanalı: gerçek bir discord kanalı değil, sohbet durumunun anahtarı
const CLI_KANAL: u64 = 1;

// kanal geçmişine yalnız BELLEKTE ekler; kanal_not aynı işi yapıp diske de yazıyor,
// tezgâh gerçek durum/kanallar dosyalarını kirletmesin
fn gecmise_ekle(d: &mut Durum, kanal: ChannelId, satir: String) {
    let g = d.kanal_gecmisi.entry(kanal).or_default();
    g.push_back(satir);
    while g.len() > KANAL_GECMIS {
        g.pop_front();
    }
}

// "isim: metin" ayrıştırması; iki nokta yoksa ya da yanı boşsa yazan "emin" sayılır
fn satir_coz(satir: &str) -> (String, String) {
    match satir.split_once(':') {
        Some((isim, metin)) if !isim.trim().is_empty() && !metin.trim().is_empty() => {
            (isim.trim().to_string(), metin.trim().to_string())
        }
        _ => ("emin".to_string(), satir.trim().to_string()),
    }
}

impl Bot {
    pub async fn sohbet_cli(&self) {
        let kanal = ChannelId::new(CLI_KANAL);
        {
            let mut d = self.durum();
            // ready olayı hiç gelmiyor, ad boş kalıyor: seçtiği isim varsa o, yoksa düz "bot"
            // (soy() ve çıktı bir ada dayanır)
            if d.bot_adi.is_empty() {
                d.bot_adi = d.gelisim.isim.clone().unwrap_or_else(|| "bot".to_string());
            }
            sohbet_baslat(&mut d, kanal, None);
        }
        println!("sohbet modu — \"isim: metin\" yaz; çıkmak için !cik (ya da ctrl-d)");
        // stdin bloklayarak okunur: bu kipte arka plan döngüsü yok, ayrı okuyucuya değmez
        let girdi = std::io::stdin();
        let mut ham = String::new();
        loop {
            print!("> ");
            let _ = std::io::stdout().flush();
            ham.clear();
            match girdi.read_line(&mut ham) {
                Ok(0) => break, // EOF
                Ok(_) => {}
                Err(e) => {
                    eprintln!("girdi okunamadı: {e}");
                    break;
                }
            }
            let satir = ham.trim();
            if satir.is_empty() {
                continue;
            }
            if satir == "!cik" {
                break;
            }
            let (isim, metin) = satir_coz(satir);

            // gelen satır: hafıza, kanal geçmişi ve sohbet geçmişi (canlıdaki message ile aynı biçim)
            {
                let mut d = self.durum();
                hatirla(&mut d, &isim, &metin);
                gecmise_ekle(&mut d, kanal, format!("{isim}: {metin}"));
                if let Some(s) = d.sohbetler.get_mut(&kanal) {
                    s.gecmis.push(kullanici(format!("{isim}: {metin}")));
                    if s.gecmis.len() > SOHBET_BOYU {
                        s.gecmis.drain(..s.gecmis.len() - SOHBET_BOYU);
                    }
                }
            }
            let (gecmis, talimat) = {
                let d = self.durum();
                let gecmis = d
                    .sohbetler
                    .get(&kanal)
                    .map(|s| s.gecmis.clone())
                    .unwrap_or_default();
                // soru tavanı canlıdaki gibi: kod ölçer, uygulamayı model yapar
                let talimat = if soru_fazla_mi(&d, kanal) {
                    "Bu sefer soru sorma; düz laf et ya da sus."
                } else {
                    ""
                };
                (gecmis, talimat)
            };

            // stream yok: tezgâhta akış temposu değil, çıkan protokol ölçülüyor
            let uretilen = match self
                .uret(&gecmis, talimat, sohbet_butcesi(), "sohbet")
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    println!("(hata: {e})");
                    continue;
                }
            };
            let bot_adi = self.durum().bot_adi.clone();
            let cevap = cevap_parcala(soy(&uretilen, &bot_adi));
            if cevap.satirlar.is_empty() && cevap.tepki.is_none() {
                println!("{}", if cevap.sus { "(sustu)" } else { "(boş)" });
                continue;
            }
            for s in &cevap.satirlar {
                println!("{bot_adi}: {s}");
            }
            if let Some(emoji) = &cevap.tepki {
                println!("[tepki {emoji}]");
            }
            // geçmişe protokol biçimiyle iter: model bir sonraki turda kendi biçimini görsün
            {
                let mut d = self.durum();
                if let Some(s) = d.sohbetler.get_mut(&kanal) {
                    s.gecmis.push(asistan(cevap.protokol_metni()));
                    s.sayac += 1;
                }
                for s in &cevap.satirlar {
                    gecmise_ekle(&mut d, kanal, format!("{bot_adi}: {s}"));
                }
                if let Some(emoji) = &cevap.tepki {
                    gecmise_ekle(&mut d, kanal, format!("{bot_adi}: tepki: {emoji}"));
                }
            }
        }
        println!("çıkıldı");
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn satir_cozulur() {
        assert_eq!(satir_coz("emin: selam"), ("emin".into(), "selam".into()));
        assert_eq!(
            satir_coz("Zeynep : naber"),
            ("Zeynep".into(), "naber".into())
        );
        // iki nokta yoksa yazan emin
        assert_eq!(satir_coz("selam"), ("emin".into(), "selam".into()));
        // iki noktanın bir yanı boşsa satırın tamamı metin sayılır
        assert_eq!(satir_coz("saat 3:"), ("emin".into(), "saat 3:".into()));
    }

    #[test]
    fn gecmis_bellekte_sinirli() {
        let kanal = ChannelId::new(CLI_KANAL);
        let mut d = Durum::default();
        for i in 0..KANAL_GECMIS + 5 {
            gecmise_ekle(&mut d, kanal, format!("emin: {i}"));
        }
        let g = &d.kanal_gecmisi[&kanal];
        assert_eq!(g.len(), KANAL_GECMIS);
        assert_eq!(g.front().unwrap(), "emin: 5");
    }
}
