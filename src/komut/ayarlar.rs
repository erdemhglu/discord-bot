async fn k_dusunme(bot: &Bot, ctx: &Context, c: &CommandInteraction) {
    match secenek_metin(c, "kip").and_then(DusunmeKip::arg_ile) {
        Some(yeni) => {
            bot.durum().dusunme = yeni;
            hafiza::yaz("dusunme.md", yeni.dosya_degeri());
            yanit_bilgi(ctx, c, "Düşünme", &format!("düşünme artık {}", yeni.ad())).await;
        }
        None => {
            let kip = bot.durum().dusunme;
            yanit_bilgi(
                ctx,
                c,
                "Düşünme",
                &format!(
                    "düşünme şu an {} · seçenekler: göster/gizle/sessiz/kapat",
                    kip.ad()
                ),
            )
            .await;
        }
    }
}

async fn k_model(bot: &Bot, ctx: &Context, c: &CommandInteraction) {
    match secenek_metin(c, "id") {
        None => {
            let m = bot.durum().model.clone();
            yanit_bilgi(ctx, c, "Model", &format!("şu an {m}")).await;
        }
        Some(id) if c.user.id.get() != FAVORI => {
            let _ = id;
            yanit_bilgi(ctx, c, "Model", "onu sen değiştiremezsin").await;
        }
        Some(id) => {
            let id = id.to_string();
            ertele(ctx, c).await;
            if bot.api_adres.contains("openrouter") && !bot.model_var_mi(&id).await {
                sonucu_bildir(ctx, c, "Model", "yok öyle model").await;
            } else {
                bot.durum().model = id.clone();
                hafiza::yaz("model.md", &id);
                sonucu_bildir(ctx, c, "Model", &format!("tamam, {id}")).await;
            }
        }
    }
}

async fn k_debug(bot: &Bot, ctx: &Context, c: &CommandInteraction) {
    let arg = secenek_metin(c, "durum").unwrap_or("");
    let acik = bot.debug_ayarla(arg);
    yanit_bilgi(
        ctx,
        c,
        "Debug",
        if acik {
            "debug açık: kararlar bu kanala düşecek (DEBUG_KANALI ayarlıysa oraya)"
        } else {
            "debug kapalı"
        },
    )
    .await;
}

