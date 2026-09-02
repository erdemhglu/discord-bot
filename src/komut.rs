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
`!zihin` zihin kartı: kişiler/konular/olaylar (detay: `/zihin` menüsü)
`!düşünme göster|gizle|sessiz|kapat` düşünme kipi (göster: cevapla spoiler'da · gizle: düşünürken \"Düşünüyorum...\", cevap sonra · sessiz: arka planda düşünür, hiç iz göstermez · kapat: istekler reasoning'siz)
`!model [id]` modeli gösterir/değiştirir (yalnız favori)
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
                {
                    // planı silme: silinirse dakika sonra yeniden kurulup tekrar uyutur.
                    // onun yerine planlı uyku bitene kadar "zorla uyanık" kal
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
                let _ = msg.react(&ctx.http, '👍').await;
                self.uyku_gecisi(ctx).await;
            }
            "uyu" => {
                {
                    let mut d = self.durum();
                    let simdi = simdi_unix();
                    let sure: i64 = arg.parse().unwrap_or(8); // saat
                    d.uyanik_zorla = 0;
                    d.planlar.push(uyku::Plan {
                        gun: -1,
                        uykusuz_bas: None,
                        bas: simdi,
                        bit: simdi + sure * 3600,
                    });
                }
                let _ = msg.react(&ctx.http, '😴').await;
                self.uyku_gecisi(ctx).await;
            }
            "durum" => {
                let metin = modal::durum_metni(&self.durum());
                soyle(metin).await;
            }
            "zihin" => {
                // kanal mesajına bileşen konmaz (modal yalnız interaction'la açılır);
                // kart + yönlendirme yeterli
                let embedler = modal::zihin_embedleri(&self.durum());
                let mesaj = CreateMessage::new()
                    .content("detay için `/zihin`")
                    .embeds(embedler);
                if let Err(e) = kanal.send_message(&ctx.http, mesaj).await {
                    log::warn!("zihin kartı gönderilemedi: {e}");
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
