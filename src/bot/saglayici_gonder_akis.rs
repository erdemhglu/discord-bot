impl Bot {
    async fn gonder_akis(
        &self,
        ctx: &Context,
        kanal: ChannelId,
        mut okuyucu: AkisOkuyucu,
        baglam: AkisBaglam<'_>,
    ) -> Result<AkisSonuc, Hata> {
        let mut metin = String::new();
        let mut dusunce = String::new();
        let mut gonderilenler: Vec<Message> = Vec::new();
        let mut son_yazma = Instant::now();
        let mut ilk = true;
        let mut akis_hatasi: Option<Hata> = None;
        // kip cevap boyunca sabit kalır; stream ortasında değişirse bir sonraki cevapta geçer
        let kip = self.durum().dusunme;
        let baslangic = Instant::now();
        let mut parca_sayisi: u32 = 0;
        let mut ilk_parca_ms: Option<u128> = None;

        loop {
            match okuyucu.sonraki().await {
                Ok(Some(p)) => {
                    parca_sayisi += 1;
                    if ilk_parca_ms.is_none() {
                        ilk_parca_ms = Some(baslangic.elapsed().as_millis());
                    }
                    metin.push_str(&p.metin);
                    if matches!(kip, DusunmeKip::Goster | DusunmeKip::Gizle) {
                        dusunce.push_str(&p.dusunce);
                    }
                    if ilk || son_yazma.elapsed() >= AKIS_DUZENLEME {
                        // soy dilim döndürür: her edit'te metnin tamamı klonlanmaz
                        let yerlesim =
                            akis_gorunum(kip, &dusunce, soy(&metin, baglam.bot_adi), false);
                        // "ilk" ancak gerçekten bir şey yazılınca harcanır: ilk deltalar
                        // yarım satır olduğu için yerleşim boş dönebiliyor, mesaj o zaman
                        // 1,2 sn beklemeden ilk anlamlı içerikle açılsın
                        if !yerlesim.is_empty() {
                            ilk = false;
                            yaz_akis(ctx, kanal, &mut gonderilenler, &yerlesim, baglam.yanit).await;
                            son_yazma = Instant::now();
                        }
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    akis_hatasi = Some(e);
                    break;
                }
            }
        }

        // kullanım metriği ve akış özeti
        self.metrik_ekle(okuyucu.kategori, okuyucu.kullanim);
        log::debug!(
            "akis [{kanal}]: parça={parca_sayisi} ilk={ilk_parca_ms:?}ms toplam={}ms done={}",
            baslangic.elapsed().as_millis(),
            okuyucu.done,
        );
        if !okuyucu.done && akis_hatasi.is_none() {
            log::warn!("akis [{kanal}]: [DONE] gelmeden kapandı, yarım kalmış olabilir");
        }

        // yeni mesaj gelse de akış tamamlanır: cevap snapshot'a gider, yeni mesaj
        // sıradaki turda ele alınır (sil-baştan yok)
        let mut cevap = cevap_parcala(soy(&metin, baglam.bot_adi));
        // model susmayı seçti: açılmış geçici mesajlar silinir, kayda hiçbir şey girmez.
        // "tepki: 💀" + "-" birlikte gelirse susma değil: emoji yine de düşmeli
        if cevap.sus && cevap.satirlar.is_empty() && cevap.tepki.is_none() {
            log::debug!("akis [{kanal}]: sus");
            sil_mesajlar(ctx, gonderilenler).await;
            return match akis_hatasi {
                Some(e) => Err(e),
                None => Ok(AkisSonuc::Sus),
            };
        }
        if cevap.bos() {
            sil_mesajlar(ctx, gonderilenler).await;
            return match akis_hatasi {
                Some(e) => Err(e),
                None => Ok(AkisSonuc::Bos),
            };
        }
        // aynı lafı iki kez etmesin: tekrar eden satırlar düşer; hiç söz kalmadıysa ve
        // tepki de yoksa bir kez yeniden üretir, yine tekrarsa susar
        let tekrarlar: Vec<String> = cevap
            .satirlar
            .iter()
            .filter(|s| self.tekrar_mi(kanal, s))
            .cloned()
            .collect();
        if !tekrarlar.is_empty() {
            cevap.satirlar.retain(|s| !tekrarlar.contains(s));
            log::debug!("akis [{kanal}]: {} tekrar satırı düştü", tekrarlar.len());
        }
        if cevap.satirlar.is_empty() && cevap.tepki.is_none() {
            let t2 = format!(
                "{}\n\nAz önce aynen şunu yazdın: \"{}\". Aynısını ya da benzerini yazma; başka bir açıdan gir ya da konuyu değiştir.",
                baglam.talimat,
                tekrarlar.join(" / ")
            );
            let yeni = match self.uret(baglam.gecmis, &t2, baglam.butce, "sohbet").await {
                Ok(y) => cevap_parcala(&y),
                Err(e) => {
                    log::debug!("akis [{kanal}]: tekrar sonrası yeniden üretim başarısız: {e}");
                    Cevap::default()
                }
            };
            // yalnız tepki dönmesi boş sayılmaz: emoji gidecek bir şeydir
            if (yeni.satirlar.is_empty() && yeni.tepki.is_none())
                || yeni.satirlar.iter().any(|s| self.tekrar_mi(kanal, s))
            {
                sil_mesajlar(ctx, gonderilenler).await;
                return Ok(AkisSonuc::Bos);
            }
            cevap = yeni;
        }
        let yerlesim = akis_yerlesim(kip, &dusunce, &cevap.satirlar);
        yaz_akis(ctx, kanal, &mut gonderilenler, &yerlesim, baglam.yanit).await;

        // emoji tepkisi: yazı yerine ya da yazının yanında, cevaplanan mesaja düşer.
        // hata akışı durdurmaz, tepki süs
        if let (Some(emoji), Some(hedef)) = (&cevap.tepki, baglam.tepki_hedefi) {
            if let Err(e) = ctx
                .http
                .create_reaction(kanal, hedef, &ReactionType::Unicode(emoji.clone()))
                .await
            {
                log::warn!("tepki eklenemedi ({kanal}): {e}");
            }
        }

        // gizlede düşünce mesajda görünmez; cevap sonuna buton konur, tıklayana
        // ephemeral kod bloğu olarak açılır (interaction_create bakar)
        if kip == DusunmeKip::Gizle {
            let dusunce_tek = tek_satir(&dusunce);
            if !dusunce_tek.is_empty() {
                if let Some(son) = gonderilenler.last_mut() {
                    self.durum().dusunce_bagla(son.id, dusunce_tek);
                    let dugme = CreateButton::new(DUSUNCE_DUGMESI)
                        .label("Düşünce Sürecini Göster")
                        .style(ButtonStyle::Secondary);
                    if let Err(e) = son
                        .edit(
                            &ctx.http,
                            EditMessage::new()
                                .components(vec![CreateActionRow::Buttons(vec![dugme])]),
                        )
                        .await
                    {
                        log::warn!("düşünce butonu eklenemedi ({kanal}): {e}");
                    }
                }
            }
        }

        // gönderilenler kayda geçer; thinking kayda girmez, hoca ve eleştirmen yalnız cevabı görür.
        // her satır ayrı bir mesaj olduğu için kayda da ayrı ayrı girer
        let mut d = self.durum();
        for satir in &cevap.satirlar {
            d.kendi_mesajlarim.push_back(satir.clone());
        }
        while d.kendi_mesajlarim.len() > 50 {
            d.kendi_mesajlarim.pop_front();
        }
        let mut notlar: Vec<String> = cevap
            .satirlar
            .iter()
            .map(|s| format!("{}: {s}", baglam.bot_adi))
            .collect();
        if let Some(emoji) = &cevap.tepki {
            // tohum tutarlılığı: model geçmişte kendi protokol biçimini görsün
            notlar.push(format!("{}: tepki: {emoji}", baglam.bot_adi));
        }
        kanal_not_coklu(&mut d, kanal, notlar);
        drop(d);
        if let Some(e) = akis_hatasi {
            log::warn!("akis [{kanal}]: yarıda kesildi, elimizdeki gönderildi: {e}");
        }
        Ok(AkisSonuc::Gonderildi(cevap.protokol_metni()))
    }

    // stream OLMAYAN yolların ortak göndericisi: cevabı protokole göre satırlara böler,
    // her satırı ayrı mesaj olarak sırayla yollar (araya küçük yazma gecikmesi girer,
    // hepsi aynı anda düşmesin). Sus ya da boş cevapta hiçbir şey gitmez, None döner.
    // ping yalnız ilk satıra takılır (hoş geldin mesajı yeni üyeyi etiketler).
}
