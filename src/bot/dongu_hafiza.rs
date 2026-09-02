async fn gecmisi_oku(bot: &Bot, ctx: &Context, guild: &Guild) {
    let bot_id = ctx.cache.current_user().id;
    let uye = match guild.member(ctx, bot_id).await {
        Ok(u) => u,
        Err(e) => {
            log::warn!("{}: üyelik alınamadı: {e}", guild.name);
            return;
        }
    };
    let sinir = simdi_unix() - GECMIS_GUN * 24 * 60 * 60;

    let mut kanallar: Vec<&GuildChannel> = guild
        .channels
        .values()
        .filter(|k| k.kind == ChannelType::Text)
        .collect();
    kanallar.sort_by_key(|k| k.position);

    let mut toplam: Vec<(i64, String, u64, String)> = Vec::new();
    for k in kanallar {
        if bot
            .izinli_kanallar
            .as_ref()
            .is_some_and(|s| !s.contains(&k.id))
        {
            continue;
        }
        let izin = guild.user_permissions_in(k, &uye);
        if !izin.contains(Permissions::VIEW_CHANNEL | Permissions::READ_MESSAGE_HISTORY) {
            continue;
        }
        let mut oncesi: Option<MessageId> = None;
        loop {
            let mut sorgu = GetMessages::new().limit(100);
            if let Some(id) = oncesi {
                sorgu = sorgu.before(id);
            }
            let parca = match k.id.messages(&ctx.http, sorgu).await {
                Ok(p) if !p.is_empty() => p,
                _ => break,
            };
            let mut eskiye_gecti = false;
            for m in &parca {
                if m.timestamp.unix_timestamp() < sinir {
                    eskiye_gecti = true;
                    break;
                }
                if !m.author.bot && !m.content.trim().is_empty() {
                    toplam.push((
                        m.timestamp.unix_timestamp(),
                        ad(&m.author),
                        m.author.id.get(),
                        m.content_safe(&ctx.cache),
                    ));
                }
            }
            if eskiye_gecti || parca.len() < 100 {
                break;
            }
            oncesi = parca.last().map(|m| m.id);
        }
    }

    // discord yeniden eskiye verir, biz eskiden yeniye istiyoruz
    toplam.sort_by_key(|t| t.0);
    let atla = toplam.len().saturating_sub(HAFIZA_BOYU);
    let mut d = bot.durum();
    for (_, isim, id, _) in toplam.iter().skip(atla) {
        if *id == FAVORI {
            d.favori_adi = Some(isim.clone());
        }
        // canlı eşleme öncelikli: tarama eski bilgiyle üstüne yazmasın
        d.ad_id.entry(isim.to_lowercase()).or_insert(*id);
    }
    // tarama sürerken canlı mesajlar da hafızaya girmiş olabilir: tarih ÖNE eklenir,
    // arkaya boca edilirse kronoloji bozulur ve canlı mesajlar ezilir
    for (_, isim, _, metin) in toplam.iter().skip(atla).rev() {
        d.hafiza.push_front(format!("{isim}: {metin}"));
    }
    while d.hafiza.len() > HAFIZA_BOYU {
        d.hafiza.pop_front();
    }
    log::debug!("{}: {} mesaj okundu", guild.name, toplam.len());
}

// haber ve hoş geldin için kanal: ayarlanmışsa o, yoksa sunucunun sistem kanalı, o da yoksa en üstteki metin kanalı
fn varsayilan_kanal(bot: &Bot, ctx: &Context) -> Option<ChannelId> {
    if let Some(k) = bot.haber_kanali {
        return Some(k);
    }
    let izinli = |k: ChannelId| bot.izinli_kanallar.as_ref().is_none_or(|s| s.contains(&k));
    for gid in ctx.cache.guilds() {
        if bot.guild_id.is_some_and(|g| g != gid) {
            continue;
        }
        let Some(g) = ctx.cache.guild(gid) else {
            continue;
        };
        if let Some(k) = g.system_channel_id.filter(|k| izinli(*k)) {
            return Some(k);
        }
        if let Some(k) = g
            .channels
            .values()
            .filter(|k| k.kind == ChannelType::Text && izinli(k.id))
            .min_by_key(|k| k.position)
        {
            return Some(k.id);
        }
    }
    None
}

// ---------- arka plan döngüleri ----------

// döngüleri hayatta tutar: panikte ya da beklenmedik dönüşte log + 5 sn sonra yeniden
// başlatır (panik kancası backtrace'i yazar, bekçi sessiz ölümü önler; 5 sn bekleme
// her iki dalda da var ki bir döngü kurulur kurulmaz dönüyorsa CPU yakmasın).
// Kapanış sinyalinde yeniden başlatmaz.
fn dongu_bekle<F, Fut>(ad: &'static str, kur: F)
where
    F: Fn() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    tokio::spawn(async move {
        loop {
            if KAPANIYOR.load(Ordering::SeqCst) {
                return;
            }
            match tokio::spawn(kur()).await {
                Ok(()) => {
                    if KAPANIYOR.load(Ordering::SeqCst) {
                        return;
                    }
                    // döngüler sonsuzdur; dönüş kendiliğindense yine başlatılır
                    log::warn!("döngü [{ad}]: beklenmedik şekilde döndü, yeniden başlıyor");
                }
                Err(e) => {
                    log::error!("döngü [{ad}]: panik, 5 sn sonra yeniden başlıyor: {e}");
                }
            }
            sleep(Duration::from_secs(5)).await;
        }
    });
}

// altı saatte bir: ajanlar çalışır, sonra hacker news'ten haber atılır
