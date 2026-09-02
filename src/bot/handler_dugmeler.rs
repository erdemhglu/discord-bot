impl Handler {
    // ayar paneli butonları: değişikliği uygula (komutlarla aynı yollar), paneli
    // yerinde yenile. Yetki komutlarla aynı: herkes (model değişimi panelde yok)
    async fn ayar_dugmesi(&self, ctx: &Context, c: ComponentInteraction) {
        let id = c.data.custom_id.clone();
        if let Some(kip) = id.strip_prefix(modal::AYAR_DUSUNME) {
            if let Some(yeni) = DusunmeKip::arg_ile(kip) {
                self.bot.durum().dusunme = yeni;
                hafiza::yaz("dusunme.md", yeni.dosya_degeri());
            }
        } else if id == modal::AYAR_DEBUG {
            self.bot.debug_ayarla("");
        } else if id == modal::AYAR_UYAN {
            self.bot.uyandir();
            self.bot.uyku_gecisi(ctx).await;
        } else if id == modal::AYAR_UYU {
            self.bot.uyut(8);
            self.bot.uyku_gecisi(ctx).await;
        } else {
            return;
        }
        log::info!("ayar [{}]: {id}", c.user.id);
        let yanit = modal::ayarlar_mesaji(&self.bot.durum(), false);
        if let Err(e) = c
            .create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(yanit))
            .await
        {
            log::warn!("ayar paneli yenilenemedi: {e}");
        }
    }

    // gizle kipindeki "Düşünce Sürecini Göster" butonu: düşünenin deposundan
    // bulur, yalnız tıklayana görünen ephemeral kod bloğu olarak açar
    async fn dusunce_dugmesi(&self, ctx: &Context, c: ComponentInteraction) {
        if c.data.custom_id != DUSUNCE_DUGMESI {
            return;
        }
        let dusunce = self.bot.durum().dusunce_deposu.get(&c.message.id).cloned();
        let Some(dusunce) = dusunce else {
            let _ = c
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .ephemeral(true)
                            .content("düşünce bulunamadı (bot yeniden başlamış olabilir)"),
                    ),
                )
                .await;
            return;
        };
        let icerik = dusunce_gosterim(&dusunce);
        if let Err(e) = c
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .ephemeral(true)
                        .content(icerik),
                ),
            )
            .await
        {
            log::warn!("düşünce ephemeral yanıtı gönderilemedi: {e}");
        }
    }
}
