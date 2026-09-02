async fn k_sifirla(bot: &Bot, ctx: &Context, c: &CommandInteraction) {
    let hepsi = secenek_bool(c, "hepsi").unwrap_or(false);
    {
        let mut d = bot.durum();
        if hepsi {
            d.sohbetler.clear();
            d.haber_bekleyen.clear();
            d.mesgul.clear();
        } else {
            d.sohbetler.remove(&c.channel_id);
            d.haber_bekleyen.remove(&c.channel_id);
            d.mesgul.remove(&c.channel_id);
        }
    }
    yanit_bilgi(
        ctx,
        c,
        "Sıfırlandı",
        if hepsi {
            "tüm kanallar sıfırlandı"
        } else {
            "bu kanal sıfırlandı"
        },
    )
    .await;
}

async fn k_haber(bot: &Bot, ctx: &Context, c: &CommandInteraction) {
    ertele(ctx, c).await;
    bot.durum().sohbetler.remove(&c.channel_id);
    let bulundu = bot.haber_at(ctx, c.channel_id).await;
    sonucu_bildir(
        ctx,
        c,
        "Haber",
        if bulundu {
            "gönderildi"
        } else {
            "haber bulamadım"
        },
    )
    .await;
}

async fn k_sorun(bot: &Bot, ctx: &Context, c: &CommandInteraction) {
    ertele(ctx, c).await;
    bot.durum().sohbetler.remove(&c.channel_id);
    bot.sorun_at(ctx, c.channel_id).await;
    sonucu_bildir(ctx, c, "Sorun", "gönderildi").await;
}

async fn k_gez(bot: &Bot, ctx: &Context, c: &CommandInteraction) {
    ertele(ctx, c).await;
    bot.gezgin().await;
    sonucu_bildir(ctx, c, "Gezinti", "tamamlandı").await;
}

async fn k_saka(bot: &Bot, ctx: &Context, c: &CommandInteraction) {
    saka_ortak(bot, ctx, c, false).await;
}

async fn k_hack(bot: &Bot, ctx: &Context, c: &CommandInteraction) {
    saka_ortak(bot, ctx, c, true).await;
}

async fn saka_ortak(bot: &Bot, ctx: &Context, c: &CommandInteraction, hack: bool) {
    ertele(ctx, c).await;
    bot.durum().sohbetler.remove(&c.channel_id);
    bot.saka_yap(ctx, c.channel_id, hack).await;
    sonucu_bildir(ctx, c, if hack { "Hack" } else { "Şaka" }, "gönderildi").await;
}

async fn k_ajanlar(bot: &Bot, ctx: &Context, c: &CommandInteraction) {
    ertele(ctx, c).await;
    bot.profilci().await;
    bot.hoca().await;
    bot.durum().dizin = hafiza::dizin_yenile();
    sonucu_bildir(ctx, c, "Ajanlar", "profilci ve hoca çalıştı").await;
}

async fn k_uyan(bot: &Bot, ctx: &Context, c: &CommandInteraction) {
    ertele(ctx, c).await;
    bot.uyandir();
    bot.uyku_gecisi(ctx).await;
    sonucu_bildir(ctx, c, "Uyandı", "uyku kesildi").await;
}

async fn k_uyu(bot: &Bot, ctx: &Context, c: &CommandInteraction) {
    ertele(ctx, c).await;
    let saat = secenek_tam(c, "saat").unwrap_or(8);
    bot.uyut(saat);
    bot.uyku_gecisi(ctx).await;
    sonucu_bildir(ctx, c, "Uyutuldu", &format!("{saat} saat")).await;
}

