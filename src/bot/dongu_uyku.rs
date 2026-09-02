impl Bot {
    // uyudu/uyandı geçişini işler; uyanınca uyurken gelen etiketlere döner
    async fn uyku_gecisi(&self, ctx: &Context) {
        let (bekleyen, uyandi) = {
            let mut d = self.durum();
            let uyanik = uyku::uyanik_mi(&d);
            if uyanik == d.uyuyor {
                log::info!("uyku: {}", if uyanik { "uyandı" } else { "uyudu" });
            }
            let uyandi = uyanik && d.uyuyor;
            let uyudu = !uyanik && !d.uyuyor;
            d.uyuyor = !uyanik;
            if uyudu {
                // gece mesajlarını kesebilmek için başlangıç işaretleri
                d.uyku_basi = simdi_unix();
                d.uyku_basi_hafiza_len = d.hafiza.len();
            }
            if uyandi {
                (std::mem::take(&mut d.bekleyen_etiketler), true)
            } else {
                (Vec::new(), false)
            }
        };

        // geçiş yoksa bu tikte uyanış işi yok
        if !uyandi {
            return;
        }

        if !bekleyen.is_empty() {
            // uyurken etiketlendiyse kesin dönüş: liste hata durumunda geri konur
            let Some(&(kanal, _)) = bekleyen.last() else {
                return;
            };
            let liste = bekleyen
                .iter()
                .map(|(_, m)| format!("- {m}"))
                .collect::<Vec<_>>()
                .join("\n");
            match self
                .uret(
                    &[kullanici(format!("uyurken sana yazılanlar:\n{liste}"))],
                    UYANDIM,
                    Some(200),
                    "uyandim",
                )
                .await
            {
                Ok(c) => match self.gonder_satirlar(ctx, kanal, &c, None, None, None).await {
                    Some(p) => {
                        sohbet_baslat(&mut self.durum(), kanal, Some(p));
                    }
                    None => log::debug!("uyandim: model sustu, atlandı"),
                },
                Err(e) => {
                    log::error!("ai [uyandim]: {e}");
                    let mut d = self.durum();
                    for b in bekleyen {
                        d.bekleyen_etiketler.push(b);
                    }
                }
            }
            return;
        }

        // etiket yoksa gece yazılanları değerlendir: ilgini çeken varsa sabah sözüyle dön
        let gece: Vec<String> = {
            let d = self.durum();
            d.hafiza
                .iter()
                .skip(d.uyku_basi_hafiza_len)
                .cloned()
                .collect()
        };
        if !gece.is_empty() {
            self.uyanis_degerlendir(ctx, &gece).await;
        }
    }

    // gece yazılanlardan botu ilgilendirenleri seçer; eşiğin üstündeyse sabah sözüyle döner
    async fn uyanis_degerlendir(&self, ctx: &Context, gece: &[String]) {
        let gece_metni = gece.join("\n");
        let talimat = {
            let d = self.durum();
            UYANIS.replace("{ad}", &d.bot_adi)
        };
        #[derive(Deserialize)]
        struct UyanisSonuc {
            #[serde(default)]
            ilgi: i32,
            #[serde(default)]
            konu: String,
        }
        let sonuc = match self.analiz(&gece_metni, &talimat, 100, "uyanis").await {
            Ok(c) => serde_json::from_str::<UyanisSonuc>(json_ayikla(&c)),
            Err(e) => {
                log::debug!("uyanis: değerlendirme çağrısı başarısız: {e}");
                return;
            }
        };
        let Ok(s) = sonuc else {
            log::debug!("uyanis: sonuç çözülemedi");
            return;
        };
        log::debug!("uyanis: ilgi={} konu={}", s.ilgi, s.konu);
        if s.ilgi < 5 {
            return;
        }
        let Some(kanal) = self.durum().son_kanal else {
            return;
        };
        if self.durum().sohbetler.contains_key(&kanal) {
            return;
        }
        let talimat = {
            let d = self.durum();
            UYANIS_CEVAP
                .replace("{ad}", &d.bot_adi)
                .replace("{konu}", &s.konu)
        };
        match self
            .uret(
                &[kullanici(format!("sen uyurken yazılanlar:\n{gece_metni}"))],
                &talimat,
                Some(250),
                "uyanis_cevap",
            )
            .await
        {
            Ok(c) => match self.gonder_satirlar(ctx, kanal, &c, None, None, None).await {
                Some(p) => {
                    sohbet_baslat(&mut self.durum(), kanal, Some(p));
                }
                None => log::debug!("uyanis_cevap: model sustu, atlandı"),
            },
            Err(e) => log::error!("ai [uyanis_cevap]: {e}"),
        }
    }
}
