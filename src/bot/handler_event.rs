struct Handler {
    bot: Arc<Bot>,
    baslatildi: AtomicBool,
    duyuruldu: AtomicBool, // sürüm duyurusu süreç başına bir kez gider
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, hazir: Ready) {
        {
            let mut d = self.bot.durum();
            d.kullanici_adi = hazir.user.name.clone();
            d.bot_adi = d
                .gelisim
                .isim
                .clone()
                .unwrap_or_else(|| hazir.user.name.clone());
        }
        log::info!("giriş yapıldı: {}", hazir.user.name);

        // slash komutlar (/durum /yardim /zihin): her sunucuya kayıt, idempotent
        for guild in ctx.cache.guilds() {
            if let Err(e) = modal::komutlari_kayit(&ctx.http, guild).await {
                log::warn!("slash komutları kaydedilemedi [{guild}]: {e}");
            }
        }

        // ready yeniden bağlanınca tekrar gelir, döngüler bir kere başlasın;
        // bekçi panikte yeniden başlatır
        if !self.baslatildi.swap(true, Ordering::SeqCst) {
            let (b, c) = (self.bot.clone(), ctx.clone());
            dongu_bekle("haber", move || haber_dongusu(b.clone(), c.clone()));
            let (b, c) = (self.bot.clone(), ctx.clone());
            dongu_bekle("durtme", move || durtme_dongusu(b.clone(), c.clone()));
            let (b, c) = (self.bot.clone(), ctx.clone());
            dongu_bekle("saka", move || saka_dongusu(b.clone(), c.clone()));
            let b = self.bot.clone();
            dongu_bekle("gezgin", move || gezgin_dongusu(b.clone()));
            let b = self.bot.clone();
            dongu_bekle("bellek", move || bellek_dongusu(b.clone()));
            let (b, c) = (self.bot.clone(), ctx.clone());
            dongu_bekle("uyku", move || uyku_dongusu(b.clone(), c.clone()));
        }
    }

    async fn guild_create(&self, ctx: Context, guild: Guild, _yeni: Option<bool>) {
        // GUILD_ID ayarlıysa yalnız o sunucu taranır; diskteki geçmiş listesine de girmez
        if self.bot.guild_id.is_some_and(|g| g != guild.id) {
            return;
        }
        let ilk_kez = {
            let mut d = self.bot.durum();
            let yeni = d.taranan.insert(guild.id);
            if yeni {
                let liste: Vec<String> = d.taranan.iter().map(|g| g.get().to_string()).collect();
                hafiza::yaz("taranan.md", &liste.join("\n"));
            }
            yeni
        };
        // yeniden başlayınca bir kez: hangi sürümün koştuğu kanalda görünsün (Emin isteği).
        // ready'de sunucu önbelleği henüz dolu değil, kanal burada bulunur. Hafızaya yazılmaz:
        // bot bunu kendi lafı sanıp sürüm muhabbeti yapmasın.
        if !self.duyuruldu.swap(true, Ordering::SeqCst) {
            if let Some(kanal) = varsayilan_kanal(&self.bot, &ctx) {
                let (model, kip) = {
                    let d = self.bot.durum();
                    (d.model.clone(), d.dusunme.ad())
                };
                let aciklama = format!("model {model} · düşünme {kip}");
                let embed = modal::bilgi_embed(&format!("geldim · {}", surum_metni()), &aciklama);
                let mesaj = CreateMessage::new().embed(embed);
                if let Err(e) = kanal.send_message(&ctx.http, mesaj).await {
                    log::warn!("sürüm duyurusu gönderilemedi ({kanal}): {e}");
                }
            }
        }
        if !ilk_kez {
            return;
        }
        let bot = self.bot.clone();
        tokio::spawn(async move {
            gecmisi_oku(&bot, &ctx, &guild).await;
            bot.profilci().await;
            if bot.durum().huy.is_empty() {
                bot.hoca().await;
            }
        });
    }

    async fn guild_member_addition(&self, ctx: Context, uye: Member) {
        if self.bot.guild_id.is_some_and(|g| g != uye.guild_id) {
            return;
        }
        let kanal = {
            let sistem = ctx
                .cache
                .guild(uye.guild_id)
                .and_then(|g| g.system_channel_id);
            match sistem.or_else(|| varsayilan_kanal(&self.bot, &ctx)) {
                Some(k) => k,
                None => return,
            }
        };
        let isim = ad(&uye.user);
        {
            let mut d = self.bot.durum();
            if uye.user.id.get() == FAVORI {
                d.favori_adi = Some(isim.clone());
            }
            if d.sohbetler.contains_key(&kanal) {
                return;
            }
        }

        let selam = match self
            .bot
            .uret(
                &[kullanici(format!("{isim} sunucuya yeni katıldı."))],
                HOS_GELDIN,
                Some(200),
                "hos_geldin",
            )
            .await
        {
            Ok(s) => s,
            Err(e) => {
                log::error!("ai [hos_geldin]: {e}");
                return;
            }
        };
        // etiket ilk satıra gönderim anında takılır (gonder_satirlar ekler): metne baştan
        // yapıştırılırsa "-" ve "tepki:" protokol satırları tanınmaz hâle geliyordu
        match self
            .bot
            .gonder_satirlar(&ctx, kanal, &selam, None, None, Some(uye.user.id))
            .await
        {
            Some(p) => {
                sohbet_baslat(&mut self.bot.durum(), kanal, Some(p));
            }
            None => log::debug!("hos_geldin: model sustu, atlandı"),
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        // botlar, webhook'lar ve özel mesajlar dışarıda; bot-bot döngüsü olmasın
        if msg.author.bot || msg.webhook_id.is_some() || msg.guild_id.is_none() {
            return;
        }
        // GUILD_ID/KANALLAR ayarlıysa yalnız izinli sunucu/kanalda çalışır (.env, isteğe bağlı)
        if self.bot.guild_id.is_some_and(|g| msg.guild_id != Some(g)) {
            return;
        }
        if self
            .bot
            .izinli_kanallar
            .as_ref()
            .is_some_and(|k| !k.contains(&msg.channel_id))
        {
            return;
        }
        let ham_metin = msg.content_safe(&ctx.cache);
        // ekteki ilk görsel modele gider; sırf resim atılmış mesaj da (metni boş) işlenir.
        // RESIM_ANALIZI kapalıysa hiç bakılmaz (Bot::resim_analizi yalnız açılışta okunur,
        // hiçbir komut değiştiremez) — mesaj sanki hiç ek yokmuş gibi işlenir.
        let resim = self.bot.resim_analizi.then(|| {
            msg.attachments
                .iter()
                .find(|e| {
                    e.content_type
                        .as_deref()
                        .is_some_and(|t| t.starts_with("image/"))
                })
                .map(|e| e.url.clone())
        }).flatten();
        if ham_metin.trim().is_empty() && resim.is_none() {
            return;
        }
        // hafıza, kanal notu ve sohbet satırı aynı metni taşır: resim işareti metnin içinde
        let metin = match &resim {
            None => ham_metin,
            Some(_) if ham_metin.trim().is_empty() => "[resim attı]".to_string(),
            Some(_) => format!("[resim] {ham_metin}"),
        };
        let kanal = msg.channel_id;
        let isim = ad(&msg.author);
        let bot_id = ctx.cache.current_user().id;

        // 1. faz (kilit): kayıtlar + bayrak kararları
        let (dogrudan_cevapla, etiketlendi, degerlendir, diyalog, debug) = {
            let mut d = self.bot.durum();
            // etiketlendi mi, adı geçti mi, mesajına yanıt mı verildi
            let etiketlendi = msg.mentions.iter().any(|u| u.id == bot_id)
                || msg
                    .referenced_message
                    .as_ref()
                    .is_some_and(|r| r.author.id == bot_id)
                || [&d.bot_adi, &d.kullanici_adi]
                    .iter()
                    .any(|a| !a.is_empty() && metin.to_lowercase().contains(&a.to_lowercase()));
            d.gelisim.mesaj += 1;
            hatirla(&mut d, &isim, &metin);
            d.ad_id.insert(isim.to_lowercase(), msg.author.id.get());
            d.kullanici_adlari
                .insert(msg.author.id.get(), msg.author.name.clone());
            d.son_kanal = Some(kanal);
            if msg.author.id.get() == FAVORI {
                d.favori_adi = Some(isim.clone());
            }

            // haber attık, yorum bekliyorduk ama süre doldu
            if d.haber_bekleyen
                .get(&kanal)
                .is_some_and(|t| Instant::now() > *t)
            {
                d.sohbetler.remove(&kanal);
                d.haber_bekleyen.remove(&kanal);
            }

            // uyuyorsa yazmaz; etiketlenmişse uyanınca döner
            if !uyku::uyanik_mi(&d) {
                if etiketlendi {
                    d.bekleyen_etiketler
                        .push((kanal, format!("{isim}: {metin}")));
                    if d.bekleyen_etiketler.len() > 20 {
                        d.bekleyen_etiketler.remove(0);
                    }
                }
                return;
            }

            let acik = d.sohbetler.contains_key(&kanal);
            // sohbet açık olması tek başına "herkese cevap ver" demek değil: gerçek insan gibi
            // az önce KENDİSİYLE konuşan kişiye otomatik devam eder (sohbetteki son user
            // mesajının sahibi bu mesajı atanla aynıysa), ama kanaldaki başka biri yazdıysa ya
            // da sohbet soğumuşsa yine isteklilik değerlendirir.
            let devam_eden_diyalog = acik
                && d.sohbetler.get(&kanal).is_some_and(|s| {
                    s.gecmis
                        .iter()
                        .rev()
                        .find(|m| m.role == "user")
                        .and_then(|m| m.content.split_once(": ").map(|(ad, _)| ad))
                        // eq_ignore_ascii_case Türkçe İ/i̇'de ıskalar; kucult tam bunun için var
                        .is_some_and(|ad| kucult(ad) == kucult(&isim))
                });
            let degerlendir = if !etiketlendi && !devam_eden_diyalog {
                // rate limit: kanal başına en sık 2 dakikada bir isteklilik çağrısı
                let simdi = Instant::now();
                let uygun = d
                    .son_degerlendirme
                    .get(&kanal)
                    .is_none_or(|t| simdi.duration_since(*t) >= DEGERLENDIRME_ARALIGI);
                if uygun {
                    d.son_degerlendirme.insert(kanal, simdi);
                }
                uygun
            } else {
                false
            };
            (
                etiketlendi || devam_eden_diyalog,
                etiketlendi,
                degerlendir,
                devam_eden_diyalog,
                d.debug,
            )
        };

        // 2. faz (kilitsiz): isteklilik değerlendirmesi — her mesaja atlamaz,
        // konu/personalık/ilgi tartılır; etiket ve sürmekte olan diyalog zaten doğrudan cevaplanır
        let mut katil = dogrudan_cevapla;
        // debug izi: kararın gerekçesi (yalnız debug açıkken kanala düşer)
        let mut iz = if etiketlendi {
            "etiket".to_string()
        } else if diyalog {
            "diyalog sürüyor (aynı kişi)".to_string()
        } else if degerlendir {
            String::new()
        } else {
            "isteklilik: 2 dk sınırı, değerlendirilmedi".to_string()
        };
        if degerlendir {
            let esik = {
                let d = self.bot.durum();
                // evre cesareti eşik üzerinden: yeni sıkıngan, eski toprak rahat
                let mut esik = ISTEK_ESIGI as i32;
                let cesaret = gelisim::evre(&d.gelisim).sans;
                if cesaret < 0.9 {
                    esik += 1;
                } else if cesaret > 1.1 {
                    esik -= 1;
                }
                if seyahat::simdi().is_some() {
                    esik += 2; // yoldayken daha az katılır
                }
                esik
            };
            match self.bot.isteklilik().await {
                Some((puan, sebep)) => {
                    log::debug!("isteklilik [{kanal}]: puan={puan} eşik={esik} sebep={sebep}");
                    katil = i32::from(puan) >= esik;
                    if debug {
                        iz = format!("isteklilik {puan}/{esik} · sebep: {sebep}");
                    }
                }
                None => {
                    // çağrı başarısız: eski yedek zar
                    katil = rand::random::<f64>() < SANS;
                    log::debug!("isteklilik [{kanal}]: çağrı yok, yedek zar={katil}");
                    if debug {
                        iz = format!("isteklilik: çağrı yok → yedek zar {katil}");
                    }
                }
            }
        }

        // 3. faz (kilit): sohbete gir ve mesajı işle
        let cevap_ver = {
            let mut d = self.bot.durum();
            let mut acik = d.sohbetler.contains_key(&kanal);
            if !acik && katil {
                sohbet_baslat(&mut d, kanal, None);
                acik = true;
            }
            if let Some(s) = d.sohbetler.get_mut(&kanal) {
                // yalnız en son resim modele gider: eski girdilerin linki düşer
                for m in s.gecmis.iter_mut() {
                    m.resim = None;
                }
                s.gecmis.push(match &resim {
                    Some(url) => kullanici_resimli(format!("{isim}: {metin}"), url),
                    None => kullanici(format!("{isim}: {metin}")),
                });
                s.son_mesaj = Some(msg.id);
                s.son_etiketlendi = etiketlendi;
                s.gelen += 1;
                s.son_gelenler.push_back((isim.clone(), msg.id));
                if s.son_gelenler.len() > 20 {
                    s.son_gelenler.pop_front();
                }
                if s.gecmis.len() > SOHBET_BOYU {
                    s.gecmis.drain(..s.gecmis.len() - SOHBET_BOYU);
                }
                d.son_aktivite.insert(kanal, Instant::now());
            }
            kanal_not(&mut d, kanal, format!("{isim}: {metin}"));
            // isteklilik sonucu açık sohbette de geçerli: başkası yazdı ve puan
            // eşiğin altındaysa mesaj geçmişe girer ama cevap gelmez (yoksa
            // değerlendirme yalnız token yakıp çöpe gidiyordu)
            acik && katil
        };

        if debug {
            let karar = if cevap_ver { "cevap" } else { "sus" };
            self.bot
                .debug_not(&ctx, kanal, format!("{iz} → {karar}"))
                .await;
        }
        if cevap_ver {
            self.bot.cevapla(&ctx, kanal).await;
        }
    }

    // slash komutlar komut::tanimlar() tablosundan yürütülür (bot yalnız slash'la
    // yönetilir); zihin kartındaki menü/butonlar detay modallarına götürür; modal
    // gönderimleri kısa onay alır (gösterimlik, girdi toplanmaz); düşünce butonu eski akışında
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(c) => match komut::tanimlar().iter().find(|k| k.ad == c.data.name)
            {
                Some(k) => (k.calistir)(&self.bot, &ctx, &c).await,
                None => log::warn!("bilinmeyen slash komut: {}", c.data.name),
            },
            Interaction::Modal(m) => {
                let _ = m
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .ephemeral(true)
                                .content("Görüntüleme amaçlı; bir şey kaydetmedim."),
                        ),
                    )
                    .await;
            }
            Interaction::Component(c) => {
                if c.data.custom_id == DUSUNCE_DUGMESI {
                    self.dusunce_dugmesi(&ctx, c).await;
                    return;
                }
                if c.data.custom_id.starts_with("ayar_") {
                    self.ayar_dugmesi(&ctx, c).await;
                    return;
                }
                // zihin detay katmanı: butonlar bölüm modalı, menü kişi modalı açar
                let m = match c.data.custom_id.as_str() {
                    modal::ZIHIN_KONULAR => Some(modal::modal_konular()),
                    modal::ZIHIN_OLAYLAR => Some(modal::modal_olaylar()),
                    modal::ZIHIN_OZET => {
                        let d = self.bot.durum();
                        Some(modal::modal_ozet(&d))
                    }
                    modal::ZIHIN_KISI_SEC => {
                        let ComponentInteractionDataKind::StringSelect { values } = &c.data.kind
                        else {
                            return;
                        };
                        let Some(id) = values.first().and_then(|v| v.parse::<u64>().ok()) else {
                            return;
                        };
                        Some(modal::modal_kisi(id))
                    }
                    _ => None,
                };
                if let Some(m) = m {
                    if let Err(e) = c
                        .create_response(&ctx.http, CreateInteractionResponse::Modal(m))
                        .await
                    {
                        log::warn!("detay modalı gönderilemedi: {e}");
                    }
                }
            }
            _ => {}
        }
    }
}
