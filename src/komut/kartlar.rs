async fn k_durum(bot: &Bot, ctx: &Context, c: &CommandInteraction) {
    let yanit = modal::durum_mesaji(&bot.durum());
    yanit_gonder(ctx, c, yanit).await;
}

async fn k_yardim(_bot: &Bot, ctx: &Context, c: &CommandInteraction) {
    yanit_gonder(ctx, c, modal::yardim_mesaji()).await;
}

async fn k_ayarlar(bot: &Bot, ctx: &Context, c: &CommandInteraction) {
    let yanit = modal::ayarlar_mesaji(&bot.durum(), true);
    yanit_gonder(ctx, c, yanit).await;
}

async fn k_zihin(bot: &Bot, ctx: &Context, c: &CommandInteraction) {
    if secenek_bool(c, "test").unwrap_or(false) {
        ertele(ctx, c).await;
        // teşhis: bu kanalın son satırlarını hemen günlükçüye ver; zihin zincirinin
        // çalışıp çalışmadığı 40 dk beklemeden görülsün (reasoning'li modelde boş
        // dönüyordu, canlı log olmadan anlaşılmıyordu)
        let mut satirlar: Vec<String> = {
            let d = bot.durum();
            d.kanal_gecmisi
                .get(&c.channel_id)
                .map(|g| g.iter().rev().take(30).cloned().collect())
                .unwrap_or_default()
        };
        if satirlar.is_empty() {
            sonucu_bildir(ctx, c, "Zihin testi", "bu kanalda hatırladığım satır yok").await;
            return;
        }
        satirlar.reverse();
        let kanal_adi = c
            .channel_id
            .name(ctx)
            .await
            .unwrap_or_else(|_| c.channel_id.to_string());
        let sonuc = bot
            .gunlukcu(satirlar.join("\n"), "zihin testi", &kanal_adi)
            .await;
        let aciklama = match sonuc {
            Ok(o) => format!(
                "günlükçü: {} kişi, {} konu, {} olay yazıldı · model çıktısı {} karakter",
                o.kisi, o.konu, o.olay, o.cikti
            ),
            Err(e) => format!("günlükçü başarısız: {}", hafiza::kirp(&e.to_string(), 300)),
        };
        sonucu_bildir(ctx, c, "Zihin testi", &aciklama).await;
        return;
    }
    let yanit = modal::zihin_mesaji(&bot.durum());
    yanit_gonder(ctx, c, yanit).await;
}

