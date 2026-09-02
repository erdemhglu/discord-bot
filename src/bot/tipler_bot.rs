struct MesgulGuard<'a> {
    durum: &'a Mutex<Durum>,
    kanal: ChannelId,
}

impl Drop for MesgulGuard<'_> {
    fn drop(&mut self) {
        self.durum
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .mesgul
            .remove(&self.kanal);
    }
}

struct Bot {
    durum: Mutex<Durum>,
    http: reqwest::Client,
    api_adres: String, // chat/completions adresi (openrouter ya da mistral)
    anahtar: String,
    haber_kanali: Option<ChannelId>,
    firecrawl: Option<String>, // yoksa sayfalar düz indirilir
    guild_id: Option<GuildId>, // .env GUILD_ID; ayarlıysa yalnız bu sunucuda çalışır
    izinli_kanallar: Option<HashSet<ChannelId>>, // .env KANALLAR; ayarlıysa yalnız bu kanallarda
    debug_kanali: Option<ChannelId>, // .env DEBUG_KANALI; debug satırları buraya, yoksa aynı kanala
    // .env RESIM_ANALIZI; yalnız açılışta okunur, hiçbir komut/buton bunu değiştiremez
    // (kasıtlı: kapatmak isteyen operatör süreci yeniden başlatmadan açtırılamasın)
    resim_analizi: bool,
}

impl Bot {
    fn durum(&self) -> MutexGuard<'_, Durum> {
        self.durum.lock().unwrap_or_else(|e| e.into_inner())
    }

    // model kullanımını oturum metriğine ekler, kategoriye göre de kırılır (!durum döker)
    fn metrik_ekle(&self, kategori: &'static str, k: Kullanim) {
        log::debug!(
            "api [{kategori}]: giris={} onbellek={} cikis={}",
            k.prompt_tokens,
            k.prompt_tokens_details.cached_tokens,
            k.completion_tokens,
        );
        let mut d = self.durum();
        d.metrik.cagri += 1;
        d.metrik.giris_token += k.prompt_tokens;
        d.metrik.onbellek_token += k.prompt_tokens_details.cached_tokens;
        d.metrik.cikis_token += k.completion_tokens;
        d.metrik.son_cagri_sn = simdi_unix();
        d.metrik.kategoriler.entry(kategori).or_default().topla(k);
    }
}
