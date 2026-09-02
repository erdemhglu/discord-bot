// Test ve yönetim komutları. Handler::message metni `!` ya da `/` ile başıyorsa
// buraya düşer; tanınan komut işlenir ve mesaj sohbete girmez.

use super::*;

pub const YARDIM: &str = "\
komutlar:
`!sifirla [hepsi]` kanal yasağını ve açık sohbeti sıfırlar
`!haber` şimdi haber atar · `!sorun` kod derdi sorar · `!gez` gündem gezintisi yapar
`!saka` / `!hack` görsel şakası / hacklenmiş taklidi
`!ajanlar` profilci ve hocayı şimdi çalıştırır
`!uyan` uykuyu keser · `!uyu [saat]` test için uyutur
`!durum` evre, sayaçlar, model, düşünme, uyku, seyahat
`!zihin` zihni panel görseli olarak atar (detay: `/zihin` menüsü) · `!zihin test` son 30 satırı hemen günlükçüye verir (zihin zinciri teşhisi)
`!düşünme göster|gizle|sessiz|kapat` düşünme kipi (göster: cevapla spoiler'da · gizle: düşünürken \"Düşünüyorum...\", cevap sonra · sessiz: arka planda düşünür, hiç iz göstermez · kapat: istekler reasoning'siz)
`!model [id]` modeli gösterir/değiştirir (yalnız favori)
`!debug [aç|kapat]` karar izleri kanala düşer: isteklilik puanı/sebebi, hedef, ruh hali, sus/tepki, sohbet kapanışı
`!ayarlar` butonlu ayar paneli: düşünme kipi, debug, uyku (slash: `/ayarlar`)
slash: `/durum` `/yardim` kart, `/zihin` interaktif kart (kişi menüsü + bölüm butonları → detay modalları)";

impl Bot {
    // tanınan komutsa true döner
    pub async fn komut(&self, ctx: &Context, msg: &Message, komut: &str, arg: &str) -> bool {
        let kanal = msg.channel_id;
        log::debug!(
            "komut [{kanal}]: !{komut} arg=\"{arg}\" kullanıcı={}",
            msg.author.id
        );
        let soyle = |m: String| async move {
            let _ = kanal.say(&ctx.http, m).await;
        };
        match komut {
            "sifirla" => {
                {
                    let mut d = self.durum();
                    if arg.contains("hepsi") {
                        d.sohbetler.clear();
                        d.haber_bekleyen.clear();
                        d.mesgul.clear();
                    } else {
                        d.sohbetler.remove(&kanal);
                        d.haber_bekleyen.remove(&kanal);
                        d.mesgul.remove(&kanal);
                    }
                }
                let _ = msg.react(&ctx.http, '👍').await;
            }
            "haber" => {
                self.durum().sohbetler.remove(&kanal);
                if !self.haber_at(ctx, kanal).await {
                    soyle("haber bulamadım".into()).await;
                }
            }
            "sorun" => {
                self.durum().sohbetler.remove(&kanal);
                self.sorun_at(ctx, kanal).await;
            }
            "gez" => {
                self.gezgin().await;
                let _ = msg.react(&ctx.http, '👍').await;
            }
            "saka" | "hack" => {
                self.durum().sohbetler.remove(&kanal);
                self.saka_yap(ctx, kanal, komut == "hack").await;
            }
            "ajanlar" => {
                self.profilci().await;
                self.hoca().await;
                self.durum().dizin = hafiza::dizin_yenile();
                let _ = msg.react(&ctx.http, '👍').await;
            }
            "uyan" => {
                self.uyandir();
                let _ = msg.react(&ctx.http, '👍').await;
                self.uyku_gecisi(ctx).await;
            }
            "uyu" => {
                self.uyut(arg.parse().unwrap_or(8));
                let _ = msg.react(&ctx.http, '😴').await;
                self.uyku_gecisi(ctx).await;
            }
            "durum" => {
                let metin = modal::durum_metni(&self.durum());
                soyle(metin).await;
            }
            "debug" => {
                let acik = self.debug_ayarla(arg);
                soyle(if acik {
                    "debug açık: kararlar bu kanala düşecek (DEBUG_KANALI ayarlıysa oraya)".into()
                } else {
                    "debug kapalı".into()
                })
                .await;
            }
            "ayarlar" | "ayar" => {
                // butonlu panel; tıklayan interaction_create'te paneli yerinde yeniler
                let (embed, bilesenler) = {
                    let d = self.durum();
                    (modal::ayarlar_embed(&d), modal::ayarlar_bilesenleri(&d))
                };
                let mesaj = CreateMessage::new().embed(embed).components(bilesenler);
                if let Err(e) = kanal.send_message(&ctx.http, mesaj).await {
                    log::warn!("ayar paneli gönderilemedi: {e}");
                }
            }
            "zihin" if arg.trim().eq_ignore_ascii_case("test") => {
                // teşhis: bu kanalın son satırlarını hemen günlükçüye ver; zihin zincirinin
                // çalışıp çalışmadığı 40 dk beklemeden görülsün (reasoning'li modelde boş
                // dönüyordu, canlı log olmadan anlaşılmıyordu)
                let mut satirlar: Vec<String> = {
                    let d = self.durum();
                    d.kanal_gecmisi
                        .get(&kanal)
                        .map(|g| g.iter().rev().take(30).cloned().collect())
                        .unwrap_or_default()
                };
                if satirlar.is_empty() {
                    soyle("bu kanalda hatırladığım satır yok".into()).await;
                } else {
                    satirlar.reverse();
                    let kanal_adi = kanal.name(ctx).await.unwrap_or_else(|_| kanal.to_string());
                    let metin = match self
                        .gunlukcu(satirlar.join("\n"), "zihin testi", &kanal_adi)
                        .await
                    {
                        Ok(o) => format!(
                            "günlükçü: {} kişi, {} konu, {} olay yazıldı · model çıktısı {} karakter",
                            o.kisi, o.konu, o.olay, o.cikti
                        ),
                        Err(e) => format!("günlükçü başarısız: {}", hafiza::kirp(&e.to_string(), 300)),
                    };
                    soyle(metin).await;
                }
            }
            "zihin" => {
                // görsel: durumun kilitli alanları burada kopyalanır (guard satır
                // sonunda düşer), dosya okuma ve rasterize bloklayan iş olduğu için
                // spawn_blocking'e taşınır. PNG diske yazılmaz: iki kanalda aynı anda
                // !zihin aynı dosyaya yarışıp yarım görsel yollamasın, bayt bellekten gider
                let mut veri = zihin_gorsel::zihin_verisi(&self.durum());
                let uretim = tokio::task::spawn_blocking(move || {
                    zihin_gorsel::dosyalari_oku(&mut veri);
                    zihin_gorsel::zihin_png(&veri)
                })
                .await;
                match uretim {
                    Ok(Ok(png)) => {
                        let baslik = format!("zihnim, {}", hafiza::tarih());
                        let ek = CreateAttachment::bytes(png, zihin_gorsel::CIKTI_ADI);
                        self.gonder_ekli(ctx, kanal, &baslik, ek).await;
                    }
                    // görsel çıkmazsa eski embed kartı yine gider; !zihin boş dönmez
                    hata => {
                        match hata {
                            Ok(Err(e)) => log::warn!("zihin görseli üretilemedi: {e}"),
                            Err(e) => log::warn!("zihin görseli görevi düştü: {e}"),
                            Ok(Ok(_)) => unreachable!(),
                        }
                        let embedler = modal::zihin_embedleri(&self.durum());
                        let mesaj = CreateMessage::new()
                            .content("detay için `/zihin`")
                            .embeds(embedler);
                        if let Err(e) = kanal.send_message(&ctx.http, mesaj).await {
                            log::warn!("zihin kartı gönderilemedi: {e}");
                        }
                    }
                }
            }
            "düşünme" | "dusunme" => {
                let arg = arg.trim().to_lowercase();
                match DusunmeKip::arg_ile(&arg) {
                    Some(yeni) => {
                        self.durum().dusunme = yeni;
                        hafiza::yaz("dusunme.md", yeni.dosya_degeri());
                        soyle(format!("düşünme artık {}", yeni.ad())).await;
                        let _ = msg.react(&ctx.http, '👍').await;
                    }
                    None => {
                        let kip = self.durum().dusunme;
                        soyle(format!(
                            "düşünme şu an {} · !düşünme göster / gizle / sessiz / kapat",
                            kip.ad()
                        ))
                        .await;
                    }
                }
            }
            "model" => {
                if arg.is_empty() {
                    let m = self.durum().model.clone();
                    soyle(format!("şu an {m}")).await;
                } else if msg.author.id.get() != FAVORI {
                    soyle("onu sen değiştiremezsin".into()).await;
                } else if self.api_adres.contains("openrouter") && !self.model_var_mi(arg).await {
                    soyle("yok öyle model".into()).await;
                } else {
                    self.durum().model = arg.to_string();
                    hafiza::yaz("model.md", arg);
                    soyle(format!("tamam, {arg}")).await;
                }
            }
            "yardım" | "yardim" | "help" => {
                soyle(YARDIM.into()).await;
            }
            _ => return false,
        }
        true
    }

    // !uyan ve ayar paneli: planı silme (silinirse dakika sonra yeniden kurulup tekrar
    // uyutur), planlı uyku bitene kadar "zorla uyanık" kal
    pub fn uyandir(&self) {
        let mut d = self.durum();
        let simdi = simdi_unix();
        let bitis = d
            .planlar
            .iter()
            .filter(|p| p.bas <= simdi && simdi < p.bit)
            .map(|p| p.bit)
            .max()
            .unwrap_or(simdi + 6 * 3600);
        d.uyanik_zorla = bitis;
    }

    // !uyu [saat] ve ayar paneli: test için geçici uyku planı
    pub fn uyut(&self, saat: i64) {
        let mut d = self.durum();
        let simdi = simdi_unix();
        d.uyanik_zorla = 0;
        d.planlar.push(uyku::Plan {
            gun: -1,
            uykusuz_bas: None,
            bas: simdi,
            bit: simdi + saat * 3600,
        });
    }

    // !debug aç|kapat (boşsa tersine çevirir); durum/debug.md'de kalıcı. Yeni durumu döner
    pub fn debug_ayarla(&self, arg: &str) -> bool {
        let mut d = self.durum();
        let yeni = match arg.trim().to_lowercase().as_str() {
            "aç" | "ac" | "açık" | "acik" | "on" => true,
            "kapat" | "kapalı" | "kapali" | "off" => false,
            _ => !d.debug,
        };
        d.debug = yeni;
        hafiza::yaz("debug.md", if yeni { "acik" } else { "kapali" });
        yeni
    }

    // openrouter model listesinde var mı
    async fn model_var_mi(&self, id: &str) -> bool {
        #[derive(Deserialize)]
        struct Liste {
            data: Vec<Kayit>,
        }
        #[derive(Deserialize)]
        struct Kayit {
            id: String,
        }
        match self
            .http
            .get("https://openrouter.ai/api/v1/models")
            .send()
            .await
        {
            Ok(r) => r
                .json::<Liste>()
                .await
                .map(|l| l.data.iter().any(|k| k.id == id))
                .unwrap_or(false),
            Err(_) => true, // liste çekilemediyse engel olma
        }
    }
}
