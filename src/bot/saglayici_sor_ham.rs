impl Bot {
    async fn sor_ham(
        &self,
        mut govde: serde_json::Value,
        kategori: &'static str,
    ) -> Result<String, Hata> {
        let mut kapatildi = self.reasoning_kapat(&mut govde, true);
        let mut son_hata: Hata = "istek hiç yapılamadı".into();
        for deneme in 0..=AI_YENIDEN_DENEME {
            if deneme > 0 {
                sleep(Duration::from_secs(u64::from(deneme) * 2)).await;
                log::warn!("ai [sor_ham]: {son_hata} — {}. deneme", deneme + 1);
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
            let govde_metni = cevap.text().await.unwrap_or_default();
            if !durum.is_success() {
                // 404 çoğunlukla "bu isimde model yok" demek; gövdedeki mesajı ve modeli göster
                let model = govde.get("model").and_then(|m| m.as_str()).unwrap_or("?");
                let hata: Hata =
                    format!("{durum} (model: {model}): {}", kirp_hata(&govde_metni)).into();
                if deneme < AI_YENIDEN_DENEME && kapatildi && reasoning_zorunlu_hatasi(&govde_metni)
                {
                    log::warn!(
                        "ai [sor_ham] [{kategori}]: model reasoning kapatılmasına izin vermiyor, düşük eforla açık yeniden deneniyor"
                    );
                    Self::reasoning_alanlarini_kaldir(&mut govde);
                    self.reasoning_dusuk_efor(&mut govde);
                    kapatildi = false;
                    if let Some(yeni) = Self::butce_buyut(&mut govde, REASONING_BUTCE_TABANI) {
                        log::warn!(
                            "ai [sor_ham] [{kategori}]: düşünce bütçeyi yiyebilir, max_tokens {yeni}'e çıkarıldı"
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
            let yanit: Yanit = serde_json::from_str(&govde_metni)?;
            if let Some(k) = yanit.usage {
                self.metrik_ekle(kategori, k);
            }
            let ilk = yanit.choices.into_iter().next();
            let dusunce_kr = ilk.as_ref().map_or(0, |s| dusunce_uzunlugu(&s.message));
            let content_bos = ilk.as_ref().is_none_or(|s| {
                s.message
                    .content
                    .as_deref()
                    .is_none_or(|c| c.trim().is_empty())
            });
            let metin = ilk
                .as_ref()
                .and_then(|s| yanit_icerigi(&s.message, kategori));
            match metin {
                Some(metin) => {
                    if content_bos {
                        log::warn!(
                            "ai [sor_ham] [{kategori}]: content boş, JSON düşünce alanından alındı ({dusunce_kr} kr düşünce)"
                        );
                    }
                    return Ok(metin);
                }
                // kapatılamayan reasoning bütçeyi yiyip content: null bırakmış olabilir;
                // bütçeyi (varsa) büyütüp düşük eforla bir kez daha dene, o da yetmediyse pes et
                None if deneme < AI_YENIDEN_DENEME => {
                    self.reasoning_dusuk_efor(&mut govde);
                    let buyudu = Self::butce_buyut(&mut govde, REASONING_BUTCE_TABANI);
                    log::warn!(
                        "ai [sor_ham] [{kategori}]: modelden boş yanıt geldi ({dusunce_kr} kr düşünce){}",
                        match buyudu {
                            Some(y) => format!(", max_tokens {y}'e çıkarılıp yeniden deneniyor"),
                            None => String::new(),
                        }
                    );
                    son_hata = "modelden boş yanıt geldi".into();
                }
                None => {
                    let model = govde.get("model").and_then(|m| m.as_str()).unwrap_or("?");
                    let butce = govde
                        .get("max_tokens")
                        .and_then(serde_json::Value::as_u64)
                        .map_or("bütçesiz".to_string(), |b| b.to_string());
                    return Err(format!(
                        "modelden boş yanıt geldi [{kategori}] (model: {model}, max_tokens: {butce}, düşünce: {dusunce_kr} kr)"
                    )
                    .into());
                }
            }
        }
        Err(son_hata)
    }

}
