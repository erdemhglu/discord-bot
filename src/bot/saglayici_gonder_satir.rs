impl Bot {
    async fn gonder_satirlar(
        &self,
        ctx: &Context,
        kanal: ChannelId,
        ham: &str,
        yanit: Option<MessageId>,
        tepki_hedefi: Option<MessageId>,
        ping: Option<UserId>,
    ) -> Option<String> {
        let bot_adi = self.durum().bot_adi.clone();
        let cevap = cevap_parcala(soy(ham, &bot_adi));
        self.gonder_cevap(ctx, kanal, cevap, yanit, tepki_hedefi, ping)
            .await
    }

    // gonder_satirlar'ın gövdesi, çözülmüş Cevap üzerinden: elinde zaten ayıklanmış
    // (tekrar elenmiş) bir cevap olan yollar metne geri dönüp yeniden çözmesin
    async fn gonder_cevap(
        &self,
        ctx: &Context,
        kanal: ChannelId,
        mut cevap: Cevap,
        yanit: Option<MessageId>,
        tepki_hedefi: Option<MessageId>,
        ping: Option<UserId>,
    ) -> Option<String> {
        let bot_adi = self.durum().bot_adi.clone();
        // tepkinin düşeceği mesaj yoksa (açılış yolları) tepki yok sayılır: yoksa kanala
        // hiçbir şey gitmediği halde "gönderildi" denip sohbet açılıyordu
        if tepki_hedefi.is_none() {
            cevap.tepki = None;
        }
        if cevap.satirlar.is_empty() && cevap.tepki.is_none() {
            log::debug!(
                "gonder_satirlar [{kanal}]: gidecek bir şey yok (sus={})",
                cevap.sus
            );
            return None;
        }
        for (i, satir) in cevap.satirlar.iter().enumerate() {
            if i > 0 {
                let bekle = (SATIR_GECIKME_TABAN
                    + SATIR_GECIKME_HARF * satir.chars().count() as u64)
                    .min(SATIR_GECIKME_TAVAN);
                let _ = kanal.broadcast_typing(&ctx.http).await;
                sleep(Duration::from_millis(bekle)).await;
            }
            let (p, y) = if i == 0 { (ping, yanit) } else { (None, None) };
            // etiket protokol çözüldükten SONRA takılır: metne baştan yapıştırılınca
            // "<@id> -" susma işareti, "<@id> tepki: 💀" da tepki satırı sayılmıyordu
            match p {
                Some(u) => {
                    self.gonder(ctx, kanal, &format!("<@{u}> {satir}"), p, None, y)
                        .await
                }
                None => self.gonder(ctx, kanal, satir, p, None, y).await,
            }
        }
        if let (Some(emoji), Some(hedef)) = (&cevap.tepki, tepki_hedefi) {
            if let Err(e) = ctx
                .http
                .create_reaction(kanal, hedef, &ReactionType::Unicode(emoji.clone()))
                .await
            {
                log::warn!("tepki eklenemedi ({kanal}): {e}");
            }
            // satırları gonder() kaydetti, tepki kaydı burada (protokol biçimiyle)
            let mut d = self.durum();
            kanal_not(&mut d, kanal, format!("{bot_adi}: tepki: {emoji}"));
        }
        Some(cevap.protokol_metni())
    }
}
