mod ajanlar;
mod hafiza;
mod promptlar;

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ajanlar::rastgele_resim;
use promptlar::*;
use serde::{Deserialize, Serialize};
use serenity::all::*;
use serenity::async_trait;
use tokio::time::sleep;

// ---------- ayarlar ----------

const MODEL: &str = "openai/gpt-4o-mini";
const MAX_MESAJ: u32 = 12; // bir sohbette en fazla kaç mesaj yazar
const VEDA_ESIGI: u32 = 9; // bu sayıdan sonra konuyu kapatmaya çalışır
const SANS: f64 = 0.10; // normal mesajlaşmaya %10 ihtimalle dalar
const BEKLEME: Duration = Duration::from_secs(3 * 60 * 60); // sohbetten kaçınca 3 saat o kanala girmez
const YORUM_SURESI: Duration = Duration::from_secs(2 * 60 * 60); // haber attıktan sonra 2 saat yorum bekler
const HABER_ARALIGI: Duration = Duration::from_secs(6 * 60 * 60); // ne sıklıkla hacker news'e bakar (ajanlar da bu turda çalışır)
const DURTME_ARALIGI: Duration = Duration::from_secs(60 * 60); // ne sıklıkla kendiliğinden laf atmayı dener
const DURTME_SANSI: f64 = 0.3; // her denemede %30 ihtimalle atar
const SAKA_ARALIGI: Duration = Duration::from_secs(3 * 60 * 60); // ne sıklıkla resim/hack şakası dener
const SAKA_SANSI: f64 = 0.1; // her denemede %10 (ortalama 30 saatte bir)
const HACK_PAYI: f64 = 0.3; // şakaların %30'u hacklenmiş taklidi, gerisi düz resim
const HACK_MESAJI: u32 = 3; // hack taklidi kaç cevap sürer (sonuncusu kendine geliş)
const GECMIS_GUN: i64 = 14; // açılışta kaç günlük mesaj okur
const HAFIZA_BOYU: usize = 2000; // akılda tuttuğu son mesaj sayısı
const SOHBET_BOYU: usize = 20; // bir sohbette modele giden son mesaj sayısı
const MESAJ_SINIRI: usize = 1900; // discord 2000 kabul ediyor, pay bırakıyoruz
const FAVORI: u64 = 259669117248864257; // bu kişiyi ne olursa olsun sever
const RESIM_KLASORU: &str = "resimler"; // şakalarda atılacak görseller
const DURUM_KLASORU: &str = "durum"; // ajanların öğrendikleri buraya yazılır

type Hata = Box<dyn std::error::Error + Send + Sync>;

// ---------- durum ----------

#[derive(Serialize, Clone)]
struct Mesaj {
    role: &'static str,
    content: String,
}

fn kullanici(metin: impl Into<String>) -> Mesaj {
    Mesaj {
        role: "user",
        content: metin.into(),
    }
}

fn asistan(metin: impl Into<String>) -> Mesaj {
    Mesaj {
        role: "assistant",
        content: metin.into(),
    }
}

#[derive(Default)]
struct Sohbet {
    gecmis: Vec<Mesaj>,
    sayac: u32,
    hackli: u32, // 0 değilse hacklenmiş taklidi sürüyor, her cevapta bir azalır
}

#[derive(Default)]
struct Durum {
    bot_adi: String,
    favori_adi: Option<String>,
    // ajanların ürettikleri (durum/ klasöründe de duruyor)
    profil: String,      // profilci
    huy: String,         // hoca
    duzeltmeler: String, // elestirmen
    kendim: String,      // gunlukcu: botun kendi hali
    dizin: String,       // hafıza dizini, her cevapta gider
    // gördükleri
    hafiza: VecDeque<String>, // sunucudaki son mesajlar, "isim: metin"
    kendi_mesajlarim: VecDeque<String>, // botun kendi son mesajları
    son_kanal: Option<ChannelId>,
    // sohbet takibi
    sohbetler: HashMap<ChannelId, Sohbet>,
    mesgul: HashSet<ChannelId>, // şu an cevap üretilen kanallar
    yasakli: HashMap<ChannelId, Instant>,
    haber_bekleyen: HashMap<ChannelId, Instant>,
    atilan_haberler: HashSet<u64>,
    taranan: HashSet<GuildId>,
}

impl Durum {
    // yeniden başlayınca sıfırdan öğrenmesin diye diskten okur
    fn yukle() -> Self {
        Durum {
            profil: hafiza::oku("profil.md"),
            huy: hafiza::oku("huy.md"),
            duzeltmeler: hafiza::oku("duzeltmeler.md"),
            kendim: hafiza::oku("kendim.md"),
            dizin: hafiza::dizin_yenile(),
            ..Durum::default()
        }
    }
}

struct Bot {
    durum: Mutex<Durum>,
    http: reqwest::Client,
    anahtar: String,
    haber_kanali: Option<ChannelId>,
}

impl Bot {
    fn durum(&self) -> MutexGuard<'_, Durum> {
        self.durum.lock().unwrap_or_else(|e| e.into_inner())
    }
}

// ---------- yardımcılar ----------

fn simdi_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn ad(u: &User) -> String {
    u.global_name.clone().unwrap_or_else(|| u.name.clone())
}

fn hatirla(d: &mut Durum, isim: &str, metin: &str) {
    d.hafiza.push_back(format!("{isim}: {metin}"));
    if d.hafiza.len() > HAFIZA_BOYU {
        d.hafiza.pop_front();
    }
}

fn son_mesajlar(d: &Durum, n: usize) -> String {
    let atla = d.hafiza.len().saturating_sub(n);
    d.hafiza
        .iter()
        .skip(atla)
        .cloned()
        .collect::<Vec<_>>()
        .join("\n")
}

fn dokum(gecmis: &[Mesaj], bot_adi: &str) -> String {
    gecmis
        .iter()
        .map(|m| {
            if m.role == "assistant" {
                format!("{bot_adi}: {}", m.content)
            } else {
                m.content.clone()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// modelden gelen metni discord'a göndermeden önce toparlar
fn temizle(mut metin: String, bot_adi: &str) -> String {
    metin = metin.trim().to_string();
    // "isim: metin" kalıbını taklit edip başına kendi adını koyabiliyor
    let onek = format!("{bot_adi}:");
    if metin.to_lowercase().starts_with(&onek.to_lowercase()) {
        metin = metin[onek.len()..].trim().to_string();
    }
    if metin.len() > 1 && metin.starts_with('"') && metin.ends_with('"') {
        metin = metin[1..metin.len() - 1].to_string();
    }
    if metin.chars().count() > MESAJ_SINIRI {
        metin = metin.chars().take(MESAJ_SINIRI).collect();
    }
    metin
}

// ```json ... ``` gibi süslerin içinden json'u çıkarır
fn json_ayikla(metin: &str) -> &str {
    match (metin.find('{'), metin.rfind('}')) {
        (Some(b), Some(s)) if s > b => &metin[b..=s],
        _ => metin,
    }
}

// ---------- yapay zeka ----------

#[derive(Deserialize)]
struct Yanit {
    choices: Vec<Secenek>,
}
#[derive(Deserialize)]
struct Secenek {
    message: Icerik,
}
#[derive(Deserialize)]
struct Icerik {
    content: Option<String>,
}

impl Bot {
    // openrouter'a ham istek; her şey buradan geçer
    async fn sor_ham(&self, govde: serde_json::Value) -> Result<String, Hata> {
        let yanit: Yanit = self
            .http
            .post("https://openrouter.ai/api/v1/chat/completions")
            .bearer_auth(&self.anahtar)
            .json(&govde)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let metin = yanit
            .choices
            .into_iter()
            .next()
            .and_then(|s| s.message.content)
            .ok_or("modelden boş yanıt geldi")?;
        Ok(metin.trim().to_string())
    }

    async fn sor(&self, sistem: &str, gecmis: &[Mesaj], max_tokens: u32) -> Result<String, Hata> {
        let mut mesajlar = vec![Mesaj {
            role: "system",
            content: sistem.to_string(),
        }];
        mesajlar.extend_from_slice(gecmis);
        self.sor_ham(serde_json::json!({
            "model": MODEL,
            "messages": mesajlar,
            "max_tokens": max_tokens,
        }))
        .await
    }

    // kişilikle konuşur: sohbet, hoş geldin, laf atma, haber tanıtma, şakalar.
    // sohbette kim var, ne konuşuluyor bakıp hafızadan yalnız ilgili parçaları getirir.
    async fn uret(&self, gecmis: &[Mesaj], talimat: &str, max_tokens: u32) -> Result<String, Hata> {
        let mut katilimcilar: Vec<String> = Vec::new();
        let mut metinler: Vec<String> = Vec::new();
        for m in gecmis.iter().filter(|m| m.role == "user") {
            match m.content.split_once(": ") {
                Some((isim, metin)) => {
                    if !katilimcilar.contains(&isim.to_string()) {
                        katilimcilar.push(isim.to_string());
                    }
                    metinler.push(metin.to_string());
                }
                None => metinler.push(m.content.clone()),
            }
        }
        let anahtar = hafiza::anahtarlar(&metinler);
        let (sistem, bot_adi) = {
            let d = self.durum();
            let getirilen = hafiza::getir(&katilimcilar, &anahtar, &d.hafiza, SOHBET_BOYU);
            (sistem_metni(&d, talimat, &getirilen), d.bot_adi.clone())
        };
        let cevap = self.sor(&sistem, gecmis, max_tokens).await?;
        Ok(temizle(cevap, &bot_adi))
    }

    // kişiliksiz, düz analiz: ajanlar bunu kullanır
    async fn analiz(&self, metin: &str, talimat: &str, max_tokens: u32) -> Result<String, Hata> {
        let girdi = kullanici(format!("{metin}\n\n---\n\n{talimat}"));
        self.sor(ANALIST, &[girdi], max_tokens).await
    }

    // mention'lar kapalı gider: model @everyone yazsa bile kimse pinglenmez.
    // gönderilen her şey kendi_mesajlarim'a düşer, hoca ve eleştirmen oradan okur.
    async fn gonder(
        &self,
        ctx: &Context,
        kanal: ChannelId,
        metin: &str,
        ping: Option<UserId>,
        dosya: Option<&PathBuf>,
    ) {
        let mut izin = CreateAllowedMentions::new();
        if let Some(u) = ping {
            izin = izin.users([u]);
        }
        let mut mesaj = CreateMessage::new().content(metin).allowed_mentions(izin);
        if let Some(yol) = dosya {
            match CreateAttachment::path(yol).await {
                Ok(ek) => mesaj = mesaj.add_file(ek),
                Err(e) => eprintln!("görsel okunamadı ({}): {e}", yol.display()),
            }
        }
        if let Err(e) = kanal.send_message(&ctx.http, mesaj).await {
            eprintln!("gönderilemedi ({kanal}): {e}");
            return;
        }
        let mut d = self.durum();
        d.kendi_mesajlarim.push_back(metin.to_string());
        if d.kendi_mesajlarim.len() > 50 {
            d.kendi_mesajlarim.pop_front();
        }
    }
}

// her cevabın sistem mesajı: çekirdek kişilik + ajanların öğrettikleri + o anki görev
fn sistem_metni(d: &Durum, talimat: &str, getirilen: &str) -> String {
    let favori_satiri = match &d.favori_adi {
        Some(f) => FAVORI_SATIRI.replace("{favori}", f),
        None => String::new(),
    };
    let mut s = KISILIK
        .replace("{ad}", &d.bot_adi)
        .replace("{favori_satiri}", &favori_satiri);

    let bolum = |s: &mut String, baslik: &str, icerik: &str| {
        if !icerik.trim().is_empty() {
            s.push_str("\n\n");
            s.push_str(baslik);
            s.push('\n');
            s.push_str(icerik.trim());
        }
    };
    bolum(&mut s, "HUYUN (hocanın son notu, buna göre davran)", &d.huy);
    bolum(&mut s, "BU GRUP HAKKINDA BİLDİKLERİN", &d.profil);
    bolum(
        &mut s,
        "HAFIZA DİZİNİ (kimi ve neyi biliyorsun; ayrıntı gerekince getiriliyor)",
        &d.dizin,
    );
    bolum(&mut s, "BU SOHBET İÇİN HAFIZADAN GETİRİLENLER", getirilen);
    bolum(&mut s, "SENİN SON HALİN", &d.kendim);
    bolum(
        &mut s,
        "KENDİNE NOTLAR (eleştirmenin son sohbetten çıkardığı dersler)",
        &d.duzeltmeler,
    );
    bolum(&mut s, "ŞU ANKİ GÖREVİN", talimat);
    s
}

// ---------- sohbet mekanizması ----------

fn sohbet_baslat(d: &mut Durum, kanal: ChannelId, acilis: Option<String>) -> &mut Sohbet {
    let mut s = Sohbet::default();
    if let Some(a) = acilis {
        s.gecmis.push(asistan(a));
        s.sayac = 1;
    }
    d.sohbetler.entry(kanal).or_insert(s)
}

fn sohbet_bitir(d: &mut Durum, kanal: ChannelId) -> Option<Sohbet> {
    d.haber_bekleyen.remove(&kanal);
    d.yasakli.insert(kanal, Instant::now() + BEKLEME);
    d.sohbetler.remove(&kanal)
}

fn girebilir_mi(d: &Durum, kanal: ChannelId) -> bool {
    d.yasakli.get(&kanal).is_none_or(|t| Instant::now() >= *t)
}

impl Bot {
    // açık sohbette sıradaki cevabı üretir; aynı kanalda aynı anda tek cevap üretilir,
    // o sırada gelen mesajlar geçmişe eklenir ve bir sonraki cevapta görülür
    async fn cevapla(&self, ctx: &Context, kanal: ChannelId) {
        let (gecmis, talimat) = {
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
            } else if s.sayac >= MAX_MESAJ - 1 {
                SON_MESAJ
            } else if s.sayac >= VEDA_ESIGI {
                VEDA_YAKLASIYOR
            } else {
                ""
            };
            let gecmis = s.gecmis.clone();
            d.mesgul.insert(kanal);
            (gecmis, talimat)
        };

        let cevap = match self.uret(&gecmis, talimat, 250).await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("ai hatası: {e}");
                self.durum().mesgul.remove(&kanal);
                return;
            }
        };
        self.gonder(ctx, kanal, &cevap, None, None).await;

        let biten = {
            let mut d = self.durum();
            d.mesgul.remove(&kanal);
            let bitti = match d.sohbetler.get_mut(&kanal) {
                Some(s) => {
                    s.gecmis.push(asistan(cevap));
                    s.sayac += 1;
                    s.hackli = s.hackli.saturating_sub(1);
                    s.sayac >= MAX_MESAJ
                }
                None => false,
            };
            if bitti {
                sohbet_bitir(&mut d, kanal)
            } else {
                None
            }
        };

        // sohbet bitti: günlükçü hafızaya yazar, eleştirmen botu değerlendirir
        if let Some(s) = biten {
            let bot_adi = self.durum().bot_adi.clone();
            let d = dokum(&s.gecmis, &bot_adi);
            let kanal_adi = kanal.name(ctx).await.unwrap_or_else(|_| kanal.to_string());
            self.gunlukcu(d.clone(), "biten sohbet", &kanal_adi).await;
            self.elestirmen(d).await;
        }
    }
}

// ---------- hafıza ----------

// sunucuya bağlanınca kanalların son iki haftasını okur
async fn gecmisi_oku(bot: &Bot, ctx: &Context, guild: &Guild) {
    let bot_id = ctx.cache.current_user().id;
    let uye = match guild.member(ctx, bot_id).await {
        Ok(u) => u,
        Err(e) => {
            eprintln!("{}: üyelik alınamadı: {e}", guild.name);
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
    for (_, isim, id, metin) in toplam.iter().skip(atla) {
        if *id == FAVORI {
            d.favori_adi = Some(isim.clone());
        }
        hatirla(&mut d, isim, metin);
    }
    println!("{}: {} mesaj okundu", guild.name, toplam.len());
}

// haber ve hoş geldin için kanal: ayarlanmışsa o, yoksa sunucunun sistem kanalı, o da yoksa en üstteki metin kanalı
fn varsayilan_kanal(bot: &Bot, ctx: &Context) -> Option<ChannelId> {
    if let Some(k) = bot.haber_kanali {
        return Some(k);
    }
    for gid in ctx.cache.guilds() {
        let Some(g) = ctx.cache.guild(gid) else {
            continue;
        };
        if let Some(k) = g.system_channel_id {
            return Some(k);
        }
        if let Some(k) = g
            .channels
            .values()
            .filter(|k| k.kind == ChannelType::Text)
            .min_by_key(|k| k.position)
        {
            return Some(k.id);
        }
    }
    None
}

// ---------- arka plan döngüleri ----------

// altı saatte bir: ajanlar çalışır, sonra hacker news'ten haber atılır
async fn haber_dongusu(bot: Arc<Bot>, ctx: Context) {
    loop {
        sleep(HABER_ARALIGI).await;

        bot.profilci().await;
        let son = son_mesajlar(&bot.durum(), 300);
        bot.gunlukcu(son, "6 saatlik gözlem, bot konuşmamış olabilir", "gozlem")
            .await;
        bot.hoca().await;

        let Some(kanal) = varsayilan_kanal(&bot, &ctx) else {
            continue;
        };
        if bot.durum().sohbetler.contains_key(&kanal) {
            continue;
        }

        let h = match bot.haberci().await {
            Ok(h) => h,
            Err(e) => {
                eprintln!("haberci: {e}");
                continue;
            }
        };
        let link = if h.url.starts_with("https://") || h.url.starts_with("http://") {
            h.url.clone()
        } else {
            format!("https://news.ycombinator.com/item?id={}", h.id)
        };
        let girdi = match bot
            .uret(&[kullanici(h.title.clone())], HABER_TANIT, 200)
            .await
        {
            Ok(g) => g,
            Err(e) => {
                eprintln!("ai hatası: {e}");
                continue;
            }
        };
        bot.gonder(&ctx, kanal, &format!("{girdi}\n{link}"), None, None)
            .await;

        let mut d = bot.durum();
        sohbet_baslat(&mut d, kanal, Some(girdi));
        d.haber_bekleyen
            .insert(kanal, Instant::now() + YORUM_SURESI);
        d.atilan_haberler.insert(h.id);
    }
}

// son konuşulan kanal boşsa ve bot oraya girebiliyorsa kanalı verir
fn bos_kanal(bot: &Bot) -> Option<(ChannelId, String)> {
    let d = bot.durum();
    let k = d.son_kanal?;
    if d.sohbetler.contains_key(&k) || !girebilir_mi(&d, k) || d.profil.is_empty() {
        return None;
    }
    Some((k, son_mesajlar(&d, 40)))
}

// arada bir, tanıdık biri gibi durup dururken laf atar
async fn durtme_dongusu(bot: Arc<Bot>, ctx: Context) {
    loop {
        sleep(DURTME_ARALIGI).await;
        if rand::random::<f64>() > DURTME_SANSI {
            continue;
        }
        let Some((kanal, son)) = bos_kanal(&bot) else {
            continue;
        };

        let laf = match bot.uret(&[kullanici(son)], DURUP_DURURKEN, 120).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("ai hatası: {e}");
                continue;
            }
        };
        bot.gonder(&ctx, kanal, &laf, None, None).await;
        sohbet_baslat(&mut bot.durum(), kanal, Some(laf));
    }
}

// arada bir resimler/ klasöründen bir görsel atar; bazen de hacklenmiş taklidiyle
async fn saka_dongusu(bot: Arc<Bot>, ctx: Context) {
    loop {
        sleep(SAKA_ARALIGI).await;
        if rand::random::<f64>() > SAKA_SANSI {
            continue;
        }
        let Some((kanal, _)) = bos_kanal(&bot) else {
            continue;
        };
        let Some(resim) = rastgele_resim() else {
            continue;
        };

        let hack = rand::random::<f64>() < HACK_PAYI;
        let metin = if hack {
            bot.uret(&[kullanici("şaka başlıyor")], HACK_GIRIS, 150)
                .await
        } else {
            bot.resimci(&resim).await
        };
        let metin = match metin {
            Ok(m) => m,
            Err(e) => {
                eprintln!("ai hatası: {e}");
                continue;
            }
        };
        bot.gonder(&ctx, kanal, &metin, None, Some(&resim)).await;

        let mut d = bot.durum();
        let s = sohbet_baslat(&mut d, kanal, Some(metin));
        if hack {
            s.hackli = HACK_MESAJI;
        }
    }
}

// ---------- discord olayları ----------

struct Handler {
    bot: Arc<Bot>,
    baslatildi: AtomicBool,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, hazir: Ready) {
        self.bot.durum().bot_adi = hazir.user.name.clone();
        println!("giriş yapıldı: {}", hazir.user.name);

        // ready yeniden bağlanınca tekrar gelir, döngüler bir kere başlasın
        if !self.baslatildi.swap(true, Ordering::SeqCst) {
            tokio::spawn(haber_dongusu(self.bot.clone(), ctx.clone()));
            tokio::spawn(durtme_dongusu(self.bot.clone(), ctx.clone()));
            tokio::spawn(saka_dongusu(self.bot.clone(), ctx));
        }
    }

    async fn guild_create(&self, ctx: Context, guild: Guild, _yeni: Option<bool>) {
        if !self.bot.durum().taranan.insert(guild.id) {
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
            if d.sohbetler.contains_key(&kanal) || !girebilir_mi(&d, kanal) {
                return;
            }
        }

        let selam = match self
            .bot
            .uret(
                &[kullanici(format!("{isim} sunucuya yeni katıldı."))],
                HOS_GELDIN,
                200,
            )
            .await
        {
            Ok(s) => s,
            Err(e) => {
                eprintln!("ai hatası: {e}");
                return;
            }
        };
        self.bot
            .gonder(
                &ctx,
                kanal,
                &format!("<@{}> {selam}", uye.user.id),
                Some(uye.user.id),
                None,
            )
            .await;
        sohbet_baslat(&mut self.bot.durum(), kanal, Some(selam));
    }

    async fn message(&self, ctx: Context, msg: Message) {
        // botlar, webhook'lar ve özel mesajlar dışarıda; bot-bot döngüsü olmasın
        if msg.author.bot || msg.webhook_id.is_some() || msg.guild_id.is_none() {
            return;
        }
        let metin = msg.content_safe(&ctx.cache);
        if metin.trim().is_empty() {
            return;
        }
        let kanal = msg.channel_id;
        let isim = ad(&msg.author);

        let cevap_ver = {
            let mut d = self.bot.durum();
            hatirla(&mut d, &isim, &metin);
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

            let mut acik = d.sohbetler.contains_key(&kanal);
            if !acik && girebilir_mi(&d, kanal) && rand::random::<f64>() < SANS {
                sohbet_baslat(&mut d, kanal, None);
                acik = true;
            }
            if let Some(s) = d.sohbetler.get_mut(&kanal) {
                s.gecmis.push(kullanici(format!("{isim}: {metin}")));
                if s.gecmis.len() > SOHBET_BOYU {
                    s.gecmis.drain(..s.gecmis.len() - SOHBET_BOYU);
                }
            }
            acik
        };

        if cevap_ver {
            self.bot.cevapla(&ctx, kanal).await;
        }
    }
}

// ---------- başlangıç ----------

fn ayar(isim: &str) -> Result<String, Hata> {
    match std::env::var(isim) {
        Ok(v) if !v.trim().is_empty() => Ok(v.trim().to_string()),
        _ => Err(format!("{isim} yok, .env dosyasına bak").into()),
    }
}

async fn kapanis_bekle() {
    let ctrl_c = tokio::signal::ctrl_c();
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("sinyal");
        tokio::select! {
            _ = ctrl_c => {},
            _ = term.recv() => {},
        }
    }
    #[cfg(not(unix))]
    {
        let _ = ctrl_c.await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Hata> {
    dotenvy::dotenv().ok();
    let token = ayar("DISCORD_TOKEN")?;
    let anahtar = ayar("OPENROUTER_KEY")?;
    let haber_kanali = match std::env::var("HABER_KANALI") {
        Ok(v) if !v.trim().is_empty() => Some(ChannelId::new(v.trim().parse()?)),
        _ => None,
    };
    for k in ["kisiler", "konular", "olaylar", "arsiv"] {
        std::fs::create_dir_all(PathBuf::from(DURUM_KLASORU).join(k))?;
    }
    std::fs::create_dir_all(RESIM_KLASORU)?;

    let bot = Arc::new(Bot {
        durum: Mutex::new(Durum::yukle()),
        http: reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()?,
        anahtar,
        haber_kanali,
    });

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler {
            bot,
            baslatildi: AtomicBool::new(false),
        })
        .await?;

    // ctrl+c veya sigterm gelince düzgün kapan
    let yonetici = client.shard_manager.clone();
    tokio::spawn(async move {
        kapanis_bekle().await;
        println!("kapanıyor");
        yonetici.shutdown_all().await;
    });

    client.start().await?;
    Ok(())
}
