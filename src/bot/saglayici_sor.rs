impl Bot {
    async fn sor(
        &self,
        sistem: &str,
        gecmis: &[Mesaj],
        max_tokens: u32,
        kategori: &'static str,
    ) -> Result<String, Hata> {
        self.sor_bolumlu(sistem, "", gecmis, Some(max_tokens), kategori)
            .await
    }

    // sistem mesajı iki blok: sabit blok destekleyen sağlayıcılarda cache_control ile
    // işaretli (anthropic/gemini); değişken blok her seferinde yeniden okunur.
    // butce None ise max_tokens gitmez, model bütçesiz konuşur.
    async fn sor_bolumlu(
        &self,
        sabit: &str,
        degisken: &str,
        gecmis: &[Mesaj],
        butce: Option<u32>,
        kategori: &'static str,
    ) -> Result<String, Hata> {
        let model = self.durum().model.clone();
        let mut mesajlar = vec![sistem_json(sabit, degisken, &self.api_adres)];
        mesajlar.extend(gecmis.iter().map(mesaj_json));
        let mut govde = serde_json::json!({
            "model": model,
            "messages": mesajlar,
            "temperature": 0.7,
        });
        if let Some(t) = butce {
            govde["max_tokens"] = serde_json::json!(t);
        }
        self.sor_ham(govde, kategori).await
    }

    // stream istek: hata kontrolü sor_ham ile aynı, gövdeye stream eklenir.
    // butce None ise max_tokens gitmez, model bütçesiz konuşur (release sohbet yolu).
    async fn sor_ham_akis(
        &self,
        sabit: &str,
        degisken: &str,
        gecmis: &[Mesaj],
        butce: Option<u32>,
        kategori: &'static str,
    ) -> Result<AkisOkuyucu, Hata> {
        let model = self.durum().model.clone();
        let mut mesajlar = vec![sistem_json(sabit, degisken, &self.api_adres)];
        mesajlar.extend(gecmis.iter().map(mesaj_json));
        let mut govde = serde_json::json!({
            "model": model,
            "messages": mesajlar,
            "temperature": 0.7,
            "stream": true,
            // son chunk'ta usage gelsin (token sayacı)
            "stream_options": { "include_usage": true },
        });
        if let Some(t) = butce {
            govde["max_tokens"] = serde_json::json!(t);
        }
        let mut kapatildi = if self.reasoning_zorunlu_biliniyor(&model) {
            Self::butce_tabanini_uygula(&mut govde, REASONING_ZORUNLU_TABAN);
            false
        } else {
            self.reasoning_kapat(&mut govde, false)
        };
        // yeniden deneme yalnız akış açılmadan önce: parça gelmeye başladıysa okuyucu dönmüş
        // olur, sonrası gonder_akis'in yarım-kaldı yoludur
        let mut son_hata: Hata = "istek hiç yapılamadı".into();
        for deneme in 0..=AI_YENIDEN_DENEME {
            if deneme > 0 {
                sleep(Duration::from_secs(u64::from(deneme) * 2)).await;
                log::warn!("ai [sor_ham_akis]: {son_hata} — {}. deneme", deneme + 1);
            }
            let cevap = match self
                .http
                .post(&self.api_adres)
                .bearer_auth(&self.anahtar)
                .json(&govde)
                .send()
                .await
            {
                Ok(c) => c,
                Err(e) if e.is_connect() || e.is_timeout() || e.is_request() => {
                    son_hata = e.into();
                    continue;
                }
                Err(e) => return Err(e.into()),
            };
            let durum = cevap.status();
            if !durum.is_success() {
                let govde_metni = cevap.text().await.unwrap_or_default();
                let model = govde.get("model").and_then(|m| m.as_str()).unwrap_or("?");
                let hata: Hata =
                    format!("{durum} (model: {model}): {}", kirp_hata(&govde_metni)).into();
                if deneme < AI_YENIDEN_DENEME && kapatildi && reasoning_zorunlu_hatasi(&govde_metni)
                {
                    log::warn!(
                        "ai [sor_ham_akis]: model reasoning kapatılmasına izin vermiyor, açık yeniden deneniyor"
                    );
                    self.reasoning_zorunlu_isaretle(model);
                    Self::reasoning_alanlarini_kaldir(&mut govde);
                    kapatildi = false;
                    if Self::butce_tabanini_uygula(&mut govde, REASONING_ZORUNLU_TABAN) {
                        log::warn!(
                            "ai [sor_ham_akis]: küçük bütçe reasoning'e yetmeyebilir, {REASONING_ZORUNLU_TABAN}'a çıkarıldı"
                        );
                    }
                    son_hata = hata;
                    continue;
                }
                if deneme < AI_YENIDEN_DENEME && durum_denenebilir(durum) {
                    son_hata = hata;
                    continue;
                }
                return Err(hata);
            }
            return Ok(AkisOkuyucu {
                cevap,
                tampon: Vec::new(),
                kuyruk: Vec::new(),
                kullanim: Kullanim::default(),
                kategori,
                done: false,
                bitti: false,
            });
        }
        Err(son_hata)
    }

    // sohbet cevabının sistem mesajı: kim konuşuyor, ne konuşuluyor bakıp
    // hafızadan yalnız ilgili parçaları getirir; uret ve uret_akis ortak kullanır
}
