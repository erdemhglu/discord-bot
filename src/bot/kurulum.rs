// ---------- başlangıç ----------

fn ayar(isim: &str) -> Result<String, Hata> {
    match std::env::var(isim) {
        Ok(v) if !v.trim().is_empty() => Ok(v.trim().to_string()),
        _ => Err(format!("{isim} yok, .env dosyasına bak").into()),
    }
}

impl Bot {
    // Sağlayıcıyı ortam değişkenlerinden seçer (.env'i main yükler), durum klasörlerini
    // açar, diskteki durumu yükler ve botu kurar. Discord'a bağlanmaz, token istemez:
    // hem main hem CLI sohbet modu buradan geçer.
    fn kur() -> Result<Arc<Bot>, Hata> {
        // sağlayıcı seçimi: SAGLAYICI=mistral zorlar; yoksa hangi anahtar varsa o, ikisi de varsa openrouter
        let saglayici = std::env::var("SAGLAYICI")
            .unwrap_or_default()
            .to_lowercase();
        let (api_adres, anahtar, varsayilan_model) = if saglayici == "mistral"
            || (ayar("OPENROUTER_KEY").is_err() && ayar("MISTRAL_KEY").is_ok())
        {
            (MISTRAL_ADRES, ayar("MISTRAL_KEY")?, MISTRAL_MODEL)
        } else {
            (OPENROUTER_ADRES, ayar("OPENROUTER_KEY")?, OPENROUTER_MODEL)
        };
        let model = ayar("MODEL").unwrap_or_else(|_| varsayilan_model.to_string());
        // API_ADRES varsa sağlayıcının varsayılan adresini ezer: openai uyumlu
        // kendi router'ına (ör. yerel ağ) yönlendirmek için
        let api_adres = match std::env::var("API_ADRES") {
            Ok(v) if !v.trim().is_empty() => v.trim().to_string(),
            _ => api_adres.to_string(),
        };
        log::info!("sağlayıcı: {api_adres} · model: {model}");
        // yalnız açılışta okunur; kapat/aç için komut yok, süreç yeniden başlamadan değişmez
        let resim_analizi = !matches!(
            std::env::var("RESIM_ANALIZI").unwrap_or_default().trim(),
            "kapali" | "kapalı" | "off" | "hayir" | "hayır" | "0"
        );
        log::info!(
            "resim analizi: {}",
            if resim_analizi { "açık" } else { "kapalı" }
        );
        let haber_kanali = match std::env::var("HABER_KANALI") {
            Ok(v) if !v.trim().is_empty() => Some(ChannelId::new(v.trim().parse()?)),
            _ => None,
        };
        // ikisi de isteğe bağlı: ayarlanmazsa bot eskisi gibi eriştiği her sunucuda/kanalda çalışır
        let guild_id = match std::env::var("GUILD_ID") {
            Ok(v) if !v.trim().is_empty() => Some(GuildId::new(v.trim().parse()?)),
            _ => None,
        };
        let izinli_kanallar = match std::env::var("KANALLAR") {
            Ok(v) if !v.trim().is_empty() => {
                let mut s = HashSet::new();
                for parca in v.split(',') {
                    let parca = parca.trim();
                    if parca.is_empty() {
                        continue;
                    }
                    s.insert(ChannelId::new(parca.parse()?));
                }
                Some(s)
            }
            _ => None,
        };
        for k in ["kisiler", "konular", "olaylar", "arsiv", "kanallar"] {
            std::fs::create_dir_all(PathBuf::from(DURUM_KLASORU).join(k))?;
        }
        std::fs::create_dir_all(RESIM_KLASORU)?;

        let mut durum = Durum::yukle();
        uyku::guncelle(&mut durum);
        let secili = hafiza::oku("model.md");
        durum.model = if secili.trim().is_empty() {
            model
        } else {
            secili.trim().to_string()
        };
        log::info!("model: {}", durum.model);
        Ok(Arc::new(Bot {
            durum: Mutex::new(durum),
            // bağlantı kurma hızlı elensin (BAGLANTI_ZAMAN_ASIMI); toplam süre sınırı yok, yalnız
            // iki veri arası en çok OKUMA_ZAMAN_ASIMI (P0: eskiden tek 60sn'lik timeout uzun
            // düşünme akışını ortasında kesebiliyordu)
            http: reqwest::Client::builder()
                .connect_timeout(BAGLANTI_ZAMAN_ASIMI)
                .read_timeout(OKUMA_ZAMAN_ASIMI)
                .build()?,
            api_adres,
            anahtar,
            haber_kanali,
            firecrawl: std::env::var("FIRECRAWL_KEY")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            guild_id,
            izinli_kanallar,
            debug_kanali: std::env::var("DEBUG_KANALI")
                .ok()
                .and_then(|v| v.trim().parse::<u64>().ok())
                .map(ChannelId::new),
            resim_analizi,
        }))
    }

    // debug modu: karar izi tek satır. Kapalıysa hiçbir şey (çağıran format!'ı da
    // debug açıkken kurar). Hafızaya/kanal notuna yazılmaz — bot bunu kendi lafı sanmasın;
    // bot mesajı olduğu için message handler'a da girmez
    async fn debug_not(&self, ctx: &Context, kanal: ChannelId, metin: String) {
        if !self.durum().debug {
            return;
        }
        log::info!("debug [{kanal}]: {metin}");
        let hedef = self.debug_kanali.unwrap_or(kanal);
        let govde: String = format!("⚙ {metin}").chars().take(300).collect();
        if let Err(e) = hedef.say(&ctx.http, govde).await {
            log::warn!("debug satırı gönderilemedi ({hedef}): {e}");
        }
    }

    // birden çok izi tek satırda yollar; iz yoksa ya da debug kapalıysa sessiz
    async fn debug_izle(&self, ctx: &Context, kanal: ChannelId, acik: bool, iz: &[String]) {
        if acik && !iz.is_empty() {
            self.debug_not(ctx, kanal, iz.join(" · ")).await;
        }
    }
}

async fn kapanis_bekle() {
    let ctrl_c = tokio::signal::ctrl_c();
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("sinyal");
        tokio::select! {
            _ = ctrl_c => {},
            _ = term.recv() => {},
        }
    }
    #[cfg(not(unix))]
    {
        let _ = ctrl_c.await;
    }
}
