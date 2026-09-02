fn bos_kanal(bot: &Bot) -> Option<(ChannelId, String)> {
    let d = bot.durum();
    let k = d.son_kanal?;
    if d.sohbetler.contains_key(&k) || d.profil.is_empty() {
        return None;
    }
    Some((k, son_mesajlar(&d, 40)))
}

// arada bir, tanıdık biri gibi durup dururken laf atar
async fn durtme_dongusu(bot: Arc<Bot>, ctx: Context) {
    loop {
        sleep(DURTME_ARALIGI).await;
        if KAPANIYOR.load(Ordering::SeqCst) {
            return;
        }
        if !uyku::uyanik_mi(&bot.durum()) {
            continue;
        }
        // seyahat: gitmeden bir gün önce haber ver, yoldayken günde bir mesaj, başka laf atma
        let talimat = if let Some(s) = seyahat::simdi() {
            if bot.durum().son_yol_mesaji == seyahat::bugun() || rand::random::<f64>() > 0.25 {
                continue;
            }
            bot.durum().son_yol_mesaji = seyahat::bugun();
            let _ = s;
            YOLDA
        } else if let Some(s) = seyahat::yarin() {
            if bot.durum().duyurulan_seyahat == s.bas {
                continue;
            }
            bot.durum().duyurulan_seyahat = s.bas;
            GIDIYORUM
        } else {
            if rand::random::<f64>() > DURTME_SANSI * gelisim::evre(&bot.durum().gelisim).durtme {
                continue;
            }
            if rand::random::<f64>() < SORUN_PAYI {
                // yazılım kanalına küçük bir kod derdi at
                if let Some(kanal) = varsayilan_kanal(&bot, &ctx) {
                    if !bot.durum().sohbetler.contains_key(&kanal) {
                        bot.sorun_at(&ctx, kanal).await;
                    }
                }
                continue;
            }
            DURUP_DURURKEN
        };
        let Some((kanal, son)) = bos_kanal(&bot) else {
            continue;
        };

        let laf = match bot.uret(&[kullanici(son)], talimat, Some(120), "laf").await {
            Ok(l) => l,
            Err(e) => {
                log::error!("ai [durtme]: {e}");
                continue;
            }
        };
        match bot
            .gonder_satirlar(&ctx, kanal, &laf, None, None, None)
            .await
        {
            Some(p) => {
                sohbet_baslat(&mut bot.durum(), kanal, Some(p));
            }
            None => log::debug!("durtme: model sustu, atlandı"),
        }
    }
}

// arada bir resimler/ klasöründen bir görsel atar; bazen de hacklenmiş taklidiyle
async fn saka_dongusu(bot: Arc<Bot>, ctx: Context) {
    loop {
        sleep(SAKA_ARALIGI).await;
        if KAPANIYOR.load(Ordering::SeqCst) {
            return;
        }
        if !uyku::uyanik_mi(&bot.durum()) || seyahat::simdi().is_some() {
            continue;
        }
        if rand::random::<f64>() > SAKA_SANSI {
            continue;
        }
        let Some((kanal, _)) = bos_kanal(&bot) else {
            continue;
        };
        bot.saka_yap(&ctx, kanal, rand::random::<f64>() < HACK_PAYI)
            .await;
    }
}

// arada internette gezer; ilk gezinti açılıştan 10 dk sonra, sonra 4 saatte bir
async fn gezgin_dongusu(bot: Arc<Bot>) {
    let mut ilk = true;
    loop {
        sleep(if ilk {
            Duration::from_secs(600)
        } else {
            GEZGIN_ARALIGI
        })
        .await;
        ilk = false;
        if KAPANIYOR.load(Ordering::SeqCst) {
            return;
        }
        if uyku::uyanik_mi(&bot.durum()) {
            bot.gezgin().await;
        }
    }
}

// 10 dakikada bir: kapanan sohbetlerin ve gözlemlerin kuyruğunu zihne işler.
// uyku kontrolüne takılmaz; gece birikenler de sabaha kalmadan kaydedilir
async fn bellek_dongusu(bot: Arc<Bot>) {
    loop {
        sleep(Duration::from_secs(10 * 60)).await;
        if KAPANIYOR.load(Ordering::SeqCst) {
            return;
        }
        // uykuda mesajlar zihne işlenmeye devam eder: 2 saatte bir gece gözlemi kuyruğa düşer
        {
            let mut d = bot.durum();
            if !uyku::uyanik_mi(&d) && simdi_unix() - d.son_gece_gozlem >= 2 * 3600 {
                let son = son_mesajlar(&d, 300);
                d.son_gece_gozlem = simdi_unix();
                d.bellek_kuyruk.push_back((
                    son,
                    "gece gözlemi (bot uykuda)".to_string(),
                    "gece".to_string(),
                    false,
                ));
            }
        }
        loop {
            let isi = {
                let mut d = bot.durum();
                if d.bellek_kuyruk.len() > 50 {
                    log::warn!(
                        "bellek: kuyruk şişti ({}), en eski atılıyor",
                        d.bellek_kuyruk.len()
                    );
                    d.bellek_kuyruk.pop_front();
                }
                d.bellek_kuyruk.pop_front()
            };
            let Some((dokum_metni, kaynak, kanal_adi, elestir)) = isi else {
                break;
            };
            let dokum_kopya = dokum_metni.clone();
            if let Err(e) = bot.gunlukcu(dokum_metni, &kaynak, &kanal_adi).await {
                log::warn!(
                    "zihin: günlükçü başarısız [{kaynak}]: {}",
                    kirp_hata(&e.to_string())
                );
            }
            if elestir {
                bot.elestirmen(dokum_kopya).await;
            }
        }
    }
}

// dakikada bir uyku planına bakar; uyanınca uyurken gelen etiketlere döner
async fn uyku_dongusu(bot: Arc<Bot>, ctx: Context) {
    loop {
        sleep(Duration::from_secs(60)).await;
        if KAPANIYOR.load(Ordering::SeqCst) {
            return;
        }
        {
            let mut d = bot.durum();
            uyku::guncelle(&mut d);
        }
        bot.uyku_gecisi(&ctx).await;
        bot.zaman_asimi_kapat(&ctx).await;
    }
}

