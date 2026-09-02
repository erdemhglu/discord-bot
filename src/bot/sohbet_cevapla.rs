impl Bot {
    // açık sohbette sıradaki cevabı üretir; aynı kanalda aynı anda tek cevap üretilir,
    // o sırada gelen mesajlar geçmişe eklenir ve bir sonraki cevapta görülür
    async fn cevapla(&self, ctx: &Context, kanal: ChannelId) {
        loop {
            let talimat = {
                let mut d = self.durum();
                if d.mesgul.contains(&kanal) {
                    return;
                }
                let Some(s) = d.sohbetler.get(&kanal) else {
                    return;
                };
                let talimat = if s.hackli > 1 {
                    HACK_DEVAM
                } else if s.hackli == 1 {
                    HACK_CIKIS
                } else {
                    ""
                };
                d.mesgul.insert(kanal);
                (talimat, d.debug)
            };
            let (talimat, debug) = talimat;
            // debug izi: bu turun kararları, sonunda tek satır olarak kanala düşer
            let mut iz: Vec<String> = Vec::new();
            // RAII: fonksiyondan her çıkışta (panik dahil) bayrağı bırakır
            let _mesgul = MesgulGuard {
                durum: &self.durum,
                kanal,
            };

            // Kısa bir okuma payı bırak; peş peşe yazılanları tek bağlamda gör.
            sleep(Duration::from_millis(150 + (rand::random::<u64>() % 200))).await;
            let (
                gecmis,
                son_mesaj,
                son_etiketlendi,
                gelen,
                son_metin,
                bekleyenler,
                sayac,
                ruh_hali_eski,
            ) = {
                let d = self.durum();
                let Some(s) = d.sohbetler.get(&kanal) else {
                    return;
                };
                let son_metin = s
                    .gecmis
                    .iter()
                    .rev()
                    .find(|m| m.role == "user")
                    .map(|m| {
                        m.content
                            .split_once(": ")
                            .map(|(_, t)| t)
                            .unwrap_or(&m.content)
                    })
                    .unwrap_or("")
                    .to_string();
                (
                    s.gecmis.clone(),
                    s.son_mesaj,
                    s.son_etiketlendi,
                    s.gelen,
                    son_metin,
                    s.son_gelenler.clone(),
                    s.sayac,
                    s.ruh_hali.clone(),
                )
            };
            log::debug!(
                "cevapla [{kanal}]: tur başı, geçmiş {} satır, gelen={gelen}",
                gecmis.len()
            );
            // reply-to yalnız etiket/ad geçtiyse ya da araya birden fazla mesaj girdiyse;
            // yoksa düz mesaj (gerçek insan gibi her cevabı "yanıtla" ile bağlamaz)
            let mut yanit = if son_etiketlendi || bekleyenler.len() > 1 {
                son_mesaj
            } else {
                None
            };
            // ruh hali her mesajda değil, birkaç turda bir tazelenir
            // (ucuz ama yine de bir çağrı; her cevapta yakmasın)
            let ruh_hali = if sayac % 4 == 0 {
                let yeni = self.ruh_hali_belirle(&gecmis).await.unwrap_or_default();
                if let Some(s) = self.durum().sohbetler.get_mut(&kanal) {
                    s.ruh_hali = yeni.clone();
                }
                if debug && !yeni.is_empty() {
                    iz.push(format!("ruh hali: {yeni}"));
                }
                yeni
            } else {
                ruh_hali_eski
            };
            // istendiyse internete bak (haber, araştır, link) ve bulduklarını göreve ekle
            let mut talimat = talimat.to_string();
            if !ruh_hali.is_empty() {
                talimat = format!(
                    "{talimat}\n\nŞU ANKİ RUH HALİN: {ruh_hali} — bunu ilan etme, üslubuna ve kelime seçimine yedir."
                );
            }
            if let Some(bulgu) = self.arastir(&son_metin).await {
                talimat = format!(
                    "{talimat}\n\nİNTERNETTEN ŞİMDİ ÇEKTİKLERİN (istendiği için baktın; kendi ağzınla anlat, liste yapma, \"kaynak\" deme):\n{bulgu}"
                );
            }
            // üst üste farklı kişiler yazdıysa hedefi model seçer; cevap o mesaja bağlanır
            let konusanlar: std::collections::HashSet<&str> =
                bekleyenler.iter().map(|(i, _)| i.as_str()).collect();
            if konusanlar.len() >= 2 {
                let satirlar: Vec<String> = bekleyenler.iter().map(|(i, _)| i.clone()).collect();
                if let Some(hedef) = self.hedef_sec(&satirlar).await {
                    if let Some((_, id)) = bekleyenler
                        .iter()
                        .rev()
                        .find(|(i, _)| i.eq_ignore_ascii_case(&hedef))
                    {
                        yanit = Some(*id);
                        talimat = format!(
                            "{talimat}\n\nBirden çok kişi yazdı; sen {hedef} adlı kişiye dönmeyi seçtin. Cevabın doğrudan ona seslensin."
                        );
                        log::debug!("cevapla [{kanal}]: hedef seçildi: {hedef}");
                        if debug {
                            iz.push(format!("hedef: {hedef}"));
                        }
                    }
                }
            }
            // üst üste soru sormasın: tavanı kod ölçer, uygulamayı model yapar
            if soru_fazla_mi(&self.durum(), kanal) {
                talimat = format!("{talimat}\n\nBu sefer soru sorma; düz laf et ya da sus.");
                log::debug!("cevapla [{kanal}]: soru tavanı doldu");
                if debug {
                    iz.push("soru tavanı: bu tur soru yok".to_string());
                }
            }
            // Model çağrısı sürerken yazıyor göstergesi görünsün; stream mesajı ilk delta ile açılır.
            let _ = kanal.broadcast_typing(&ctx.http).await;
            let butce = cevap_butcesi!();
            let (okuyucu, bot_adi) = match self.uret_akis(&gecmis, &talimat, butce, "sohbet").await
            {
                Ok(x) => x,
                Err(e) => {
                    log::error!("ai [uret_akis] [{kanal}]: {e}");
                    return;
                }
            };
            let cevap = match self
                .gonder_akis(
                    ctx,
                    kanal,
                    okuyucu,
                    AkisBaglam {
                        bot_adi: &bot_adi,
                        yanit,
                        tepki_hedefi: son_mesaj,
                        gecmis: &gecmis,
                        talimat: &talimat,
                        butce,
                    },
                )
                .await
            {
                Ok(AkisSonuc::Gonderildi(c)) => c,
                Ok(AkisSonuc::Sus) => {
                    // model susmayı seçti: geçmişe, sayaca, aktiviteye hiçbir şey yazılmaz.
                    // yeni mesaj geldiyse yine de bir tur daha bakılır
                    log::debug!("cevapla [{kanal}]: sus");
                    if debug {
                        iz.push("sus (-)".to_string());
                        self.debug_izle(ctx, kanal, debug, &iz).await;
                    }
                    // hack şakası sussa da ilerler: yoksa HACK_DEVAM talimatı takılı kalır
                    if let Some(s) = self.durum().sohbetler.get_mut(&kanal) {
                        s.hackli = s.hackli.saturating_sub(1);
                    }
                    if yeni_mesaj_var(&self.durum(), kanal, gelen) {
                        drop(_mesgul);
                        continue;
                    }
                    return;
                }
                Ok(AkisSonuc::Bos) => {
                    // akıştan kullanılır bir şey çıkmadı; yeni mesaj geldiyse onu ele al
                    if debug {
                        iz.push("akış boş → yedek uret".to_string());
                    }
                    if yeni_mesaj_var(&self.durum(), kanal, gelen) {
                        // bayrak elle bırakılır: yeni tur üstte yeniden insert eder
                        drop(_mesgul);
                        continue;
                    }
                    match self.uret(&gecmis, &talimat, butce, "sohbet").await {
                        Ok(c) => {
                            // tekrar elemesi burada da SATIR bazlı: kanal geçmişinde bot
                            // satırları tek tek duruyor, çok satırlı ham blob hiç eşleşmez
                            let mut yedek = cevap_parcala(soy(&c, &bot_adi));
                            yedek.satirlar.retain(|s| !self.tekrar_mi(kanal, s));
                            match self
                                .gonder_cevap(ctx, kanal, yedek, yanit, son_mesaj, None)
                                .await
                            {
                                Some(p) => p,
                                None => return,
                            }
                        }
                        Err(e) => {
                            log::error!("ai [uret yedek] [{kanal}]: {e}");
                            return;
                        }
                    }
                }
                Err(e) => {
                    log::error!("ai [gonder_akis] [{kanal}]: {e}");
                    return;
                }
            };

            if debug {
                // protokol metninden ne gittiğini say: tepki satırı ve görünen satırlar
                let tepki = cevap
                    .lines()
                    .find_map(|l| tepki_govdesi(l.trim()).map(|g| g.trim().to_string()));
                let satir = cevap
                    .lines()
                    .filter(|l| !l.trim().is_empty() && tepki_govdesi(l.trim()).is_none())
                    .count();
                let mut sonuc = format!("{satir} satır gönderildi");
                if let Some(t) = tepki {
                    sonuc = format!("{sonuc} · tepki {t}");
                }
                iz.push(sonuc);
                self.debug_izle(ctx, kanal, debug, &iz).await;
            }
            {
                let mut d = self.durum();
                if let Some(s) = d.sohbetler.get_mut(&kanal) {
                    s.gecmis.push(asistan(cevap));
                    s.sayac += 1;
                    s.hackli = s.hackli.saturating_sub(1);
                    // cevap gitti: hedef seçimi sıfırdan başlar
                    s.son_gelenler.clear();
                }
                d.son_aktivite.insert(kanal, Instant::now());
            }

            // sohbeti kapatma yok: sessiz kalırsa zaman aşımı kapatır (uyku tikinde)
            // cevap yazarken yeni mesaj geldiyse bir tur daha; yoksa çık
            let bekleyen = yeni_mesaj_var(&self.durum(), kanal, gelen);
            if !bekleyen {
                break;
            }
        }
    }
}

// ---------- araştırma ----------

