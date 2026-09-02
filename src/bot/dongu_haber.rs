async fn haber_dongusu(bot: Arc<Bot>, ctx: Context) {
    loop {
        sleep(HABER_ARALIGI).await;
        if KAPANIYOR.load(Ordering::SeqCst) {
            return;
        }
        if !uyku::uyanik_mi(&bot.durum()) {
            // uykuda haber atılmaz ama seçilir: uyanınca "sabah haberi" olarak stoklanır
            let stok_bos = bot.durum().stok_haber.is_none();
            if stok_bos {
                match bot.haberci().await {
                    Ok(h) => {
                        bot.durum().stok_haber = Some(h);
                        log::debug!("haber: uykuda stoklandı");
                    }
                    Err(e) => log::debug!("haber: uyku stoku seçilemedi: {e}"),
                }
            }
            continue;
        }
        if seyahat::simdi().is_some() {
            // yolda haber atmaz ama ajanlar gene çalışsın, öğrenmeye devam
            bot.profilci().await;
            bot.hoca().await;
            continue;
        }

        bot.gelisim_kontrol(&ctx).await;
        bot.profilci().await;
        let son = son_mesajlar(&bot.durum(), 300);
        // gözlem de kuyruktan işlenir (elestirmen gerekmez)
        bot.durum().bellek_kuyruk.push_back((
            son,
            "6 saatlik gözlem, bot konuşmamış olabilir".to_string(),
            "gozlem".to_string(),
            false,
        ));
        bot.hoca().await;

        let Some(kanal) = varsayilan_kanal(&bot, &ctx) else {
            continue;
        };
        if bot.durum().sohbetler.contains_key(&kanal) {
            continue;
        }

        // uykuda stoklanan haber varsa önce o gider ("sabah haberi")
        let stok = bot.durum().stok_haber.take();
        match stok {
            Some(h) => {
                bot.haber_gonder(&ctx, kanal, h).await;
            }
            None => {
                bot.haber_at(&ctx, kanal).await;
            }
        }
    }
}

impl Bot {
    // küçük, uydurma ama inandırıcı bir yazılım derdi atar, "nasıl çözerim" diye sorar
    async fn sorun_at(&self, ctx: &Context, kanal: ChannelId) {
        let son = son_mesajlar(&self.durum(), 30);
        match self
            .uret(&[kullanici(son)], SORUN, Some(160), "sorun")
            .await
        {
            Ok(laf) => match self
                .gonder_satirlar(ctx, kanal, &laf, None, None, None)
                .await
            {
                Some(p) => {
                    sohbet_baslat(&mut self.durum(), kanal, Some(p));
                }
                None => log::debug!("sorun: model sustu, atlandı"),
            },
            Err(e) => log::error!("ai [sorun]: {e}"),
        }
    }

    // seçilmiş bir haberi kanala atar ve yorum bekleme sohbeti açar
    async fn haber_at(&self, ctx: &Context, kanal: ChannelId) -> bool {
        let h = match self.haberci().await {
            Ok(h) => h,
            Err(e) => {
                log::warn!("haberci: {e}");
                return false;
            }
        };
        self.haber_gonder(ctx, kanal, h).await
    }

    // seçilmiş haberi paylaşır: tur haberi de uykuda stoklanan da buradan gider
    async fn haber_gonder(&self, ctx: &Context, kanal: ChannelId, h: ajanlar::Haber) -> bool {
        let link = if h.url.starts_with("https://") || h.url.starts_with("http://") {
            h.url.clone()
        } else {
            format!("https://news.ycombinator.com/item?id={}", h.id)
        };
        let girdi = match self
            .uret(
                &[kullanici(h.title.clone())],
                HABER_TANIT,
                Some(200),
                "haber_tanit",
            )
            .await
        {
            Ok(g) => g,
            Err(e) => {
                log::error!("ai [haber_tanit]: {e}");
                return false;
            }
        };
        // tanıtım satır satır gider, link ayrı mesaj olarak peşinden (insanlar da öyle atar)
        let Some(protokol) = self
            .gonder_satirlar(ctx, kanal, &girdi, None, None, None)
            .await
        else {
            log::debug!("haber: tanıtımda model sustu, haber atlandı");
            return false;
        };
        self.gonder(ctx, kanal, &link, None, None, None).await;

        let mut d = self.durum();
        sohbet_baslat(&mut d, kanal, Some(protokol));
        d.haber_bekleyen
            .insert(kanal, Instant::now() + YORUM_SURESI);
        d.atilan_haberler.insert(h.id);
        true
    }

    // görsel şakası; hack ise hacklenmiş taklidiyle başlar
    async fn saka_yap(&self, ctx: &Context, kanal: ChannelId, hack: bool) {
        let Some(resim) = rastgele_resim() else {
            let mesaj = CreateMessage::new().embed(modal::bilgi_embed("Şaka", "resimler klasörü boş"));
            let _ = kanal.send_message(&ctx.http, mesaj).await;
            return;
        };
        let metin = if hack {
            self.uret(
                &[kullanici("şaka başlıyor")],
                HACK_GIRIS,
                Some(150),
                "hack_giris",
            )
            .await
        } else {
            self.resimci(&resim).await
        };
        let metin = match metin {
            Ok(m) => m,
            Err(e) => {
                log::error!("ai [saka]: {e}");
                return;
            }
        };
        // görsel tek mesajda gider: protokolden yalnız ilk satır alınır, süsler temizlenir.
        // soy() diğer bütün yollarda olduğu gibi burada da uygulanır (ad öneki, tırnak)
        let bot_adi = self.durum().bot_adi.clone();
        let cevap = cevap_parcala(soy(&metin, &bot_adi));
        let Some(ilk) = cevap.satirlar.first().cloned() else {
            log::debug!("saka: model sustu, şaka atlandı");
            return;
        };
        self.gonder(ctx, kanal, &ilk, None, Some(&resim), None)
            .await;

        let mut d = self.durum();
        let s = sohbet_baslat(&mut d, kanal, Some(ilk));
        if hack {
            s.hackli = HACK_MESAJI;
        }
    }
}

// son konuşulan kanal boşsa ve bot oraya girebiliyorsa kanalı verir
