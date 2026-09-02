impl Bot {
    // sessiz kalan sohbetleri kapatır: veda mesajı yok, kanal yasağı yok.
    // kapanan sohbetin dökümü günlükçüye ve eleştirmene gider (bellek adımında kuyruğa taşınacak)
    async fn zaman_asimi_kapat(&self, ctx: &Context) {
        let kapananlar: Vec<(ChannelId, Sohbet)> = {
            let mut d = self.durum();
            // sohbeti kalmayan aktivite kayıtlarını temizle
            let aciklar: HashSet<ChannelId> = d.sohbetler.keys().copied().collect();
            d.son_aktivite.retain(|kanal, _| aciklar.contains(kanal));
            let simdi = Instant::now();
            let kapanacak: Vec<ChannelId> = d
                .son_aktivite
                .iter()
                .filter(|(k, t)| {
                    !d.mesgul.contains(k) && simdi.duration_since(**t) >= SOHBET_ZAMAN_ASIMI
                })
                .map(|(k, _)| *k)
                .collect();
            let mut kapananlar = Vec::new();
            for kanal in kapanacak {
                if let Some(s) = sohbet_bitir(&mut d, kanal) {
                    d.son_aktivite.remove(&kanal);
                    kapananlar.push((kanal, s));
                }
            }
            // süresi dolan haber sohbetleri: kimse yorum yazmadıysa (aktivite pencere
            // boyunca yok ya da kayıt zaten düşmüş) sessizce kapanır, harita şişmez
            let haber_dolen: Vec<ChannelId> = d
                .haber_bekleyen
                .iter()
                .filter(|(k, t)| {
                    simdi >= **t
                        && d.son_aktivite
                            .get(k)
                            .is_none_or(|a| simdi.duration_since(*a) >= YORUM_SURESI)
                })
                .map(|(k, _)| *k)
                .collect();
            for kanal in haber_dolen {
                d.haber_bekleyen.remove(&kanal);
                d.sohbetler.remove(&kanal);
                d.son_aktivite.remove(&kanal);
                log::debug!("haber [{kanal}]: yorum gelmedi, sohbet sessizce kapandı");
            }
            kapananlar
        };
        for (kanal, s) in kapananlar {
            let bot_adi = self.durum().bot_adi.clone();
            let dokum_metni = dokum(&s.gecmis, &bot_adi);
            let kanal_adi = kanal.name(ctx).await.unwrap_or_else(|_| kanal.to_string());
            // ajanlar inline değil, bellek döngüsünde işlenir (elestirmen de çalışsın)
            let kuyruk = {
                let mut d = self.durum();
                d.bellek_kuyruk.push_back((
                    dokum_metni,
                    "biten sohbet".to_string(),
                    kanal_adi,
                    true,
                ));
                d.bellek_kuyruk.len()
            };
            let dk = SOHBET_ZAMAN_ASIMI.as_secs() / 60;
            log::info!(
                "zihin: sohbet kapandı [{kanal}] ({dk} dk sessiz) → kuyruk ({kuyruk}), günlükçü 10 dk içinde"
            );
            self.debug_not(ctx, kanal, format!("sohbet kapandı ({dk} dk sessiz)"))
                .await;
            self.durum().gelisim.sohbet += 1;
            self.gelisim_kontrol(ctx).await;
        }
    }
}

// ---------- gelişim ----------

impl Bot {
    // hak edilen evreye atlar, kaydeder; yerleşik olunca isim seçer
    async fn gelisim_kontrol(&self, ctx: &Context) {
        let isim_gerek = {
            let mut d = self.durum();
            let hak = gelisim::hak_edilen(&d.gelisim);
            if hak > d.gelisim.evre {
                d.gelisim.evre = hak;
                log::info!("gelisim: {} evresine geçti", gelisim::evre(&d.gelisim).ad);
            }
            gelisim::kaydet(&d.gelisim);
            d.gelisim.isim.is_none() && d.gelisim.evre >= gelisim::ISIM_EVRESI
        };
        if isim_gerek {
            self.isim_sec(ctx).await;
        }
    }

    // kendine isim seçer, takma adını her sunucuda değiştirir, gruba söyler
    async fn isim_sec(&self, ctx: &Context) {
        let cevap = match self
            .uret(
                &[kullanici("isim seçme vakti")],
                ISIM_SEC,
                Some(12),
                "isim_sec",
            )
            .await
        {
            Ok(c) => c,
            Err(e) => return log::error!("isim: {e}"),
        };
        let Some(isim) = gelisim::isim_temizle(&cevap) else {
            return log::warn!("isim: seçim çözülemedi: {cevap}");
        };
        for gid in ctx.cache.guilds() {
            if let Err(e) = gid.edit_nickname(&ctx.http, Some(&isim)).await {
                log::warn!("isim: takma ad değiştirilemedi ({gid}): {e}");
            }
        }
        {
            let mut d = self.durum();
            d.gelisim.isim = Some(isim.clone());
            d.bot_adi = isim.clone();
            gelisim::kaydet(&d.gelisim);
        }
        log::info!("gelisim: yeni isim {isim}");

        let Some(kanal) = varsayilan_kanal(self, ctx) else {
            return;
        };
        match self
            .uret(
                &[kullanici("ismini seçtin")],
                &ISIM_DUYURU.replace("{isim}", &isim),
                Some(150),
                "isim_duyuru",
            )
            .await
        {
            Ok(duyuru) => match self
                .gonder_satirlar(ctx, kanal, &duyuru, None, None, None)
                .await
            {
                Some(p) => {
                    sohbet_baslat(&mut self.durum(), kanal, Some(p));
                }
                None => log::debug!("isim: duyuruda model sustu, atlandı"),
            },
            Err(e) => log::error!("isim: {e}"),
        }
    }
}

// ---------- hafıza ----------

// sunucuya bağlanınca kanalların son iki haftasını okur
