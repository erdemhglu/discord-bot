impl Bot {
    fn sohbet_sistemi(&self, gecmis: &[Mesaj], talimat: &str) -> (String, String, String) {
        let mut katilimcilar: Vec<String> = Vec::new();
        let mut metinler: Vec<String> = Vec::new();
        for m in gecmis.iter().filter(|m| m.role == "user") {
            match m.content.split_once(": ") {
                Some((isim, metin)) => {
                    // contains için geçici String üretilmez; dilimle karşılaştırılır
                    if !katilimcilar.iter().any(|k| k.as_str() == isim) {
                        katilimcilar.push(isim.to_string());
                    }
                    metinler.push(metin.to_string());
                }
                None => metinler.push(m.content.clone()),
            }
        }
        let anahtar = hafiza::anahtarlar(&metinler);
        let d = self.durum();
        let getirilen = hafiza::getir(&katilimcilar, &d.ad_id, &anahtar, &d.hafiza, SOHBET_BOYU);
        let (sabit, degisken) = sistem_metni(&d, talimat, &getirilen);
        (sabit, degisken, d.bot_adi.clone())
    }

    // kişilikle konuşur: sohbet, hoş geldin, laf atma, haber tanıtma, şakalar.
    // butce None ise max_tokens gitmez; sohbet cevapları bunu cevap_butcesi! ile belirler.
    // kategori yalnız token metriğinde kırılım için (!durum), isteğe hiçbir etkisi yok.
    async fn uret(
        &self,
        gecmis: &[Mesaj],
        talimat: &str,
        butce: Option<u32>,
        kategori: &'static str,
    ) -> Result<String, Hata> {
        let (sabit, degisken, bot_adi) = self.sohbet_sistemi(gecmis, talimat);
        let cevap = self
            .sor_bolumlu(&sabit, &degisken, gecmis, butce, kategori)
            .await?;
        Ok(temizle(cevap, &bot_adi))
    }

    // sohbet cevabını akış olarak açar; parçalar geldikçe okuyucudan okunur
    async fn uret_akis(
        &self,
        gecmis: &[Mesaj],
        talimat: &str,
        butce: Option<u32>,
        kategori: &'static str,
    ) -> Result<(AkisOkuyucu, String), Hata> {
        let (sabit, degisken, bot_adi) = self.sohbet_sistemi(gecmis, talimat);
        let okuyucu = self
            .sor_ham_akis(&sabit, &degisken, gecmis, butce, kategori)
            .await?;
        Ok((okuyucu, bot_adi))
    }

    // kişiliksiz, düz analiz: ajanlar bunu kullanır
    async fn analiz(
        &self,
        metin: &str,
        talimat: &str,
        max_tokens: u32,
        kategori: &'static str,
    ) -> Result<String, Hata> {
        let girdi = kullanici(format!("{metin}\n\n---\n\n{talimat}"));
        self.sor(ANALIST, &[girdi], max_tokens, kategori).await
    }

    // "bu konuşmaya katılmak istiyor muyum?" mini değerlendirmesi (0-10 puan).
    // etiket/yanıt her zaman cevaplanır, buraya hiç gelmez; hata durumunda None (yedek zar devrede).
    // profil+dizin sabit blokta (cache_control): ana sohbetteki aynı içerikle örtüşür, 6 saatte
    // bir değişir; her mesajda tekrar tekrar tam fiyatına yollanmasın diye ayrı tutulur.
    async fn isteklilik(&self) -> Option<(u8, String)> {
        let (baglam, profil, dizin, bot_adi) = {
            let d = self.durum();
            (
                son_mesajlar(&d, 12),
                d.profil.clone(),
                d.dizin.clone(),
                d.bot_adi.clone(),
            )
        };
        if baglam.trim().is_empty() {
            return None;
        }
        let sabit = format!("{ANALIST}\n\nGRUP PROFİLİ\n{profil}\n\nKİŞİ DİZİNİ\n{dizin}");
        let degisken = ISTEKLILIK.replace("{ad}", &bot_adi);
        let girdi = kullanici(format!("SON MESAJLAR\n{baglam}"));
        match self
            .sor_bolumlu(&sabit, &degisken, &[girdi], Some(80), "isteklilik")
            .await
        {
            Ok(c) => isteklilik_coz(&c),
            Err(e) => {
                log::debug!("isteklilik: çağrı başarısız: {e}");
                None
            }
        }
    }

    // üst üste farklı kişiler yazınca cevap kime dönsün? bekleyen isimler arasından seçer.
    // talimat sabit blokta (cache_control): profil/dizin içermez ama en azından kendi başına sabit kalır.
    async fn hedef_sec(&self, bekleyenler: &[String]) -> Option<String> {
        let (baglam, bot_adi) = {
            let d = self.durum();
            (son_mesajlar(&d, 12), d.bot_adi.clone())
        };
        let sabit = format!("{ANALIST}\n\n{}", HEDEF_SEC.replace("{ad}", &bot_adi));
        let degisken = format!("BEKLEYENLER\n- {}", bekleyenler.join("\n- "));
        let girdi = kullanici(format!("SON MESAJLAR\n{baglam}"));
        match self
            .sor_bolumlu(&sabit, &degisken, &[girdi], Some(40), "hedef_sec")
            .await
        {
            Ok(c) => hedef_ayikla(&c, bekleyenler),
            Err(e) => {
                log::debug!("hedef_sec: çağrı başarısız: {e}");
                None
            }
        }
    }

    // bu sohbetin ruh halini belirler: ucuz mini çağrı, `cevapla` yalnız sohbet açılırken ve
    // birkaç turda bir çağırır (her mesajda değil). Nötr/düşük yoğunluk ya da hata → None,
    // çağıran taraf bunu "belirgin bir ruh hali yok" olarak ele alır.
    async fn ruh_hali_belirle(&self, gecmis: &[Mesaj]) -> Option<String> {
        if gecmis.is_empty() {
            return None;
        }
        // görsel bu mini çağrıya girmez: 40 token'lık ruh hali analizine tam resim yükü
        // yollamak token yakar, vision desteklemeyen bir route'ta da çağrıyı hataya düşürür
        let gecmis: Vec<Mesaj> = gecmis
            .iter()
            .map(|m| Mesaj {
                resim: None,
                ..m.clone()
            })
            .collect();
        let degisken = RUH_HALI.replace("{ad}", &self.durum().bot_adi);
        match self
            .sor_bolumlu(ANALIST, &degisken, &gecmis, Some(40), "ruh_hali")
            .await
        {
            Ok(c) => ruh_hali_ayikla(&c),
            Err(e) => {
                log::debug!("ruh_hali: çağrı başarısız: {e}");
                None
            }
        }
    }

    // mention'lar kapalı gider: model @everyone yazsa bile kimse pinglenmez.
    // gönderilen her şey kendi_mesajlarim'a düşer, hoca ve eleştirmen oradan okur.
    async fn gonder(
        &self,
        ctx: &Context,
        kanal: ChannelId,
        metin: &str,
        ping: Option<UserId>,
        dosya: Option<&PathBuf>,
        yanit: Option<MessageId>, // verilirse discord yanıtı olur, kişi etiketlenir
    ) {
        let mut izin = CreateAllowedMentions::new();
        if let Some(u) = ping {
            izin = izin.users([u]);
        }
        if yanit.is_some() {
            izin = izin.replied_user(true);
        }
        let mut mesaj = CreateMessage::new().content(metin).allowed_mentions(izin);
        if let Some(id) = yanit {
            mesaj = mesaj.reference_message((kanal, id));
        }
        if let Some(yol) = dosya {
            match CreateAttachment::path(yol).await {
                Ok(ek) => mesaj = mesaj.add_file(ek),
                Err(e) => log::warn!("görsel okunamadı ({}): {e}", yol.display()),
            }
        }
        if let Err(e) = kanal.send_message(&ctx.http, mesaj).await {
            log::error!("gönderilemedi ({kanal}): {e}");
            return;
        }
        let mut d = self.durum();
        d.kendi_mesajlarim.push_back(metin.to_string());
        if d.kendi_mesajlarim.len() > 50 {
            d.kendi_mesajlarim.pop_front();
        }
        let satir = format!("{}: {}", d.bot_adi, metin);
        kanal_not(&mut d, kanal, satir);
    }

    // sohbet cevabını akışla gönderir: mesaj erken belirir, aralıklarla düzenlenir.
    // thinking kırpılmadan spoiler bloklarında durur; 1900'ü aşan cevap yeni mesaja bölünür.
    // yanıt bağı yalnız ilk mesajda, mention yalnız onunla gider.
}
