mod ajanlar;
mod gelisim;
mod gundem;
mod hafiza;
mod promptlar;
mod seyahat;
mod uyku;

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

// sağlayıcılar: ikisi de openai uyumlu chat/completions; .env'de hangisinin anahtarı varsa o
const OPENROUTER_ADRES: &str = "https://openrouter.ai/api/v1/chat/completions";
const OPENROUTER_MODEL: &str = "openai/gpt-4o-mini";
const MISTRAL_ADRES: &str = "https://api.mistral.ai/v1/chat/completions";
const MISTRAL_MODEL: &str = "mistral-medium-latest";
const MAX_MESAJ: u32 = 12; // bir sohbette en fazla kaç mesaj yazar
const VEDA_ESIGI: u32 = 9; // bu sayıdan sonra konuyu kapatmaya çalışır
const SANS: f64 = 0.35; // normal mesajlaşmaya %10 ihtimalle dalar
const BEKLEME: Duration = Duration::from_secs(3 * 60 * 60); // sohbetten kaçınca 3 saat o kanala girmez
const YORUM_SURESI: Duration = Duration::from_secs(2 * 60 * 60); // haber attıktan sonra 2 saat yorum bekler
const HABER_ARALIGI: Duration = Duration::from_secs(6 * 60 * 60); // ne sıklıkla hacker news'e bakar (ajanlar da bu turda çalışır)
const DURTME_ARALIGI: Duration = Duration::from_secs(60 * 60); // ne sıklıkla kendiliğinden laf atmayı dener
const DURTME_SANSI: f64 = 0.3; // her denemede %30 ihtimalle atar
const SAKA_ARALIGI: Duration = Duration::from_secs(3 * 60 * 60); // ne sıklıkla resim/hack şakası dener
const SAKA_SANSI: f64 = 0.1; // her denemede %10 (ortalama 30 saatte bir)
const HACK_PAYI: f64 = 0.3; // şakaların %30'u hacklenmiş taklidi, gerisi düz resim
const SORUN_PAYI: f64 = 0.25; // laf atma turlarının bu kadarı yazılım kanalına "sikko sorun"
const HACK_MESAJI: u32 = 3; // hack taklidi kaç cevap sürer (sonuncusu kendine geliş)
const GECMIS_GUN: i64 = 14; // açılışta kaç günlük mesaj okur
const HAFIZA_BOYU: usize = 2000; // akılda tuttuğu son mesaj sayısı
const KANAL_GECMIS: usize = 60; // kanal başına diskte tutulan son satır (bot dahil)
const SOHBET_TOHUM: usize = 10; // yeni sohbet açılırken kanal geçmişinden alınan satır
const SOHBET_BOYU: usize = 20; // bir sohbette modele giden son mesaj sayısı
const MESAJ_SINIRI: usize = 1900; // discord 2000 kabul ediyor, pay bırakıyoruz
const FAVORI: u64 = 259669117248864257; // bu kişiyi ne olursa olsun sever
const GEZGIN_ARALIGI: Duration = Duration::from_secs(4 * 60 * 60); // ne sıklıkla internette gezer
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
    son_mesaj: Option<MessageId>, // cevaplanacak mesaj (discord yanıtı olarak)
    gelen: u32,
    etiketli: bool, // son gelen mesaj botu etiketledi mi (cevap yanıt olarak gider)  // gelen kullanıcı mesajı sayısı; cevap yazarken yenisi geldi mi anlamak için
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
    // gündem ve uyku
    gundem: String, // gezgin: son okudukları ve düşündükleri
    planlar: Vec<uyku::Plan>,
    uyuyor: bool,
    uyanik_zorla: i64, // !uyan sonrası bu ana kadar uyku planı işlemez (unix)
    kanal_gecmisi: HashMap<ChannelId, VecDeque<String>>, // kanal başına son satırlar, diskte de durur
    bekleyen_etiketler: Vec<(ChannelId, String)>,        // uyurken etiketlenmişse uyanınca döner
    gelisim: gelisim::Gelisim,                           // evre, sayaçlar, seçtiği isim
    kullanici_adi: String, // discord kullanıcı adı; bot_adi seçilen isim olabilir
    model: String,         // kullanılan model; !model ile değişir, durum/model.md'de kalır
    son_yol_mesaji: i64,   // seyahatte en son hangi gün yoldan yazdı
    duyurulan_seyahat: i64, // "yarın gidiyorum" dediği seyahatin başlangıç günü
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
            gundem: gundem::son_gundem(&hafiza::oku("gundem.md")),
            gelisim: gelisim::yukle(),
            kanal_gecmisi: hafiza::kanal_gecmisi_yukle()
                .into_iter()
                .map(|(id, v)| (ChannelId::new(id), v))
                .collect(),
            ..Durum::default()
        }
    }
}

struct Bot {
    durum: Mutex<Durum>,
    http: reqwest::Client,
    api_adres: String, // chat/completions adresi (openrouter ya da mistral)
    anahtar: String,
    haber_kanali: Option<ChannelId>,
    firecrawl: Option<String>, // yoksa sayfalar düz indirilir
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

// grubun son mesajlarının ortalama boyu (karakter); bot bunun iki katını geçemez
fn ortalama_boy(d: &Durum) -> usize {
    let son: Vec<usize> = d
        .hafiza
        .iter()
        .rev()
        .take(200)
        .map(|s| {
            s.split_once(": ")
                .map(|(_, m)| m.chars().count())
                .unwrap_or(0)
        })
        .filter(|n| *n > 0)
        .collect();
    if son.is_empty() {
        60
    } else {
        son.iter().sum::<usize>() / son.len()
    }
}

// cevabı cümle sınırında keser: önce ilk iki cümle, sonra karakter sınırı
fn kisalt(metin: &str, sinir: usize, en_cok_cumle: usize) -> String {
    let mut sonuc = String::new();
    let mut cumle = 0;
    for c in metin.chars() {
        sonuc.push(c);
        if matches!(c, '.' | '!' | '?' | '\n') {
            cumle += 1;
            if cumle >= en_cok_cumle {
                break;
            }
        }
        if sonuc.chars().count() >= sinir {
            // kelime ortasında kesmesin
            if let Some(i) = sonuc.rfind(' ') {
                if i > sinir / 2 {
                    sonuc.truncate(i);
                }
            }
            break;
        }
    }
    sonuc.trim().trim_end_matches(['.', ',']).to_string()
}

// grubun gerçek mesajlarından boy ve ton örneği: kısa olanlardan 12 tane
fn ornek_mesajlar(d: &Durum) -> String {
    let uygun: Vec<&String> = d
        .hafiza
        .iter()
        .rev()
        .take(300)
        .filter(|s| {
            let n = s
                .split_once(": ")
                .map(|(_, m)| m.chars().count())
                .unwrap_or(0);
            (4..=100).contains(&n)
        })
        .collect();
    if uygun.is_empty() {
        return String::new();
    }
    let adim = (uygun.len() / 12).max(1);
    uygun
        .iter()
        .step_by(adim)
        .take(12)
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

// kanalın geçmişine satır ekler ve dosyaya yazar; sohbet bitse, bot yeniden başlasa da kalır
fn kanal_not(d: &mut Durum, kanal: ChannelId, satir: String) {
    let g = d.kanal_gecmisi.entry(kanal).or_default();
    g.push_back(satir);
    while g.len() > KANAL_GECMIS {
        g.pop_front();
    }
    let icerik = g.iter().cloned().collect::<Vec<_>>().join("\n");
    hafiza::yaz(&format!("kanallar/{}.md", kanal.get()), &icerik);
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

// hata gövdesini tek satıra indirir
fn kirp_hata(metin: &str) -> String {
    metin
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(300)
        .collect()
}

// "DÜŞÜNCE: ... / CEVAP: ..." biçiminden yalnız cevabı alır; biçim yoksa metnin kendisi
fn cevap_ayikla(metin: &str) -> String {
    let m = metin.trim();
    for etiket in ["CEVAP:", "Cevap:", "cevap:"] {
        if let Some(i) = m.rfind(etiket) {
            return m[i + etiket.len()..].trim().trim_matches('"').to_string();
        }
    }
    // düşünce satırı var ama cevap etiketi yoksa düşünceyi at
    let temiz: Vec<&str> = m
        .lines()
        .filter(|l| !l.trim_start().to_uppercase().starts_with("DÜŞÜNCE"))
        .collect();
    temiz.join("\n").trim().to_string()
}

const DUSUN_SONRA_CEVAPLA: &str = "\n\nÖNCE tek satır düşün, şu biçimde: \"DÜŞÜNCE: kim yazdı, ne istiyor (şaka mı, laf sokma mı, \
istek mi, ciddi soru mu), buna nasıl bir cevap yakışır (kısa/uzun, sert/tatlı, ciddi/dalga).\" \
SONRA \"CEVAP: \" satırında yalnız gönderilecek mesajı yaz. Düşünce satırı kimseye gitmez, cevap satırı gider.";

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
        let cevap = self
            .http
            .post(&self.api_adres)
            .bearer_auth(&self.anahtar)
            .json(&govde)
            .send()
            .await?;
        let durum = cevap.status();
        let govde_metni = cevap.text().await?;
        if !durum.is_success() {
            // 404 çoğunlukla "bu isimde model yok" demek; gövdedeki mesajı ve modeli göster
            let model = govde.get("model").and_then(|m| m.as_str()).unwrap_or("?");
            return Err(format!("{durum} (model: {model}): {}", kirp_hata(&govde_metni)).into());
        }
        let yanit: Yanit = serde_json::from_str(&govde_metni)?;
        let metin = yanit
            .choices
            .into_iter()
            .next()
            .and_then(|s| s.message.content)
            .ok_or("modelden boş yanıt geldi")?;
        Ok(metin.trim().to_string())
    }

    async fn sor(&self, sistem: &str, gecmis: &[Mesaj], max_tokens: u32) -> Result<String, Hata> {
        self.sor_bolumlu(sistem, "", gecmis, max_tokens).await
    }

    // sistem mesajı iki blok: sabit blok cache_control ile işaretli (anthropic/gemini önbelleğe alır,
    // openai zaten öneki kendi önbellekler); değişken blok her seferinde yeniden okunur
    async fn sor_bolumlu(
        &self,
        sabit: &str,
        degisken: &str,
        gecmis: &[Mesaj],
        max_tokens: u32,
    ) -> Result<String, Hata> {
        let sistem = if degisken.is_empty() {
            serde_json::json!({ "role": "system", "content": sabit })
        } else {
            serde_json::json!({ "role": "system", "content": [
                { "type": "text", "text": sabit, "cache_control": { "type": "ephemeral" } },
                { "type": "text", "text": degisken }
            ]})
        };
        let mut mesajlar = vec![sistem];
        mesajlar.extend(
            gecmis
                .iter()
                .map(|m| serde_json::json!({ "role": m.role, "content": m.content })),
        );
        self.sor_ham(serde_json::json!({
            "model": self.durum().model.clone(),
            "messages": mesajlar,
            "max_tokens": max_tokens,
            "temperature": 0.8,
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
        let (sabit, degisken, bot_adi) = {
            let d = self.durum();
            let getirilen = hafiza::getir(&katilimcilar, &anahtar, &d.hafiza, SOHBET_BOYU);
            let (sabit, degisken) = sistem_metni(&d, talimat, &getirilen);
            (sabit, degisken, d.bot_adi.clone())
        };
        let cevap = self
            .sor_bolumlu(&sabit, &degisken, gecmis, max_tokens)
            .await?;
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
        yanit: Option<MessageId>, // verilirse discord yanıtı olur, kişi etiketlenir
    ) {
        let mut izin = CreateAllowedMentions::new();
        if let Some(u) = ping {
            izin = izin.users([u]);
        }
        if yanit.is_some() {
            izin = izin.replied_user(true);
        }
        let mut mesaj = CreateMessage::new().content(metin).allowed_mentions(izin);
        if let Some(id) = yanit {
            mesaj = mesaj.reference_message((kanal, id));
        }
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
        let satir = format!("{}: {}", d.bot_adi, metin);
        kanal_not(&mut d, kanal, satir);
    }
}

// her cevabın sistem mesajı: çekirdek kişilik + ajanların öğrettikleri + o anki görev
// her cevabın sistem mesajı iki parça: SABİT (kişilik, huy, profil, dizin...; ajan çalışınca
// değişir, prompt cache buradan kazanır) ve DEĞİŞKEN (örnek mesajlar, getirilenler, saat, görev)
fn sistem_metni(d: &Durum, talimat: &str, getirilen: &str) -> (String, String) {
    let favori_satiri = match &d.favori_adi {
        Some(f) => FAVORI_SATIRI.replace("{favori}", f),
        None => String::new(),
    };
    let bolum = |s: &mut String, baslik: &str, icerik: &str| {
        if !icerik.trim().is_empty() {
            if !s.is_empty() {
                s.push_str("\n\n");
            }
            s.push_str(baslik);
            s.push('\n');
            s.push_str(icerik.trim());
        }
    };

    let mut sabit = KISILIK
        .replace("{ad}", &d.bot_adi)
        .replace("{favori_satiri}", &favori_satiri);
    bolum(
        &mut sabit,
        "GELİŞİM EVREN",
        &gelisim::evre_metni(&d.gelisim),
    );
    bolum(
        &mut sabit,
        "HUYUN (hocanın son notu, buna göre davran)",
        &d.huy,
    );
    bolum(&mut sabit, "BU GRUP HAKKINDA BİLDİKLERİN", &d.profil);
    bolum(
        &mut sabit,
        "HAFIZA DİZİNİ (kimi ve neyi biliyorsun; ayrıntı gerekince getiriliyor)",
        &d.dizin,
    );
    bolum(
        &mut sabit,
        "GÜNDEM (internette gezerken okudukların ve düşündüklerin)",
        &d.gundem,
    );
    bolum(&mut sabit, "SENİN SON HALİN", &d.kendim);
    bolum(
        &mut sabit,
        "KENDİNE NOTLAR (eleştirmenin son sohbetten çıkardığı dersler)",
        &d.duzeltmeler,
    );

    let mut degisken = String::new();
    bolum(
        &mut degisken,
        &format!(
            "GRUBUN GERÇEK MESAJLARI (boy ve ton örneği; ortalama {} karakter, sıradan lafta bunu geçme)",
            ortalama_boy(d)
        ),
        &ornek_mesajlar(d),
    );
    bolum(
        &mut degisken,
        "BU SOHBET İÇİN HAFIZADAN GETİRİLENLER",
        getirilen,
    );
    bolum(
        &mut degisken,
        "ŞU AN",
        &format!("{} {}", uyku::durum_metni(d), seyahat::durum_metni()),
    );
    bolum(&mut degisken, "ŞU ANKİ GÖREVİN", talimat);
    (sabit, degisken)
}

// ---------- sohbet mekanizması ----------

fn sohbet_baslat(d: &mut Durum, kanal: ChannelId, acilis: Option<String>) -> &mut Sohbet {
    let mut s = Sohbet::default();
    // kanalın son satırlarıyla başla ki daha önce ne konuşulduğunu bilsin
    let onek = format!("{}: ", d.bot_adi);
    if let Some(g) = d.kanal_gecmisi.get(&kanal) {
        let atla = g.len().saturating_sub(SOHBET_TOHUM);
        for satir in g.iter().skip(atla) {
            match satir.strip_prefix(&onek) {
                Some(m) => s.gecmis.push(asistan(m)),
                None => s.gecmis.push(kullanici(satir.clone())),
            }
        }
    }
    if let Some(a) = acilis {
        // açılış mesajı zaten gönderilip geçmişe düşmüş olabilir, iki kez olmasın
        if s.gecmis
            .last()
            .is_some_and(|m| m.role == "assistant" && m.content == a)
        {
            s.gecmis.pop();
        }
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
                } else if s.sayac >= MAX_MESAJ - 1 {
                    SON_MESAJ
                } else if s.sayac >= VEDA_ESIGI {
                    VEDA_YAKLASIYOR
                } else {
                    ""
                };
                d.mesgul.insert(kanal);
                talimat
            };

            // Kısa bir okuma payı bırak; peş peşe yazılanları tek bağlamda gör.
            sleep(Duration::from_millis(400 + (rand::random::<u64>() % 800))).await;
            let (gecmis, yanit, gelen, sinir, en_cok_cumle, token, son_metin) = {
                let d = self.durum();
                let Some(s) = d.sohbetler.get(&kanal) else {
                    drop(d);
                    self.durum().mesgul.remove(&kanal);
                    return;
                };
                // sınır gelen mesaja göre: kısa laf → kısa cevap; soru ya da uzun mesaj → yer aç
                let ort = ortalama_boy(&d);
                let son = s
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
                    .to_lowercase();
                // üç kademe: sıradan laf kısa; soru/orta boy mesaj orta; ciddi konu uzun
                let gecen = |liste: &[&str]| liste.iter().any(|k| son.contains(k));
                let ciddi = gecen(&[
                    "ciddi",
                    "anlat",
                    "açıkla",
                    "ne düşün",
                    "felsefe",
                    "sence",
                    "neden",
                ]) || son.chars().count() > 150;
                let soru = son.contains('?')
                    || gecen(&["nasıl", "olsa", "olsan", "mı ", "mi ", "mu ", "mü "]);
                let (sinir, en_cok_cumle, token) = if ciddi {
                    ((ort * 6).clamp(220, 600), 5, 220)
                } else if soru || son.chars().count() > 60 {
                    ((ort * 4).clamp(140, 360), 3, 140)
                } else {
                    ((ort * 2).clamp(40, 220), 2, 90)
                };
                let son_metin = s
                    .gecmis
                    .iter()
                    .rev()
                    .find(|m| m.role == "user")
                    .map(|m| {
                        m.content
                            .split_once(": ")
                            .map(|(_, t)| t.to_string())
                            .unwrap_or(m.content.clone())
                    })
                    .unwrap_or_default();
                (
                    s.gecmis.clone(),
                    s.son_mesaj,
                    s.gelen,
                    sinir,
                    en_cok_cumle,
                    token,
                    son_metin,
                )
            };
            // istendiyse internete bak (haber, araştır, link) ve bulduklarını göreve ekle
            let mut talimat_tam = talimat.to_string();
            let mut token = token;
            if let Some(bulgu) = self.arastir(&son_metin).await {
                talimat_tam = format!(
                    "{talimat}\n\nİNTERNETTEN ŞİMDİ ÇEKTİKLERİN (istendiği için baktın; kendi ağzınla anlat, liste yapma, \"kaynak\" deme):\n{bulgu}"
                );
                token = token.max(220);
            }
            // küçük modeller için: önce bir satır düşün (kimseye gitmez), sonra cevap
            talimat_tam.push_str(DUSUN_SONRA_CEVAPLA);
            let token = token + 70;
            let cevap = match self.uret(&gecmis, &talimat_tam, token).await {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("ai hatası: {e}");
                    self.durum().mesgul.remove(&kanal);
                    return;
                }
            };
            let cevap = cevap_ayikla(&cevap);
            let cevap = if talimat.is_empty() || talimat == VEDA_YAKLASIYOR || talimat == SON_MESAJ
            {
                kisalt(&cevap, sinir, en_cok_cumle)
            } else {
                cevap
            };
            // boş ya da önceki mesajın kırıntısı ("'cım" gibi) gitmesin
            if cevap.chars().count() < 3 || cevap.starts_with('\'') {
                self.durum().mesgul.remove(&kanal);
                return;
            }
            // aynı lafı iki kez etmesin: son 5 mesajıyla aynıysa bir kez daha dener, yine aynıysa susar
            let cevap = if self.tekrar_mi(kanal, &cevap) {
                let t2 = format!("{talimat_tam}\n\nAz önce aynen şunu yazdın: \"{cevap}\". Aynısını ya da benzerini yazma; başka bir açıdan gir ya da konuyu değiştir.");
                match self.uret(&gecmis, &t2, token).await {
                    Ok(c)
                        if !self.tekrar_mi(kanal, &cevap_ayikla(&c)) && c.chars().count() >= 3 =>
                    {
                        kisalt(&cevap_ayikla(&c), sinir, en_cok_cumle)
                    }
                    _ => {
                        self.durum().mesgul.remove(&kanal);
                        return;
                    }
                }
            } else {
                cevap
            };
            // "yazıyor..." gösterip boyuna göre kısa bir yazma payı bırakır.
            let _ = kanal.broadcast_typing(&ctx.http).await;
            sleep(Duration::from_millis(
                (cevap.chars().count() as u64 * 25).clamp(350, 2500),
            ))
            .await;
            // insan gibi: kalabalıkta, araya mesaj girdiyse ya da etiketlendiyse yanıtla; yoksa düz yaz
            let yanitla = {
                let d = self.durum();
                let s = d.sohbetler.get(&kanal);
                let etiketli = s.is_some_and(|s| s.etiketli);
                let araya_girdi = s.is_some_and(|s| s.gelen > gelen);
                let bot_onek = format!("{}: ", d.bot_adi);
                let konusan: HashSet<&str> = d
                    .kanal_gecmisi
                    .get(&kanal)
                    .map(|g| {
                        g.iter()
                            .rev()
                            .take(5)
                            .filter(|l| !l.starts_with(&bot_onek))
                            .filter_map(|l| l.split_once(": ").map(|(i, _)| i))
                            .collect()
                    })
                    .unwrap_or_default();
                etiketli || araya_girdi || konusan.len() >= 2
            };
            let yanit = if yanitla { yanit } else { None };
            self.gonder(ctx, kanal, &cevap, None, None, yanit).await;

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
                self.durum().gelisim.sohbet += 1;
                self.gelisim_kontrol(ctx).await;
                return;
            }
            // cevap yazarken yeni mesaj geldiyse bir tur daha; yoksa çık
            let bekleyen = self
                .durum()
                .sohbetler
                .get(&kanal)
                .is_some_and(|s| s.gelen > gelen);
            if !bekleyen {
                break;
            }
        }
    }
}

// ---------- araştırma ----------

impl Bot {
    // botun son 5 mesajından biriyle aynı mı
    fn tekrar_mi(&self, kanal: ChannelId, cevap: &str) -> bool {
        let d = self.durum();
        let onek = format!("{}: ", d.bot_adi);
        let hedef = cevap.trim().to_lowercase();
        d.kanal_gecmisi
            .get(&kanal)
            .map(|g| {
                g.iter()
                    .rev()
                    .filter_map(|l| l.strip_prefix(&onek))
                    .take(5)
                    .any(|l| l.trim().to_lowercase() == hedef)
            })
            .unwrap_or(false)
    }

    // mesaj internete bakmayı gerektiriyorsa bakar: link → sayfa; "araştır/bak" → firecrawl arama
    // (anahtar varsa); "haber/gündem/ne oldu" → rss başlıkları
    async fn arastir(&self, metin: &str) -> Option<String> {
        let m = metin.to_lowercase();
        if let Some(url) = metin
            .split_whitespace()
            .find(|w| w.starts_with("http://") || w.starts_with("https://"))
        {
            let url = url.trim_end_matches(['>', ')', ',', '.']);
            return match self.sayfa_oku(url).await {
                Ok(s) if !s.trim().is_empty() => {
                    Some(format!("Atılan link ({url}):\n{}", hafiza::kirp(&s, 1500)))
                }
                _ => Some(format!("Link açılamadı: {url}")),
            };
        }
        let gecen = |liste: &[&str]| liste.iter().any(|k| m.contains(k));
        let haber = gecen(&[
            "haber",
            "gündem",
            "ne oldu",
            "son dakika",
            "neler oluyor",
            "güncel",
        ]);
        let tetik = [
            "araştır",
            "bak bakalım",
            "baksana",
            "bi bak",
            "googlela",
            "ara bakalım",
            "arasana",
            "internete bak",
            "internetten bak",
        ];
        let ara = gecen(&tetik);
        if ara && self.firecrawl.is_some() {
            let mut sorgu = m.clone();
            for k in tetik
                .iter()
                .chain(["bakar mısın", " lan", " la ", " aq"].iter())
            {
                sorgu = sorgu.replace(k, " ");
            }
            let sorgu: String = sorgu
                .split_whitespace()
                .filter(|w| !w.starts_with('@'))
                .collect::<Vec<_>>()
                .join(" ");
            let sorgu = if sorgu.trim().is_empty() {
                metin.to_string()
            } else {
                sorgu
            };
            if let Ok(sonuc) = self.firecrawl_ara(&sorgu).await {
                return Some(format!("\"{sorgu}\" araması:\n{sonuc}"));
            }
        }
        if haber || ara {
            if let Ok(rss) = gundem::rss(&self.http).await {
                let liste = rss
                    .iter()
                    .take(12)
                    .map(|h| format!("- {} — {}", h.baslik, hafiza::kirp(&h.ozet, 100)))
                    .collect::<Vec<_>>()
                    .join("\n");
                return Some(format!("Sözcü'den şu anki başlıklar:\n{liste}"));
            }
        }
        None
    }
}

// ---------- gelişim ----------

impl Bot {
    // hak edilen evreye atlar, kaydeder; yerleşik olunca isim seçer
    async fn gelisim_kontrol(&self, ctx: &Context) {
        let isim_gerek = {
            let mut d = self.durum();
            let hak = gelisim::hak_edilen(&d.gelisim);
            if hak > d.gelisim.evre {
                d.gelisim.evre = hak;
                println!("gelisim: {} evresine geçti", gelisim::evre(&d.gelisim).ad);
            }
            gelisim::kaydet(&d.gelisim);
            d.gelisim.isim.is_none() && d.gelisim.evre >= gelisim::ISIM_EVRESI
        };
        if isim_gerek {
            self.isim_sec(ctx).await;
        }
    }

    // kendine isim seçer, takma adını her sunucuda değiştirir, gruba söyler
    async fn isim_sec(&self, ctx: &Context) {
        let cevap = match self
            .uret(&[kullanici("isim seçme vakti")], ISIM_SEC, 12)
            .await
        {
            Ok(c) => c,
            Err(e) => return eprintln!("isim: {e}"),
        };
        let Some(isim) = gelisim::isim_temizle(&cevap) else {
            return eprintln!("isim: seçim çözülemedi: {cevap}");
        };
        for gid in ctx.cache.guilds() {
            if let Err(e) = gid.edit_nickname(&ctx.http, Some(&isim)).await {
                eprintln!("isim: takma ad değiştirilemedi ({gid}): {e}");
            }
        }
        {
            let mut d = self.durum();
            d.gelisim.isim = Some(isim.clone());
            d.bot_adi = isim.clone();
            gelisim::kaydet(&d.gelisim);
        }
        println!("gelisim: yeni isim {isim}");

        let Some(kanal) = varsayilan_kanal(self, ctx) else {
            return;
        };
        match self
            .uret(
                &[kullanici("ismini seçtin")],
                &ISIM_DUYURU.replace("{isim}", &isim),
                150,
            )
            .await
        {
            Ok(duyuru) => {
                self.gonder(ctx, kanal, &duyuru, None, None, None).await;
                sohbet_baslat(&mut self.durum(), kanal, Some(duyuru));
            }
            Err(e) => eprintln!("isim: {e}"),
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
        if !uyku::uyanik_mi(&bot.durum()) {
            continue;
        }
        if seyahat::simdi().is_some() {
            // yolda haber atmaz ama ajanlar gene çalışsın, öğrenmeye devam
            bot.profilci().await;
            bot.hoca().await;
            continue;
        }

        bot.gelisim_kontrol(&ctx).await;
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

        bot.haber_at(&ctx, kanal).await;
    }
}

impl Bot {
    // küçük, uydurma ama inandırıcı bir yazılım derdi atar, "nasıl çözerim" diye sorar
    async fn sorun_at(&self, ctx: &Context, kanal: ChannelId) {
        let son = son_mesajlar(&self.durum(), 30);
        match self.uret(&[kullanici(son)], SORUN, 160).await {
            Ok(laf) => {
                self.gonder(ctx, kanal, &laf, None, None, None).await;
                sohbet_baslat(&mut self.durum(), kanal, Some(laf));
            }
            Err(e) => eprintln!("ai hatası: {e}"),
        }
    }

    // seçilmiş bir haberi kanala atar ve yorum bekleme sohbeti açar
    async fn haber_at(&self, ctx: &Context, kanal: ChannelId) -> bool {
        let h = match self.haberci().await {
            Ok(h) => h,
            Err(e) => {
                eprintln!("haberci: {e}");
                return false;
            }
        };
        let link = if h.url.starts_with("https://") || h.url.starts_with("http://") {
            h.url.clone()
        } else {
            format!("https://news.ycombinator.com/item?id={}", h.id)
        };
        let girdi = match self
            .uret(&[kullanici(h.title.clone())], HABER_TANIT, 200)
            .await
        {
            Ok(g) => g,
            Err(e) => {
                eprintln!("ai hatası: {e}");
                return false;
            }
        };
        self.gonder(ctx, kanal, &format!("{girdi}\n{link}"), None, None, None)
            .await;

        let mut d = self.durum();
        sohbet_baslat(&mut d, kanal, Some(girdi));
        d.haber_bekleyen
            .insert(kanal, Instant::now() + YORUM_SURESI);
        d.atilan_haberler.insert(h.id);
        true
    }

    // görsel şakası; hack ise hacklenmiş taklidiyle başlar
    async fn saka_yap(&self, ctx: &Context, kanal: ChannelId, hack: bool) {
        let Some(resim) = rastgele_resim() else {
            let _ = kanal.say(&ctx.http, "resimler klasörü boş").await;
            return;
        };
        let metin = if hack {
            self.uret(&[kullanici("şaka başlıyor")], HACK_GIRIS, 150)
                .await
        } else {
            self.resimci(&resim).await
        };
        let metin = match metin {
            Ok(m) => m,
            Err(e) => {
                eprintln!("ai hatası: {e}");
                return;
            }
        };
        self.gonder(ctx, kanal, &metin, None, Some(&resim), None)
            .await;

        let mut d = self.durum();
        let s = sohbet_baslat(&mut d, kanal, Some(metin));
        if hack {
            s.hackli = HACK_MESAJI;
        }
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
        if !uyku::uyanik_mi(&bot.durum()) {
            continue;
        }
        // seyahat: gitmeden bir gün önce haber ver, yoldayken günde bir mesaj, başka laf atma
        let talimat = if let Some(s) = seyahat::simdi() {
            if bot.durum().son_yol_mesaji == seyahat::bugun() || rand::random::<f64>() > 0.25 {
                continue;
            }
            bot.durum().son_yol_mesaji = seyahat::bugun();
            let _ = s;
            YOLDA
        } else if let Some(s) = seyahat::yarin() {
            if bot.durum().duyurulan_seyahat == s.bas {
                continue;
            }
            bot.durum().duyurulan_seyahat = s.bas;
            GIDIYORUM
        } else {
            if rand::random::<f64>() > DURTME_SANSI * gelisim::evre(&bot.durum().gelisim).durtme {
                continue;
            }
            if rand::random::<f64>() < SORUN_PAYI {
                // yazılım kanalına küçük bir kod derdi at
                if let Some(kanal) = varsayilan_kanal(&bot, &ctx) {
                    if !bot.durum().sohbetler.contains_key(&kanal) {
                        bot.sorun_at(&ctx, kanal).await;
                    }
                }
                continue;
            }
            DURUP_DURURKEN
        };
        let Some((kanal, son)) = bos_kanal(&bot) else {
            continue;
        };

        let laf = match bot.uret(&[kullanici(son)], talimat, 120).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("ai hatası: {e}");
                continue;
            }
        };
        bot.gonder(&ctx, kanal, &laf, None, None, None).await;
        sohbet_baslat(&mut bot.durum(), kanal, Some(laf));
    }
}

// arada bir resimler/ klasöründen bir görsel atar; bazen de hacklenmiş taklidiyle
async fn saka_dongusu(bot: Arc<Bot>, ctx: Context) {
    loop {
        sleep(SAKA_ARALIGI).await;
        if !uyku::uyanik_mi(&bot.durum()) || seyahat::simdi().is_some() {
            continue;
        }
        if rand::random::<f64>() > SAKA_SANSI {
            continue;
        }
        let Some((kanal, _)) = bos_kanal(&bot) else {
            continue;
        };
        bot.saka_yap(&ctx, kanal, rand::random::<f64>() < HACK_PAYI)
            .await;
    }
}

// arada internette gezer; ilk gezinti açılıştan 10 dk sonra, sonra 4 saatte bir
async fn gezgin_dongusu(bot: Arc<Bot>) {
    let mut ilk = true;
    loop {
        sleep(if ilk {
            Duration::from_secs(600)
        } else {
            GEZGIN_ARALIGI
        })
        .await;
        ilk = false;
        if uyku::uyanik_mi(&bot.durum()) {
            bot.gezgin().await;
        }
    }
}

// dakikada bir uyku planına bakar; uyanınca uyurken gelen etiketlere döner
async fn uyku_dongusu(bot: Arc<Bot>, ctx: Context) {
    loop {
        sleep(Duration::from_secs(60)).await;
        {
            let mut d = bot.durum();
            uyku::guncelle(&mut d);
        }
        bot.uyku_gecisi(&ctx).await;
    }
}

impl Bot {
    // uyudu/uyandı geçişini işler; uyanınca uyurken gelen etiketlere döner
    async fn uyku_gecisi(&self, ctx: &Context) {
        let bekleyen = {
            let mut d = self.durum();
            let uyanik = uyku::uyanik_mi(&d);
            if uyanik == d.uyuyor {
                println!("uyku: {}", if uyanik { "uyandı" } else { "uyudu" });
            }
            let uyandi = uyanik && d.uyuyor;
            d.uyuyor = !uyanik;
            if uyandi {
                std::mem::take(&mut d.bekleyen_etiketler)
            } else {
                Vec::new()
            }
        };
        let Some((kanal, _)) = bekleyen.last() else {
            return;
        };
        let kanal = *kanal;
        let liste = bekleyen
            .iter()
            .map(|(_, m)| format!("- {m}"))
            .collect::<Vec<_>>()
            .join("\n");
        match self
            .uret(
                &[kullanici(format!("uyurken sana yazılanlar:\n{liste}"))],
                UYANDIM,
                200,
            )
            .await
        {
            Ok(c) => {
                self.gonder(ctx, kanal, &c, None, None, None).await;
                sohbet_baslat(&mut self.durum(), kanal, Some(c));
            }
            Err(e) => eprintln!("ai hatası: {e}"),
        }
    }
}

// ---------- discord olayları ----------

impl Bot {
    // test ve yönetim komutları; tanınan komutsa true döner
    async fn komut(&self, ctx: &Context, msg: &Message, komut: &str, arg: &str) -> bool {
        let kanal = msg.channel_id;
        let soyle = |m: String| async move {
            let _ = kanal.say(&ctx.http, m).await;
        };
        match komut {
            "sifirla" => {
                {
                    let mut d = self.durum();
                    if arg.contains("hepsi") {
                        d.yasakli.clear();
                        d.sohbetler.clear();
                        d.haber_bekleyen.clear();
                        d.mesgul.clear();
                    } else {
                        d.yasakli.remove(&kanal);
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
                let metin =
                    {
                        let d = self.durum();
                        let g = &d.gelisim;
                        format!(
                        "evre: {} ({}. gün, {} sohbet, {} mesaj) · model: {} · {} · seyahat: {}",
                        gelisim::evre(g).ad,
                        gelisim::gun(g) + 1,
                        g.sohbet,
                        g.mesaj,
                        d.model,
                        if uyku::uyanik_mi(&d) { "uyanık" } else { "uyuyor" },
                        seyahat::simdi().map(|s| s.yer).unwrap_or("yok"),
                    )
                    };
                soyle(metin).await;
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

struct Handler {
    bot: Arc<Bot>,
    baslatildi: AtomicBool,
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
        println!("giriş yapıldı: {}", hazir.user.name);

        // ready yeniden bağlanınca tekrar gelir, döngüler bir kere başlasın
        if !self.baslatildi.swap(true, Ordering::SeqCst) {
            tokio::spawn(haber_dongusu(self.bot.clone(), ctx.clone()));
            tokio::spawn(durtme_dongusu(self.bot.clone(), ctx.clone()));
            tokio::spawn(saka_dongusu(self.bot.clone(), ctx.clone()));
            tokio::spawn(gezgin_dongusu(self.bot.clone()));
            tokio::spawn(uyku_dongusu(self.bot.clone(), ctx));
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
        let bot_id = ctx.cache.current_user().id;

        // komutlar: ! ya da / ile başlar; bilinmeyen komut normal mesaj sayılır
        let kelime = metin.trim();
        if kelime.starts_with('!') || kelime.starts_with('/') {
            let mut parcalar = kelime[1..].split_whitespace();
            let komut = parcalar.next().unwrap_or("").to_lowercase();
            let arg = parcalar.collect::<Vec<_>>().join(" ");
            if self.bot.komut(&ctx, &msg, &komut, &arg).await {
                return;
            }
        }

        let cevap_ver = {
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

            // etiketlenince her zaman cevap verir; yoksa şansa bağlı araya girer
            let mut acik = d.sohbetler.contains_key(&kanal);
            let mut sans = SANS * gelisim::evre(&d.gelisim).sans;
            if seyahat::simdi().is_some() {
                sans *= seyahat::YOLDA_SANS_CARPANI;
            }
            if !acik && (etiketlendi || (girebilir_mi(&d, kanal) && rand::random::<f64>() < sans)) {
                sohbet_baslat(&mut d, kanal, None);
                acik = true;
            }
            if let Some(s) = d.sohbetler.get_mut(&kanal) {
                s.gecmis.push(kullanici(format!("{isim}: {metin}")));
                s.son_mesaj = Some(msg.id);
                s.etiketli = etiketlendi;
                s.gelen += 1;
                if s.gecmis.len() > SOHBET_BOYU {
                    s.gecmis.drain(..s.gecmis.len() - SOHBET_BOYU);
                }
            }
            kanal_not(&mut d, kanal, format!("{isim}: {metin}"));
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
    // sağlayıcı seçimi: SAGLAYICI=mistral zorlar; yoksa hangi anahtar varsa o, ikisi de varsa openrouter
    let saglayici = std::env::var("SAGLAYICI")
        .unwrap_or_default()
        .to_lowercase();
    let (api_adres, anahtar, varsayilan_model) = if saglayici == "mistral"
        || (ayar("OPENROUTER_KEY").is_err() && ayar("MISTRAL_KEY").is_ok())
    {
        (MISTRAL_ADRES, ayar("MISTRAL_KEY")?, MISTRAL_MODEL)
    } else {
        (OPENROUTER_ADRES, ayar("OPENROUTER_KEY")?, OPENROUTER_MODEL)
    };
    let model = ayar("MODEL").unwrap_or_else(|_| varsayilan_model.to_string());
    println!("sağlayıcı: {api_adres} · model: {model}");
    let haber_kanali = match std::env::var("HABER_KANALI") {
        Ok(v) if !v.trim().is_empty() => Some(ChannelId::new(v.trim().parse()?)),
        _ => None,
    };
    for k in ["kisiler", "konular", "olaylar", "arsiv", "kanallar"] {
        std::fs::create_dir_all(PathBuf::from(DURUM_KLASORU).join(k))?;
    }
    std::fs::create_dir_all(RESIM_KLASORU)?;

    let mut durum = Durum::yukle();
    uyku::guncelle(&mut durum);
    let secili = hafiza::oku("model.md");
    durum.model = if secili.trim().is_empty() {
        model
    } else {
        secili.trim().to_string()
    };
    println!("model: {}", durum.model);
    let bot = Arc::new(Bot {
        durum: Mutex::new(durum),
        http: reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()?,
        api_adres: api_adres.to_string(),
        anahtar,
        haber_kanali,
        firecrawl: std::env::var("FIRECRAWL_KEY")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn kisalt_iki_cumle() {
        let m = "tamam la. sen bilirsin. ama bak sonra ağlama. cidden.";
        assert_eq!(kisalt(m, 200, 2), "tamam la. sen bilirsin");
        assert_eq!(
            kisalt(m, 200, 3),
            "tamam la. sen bilirsin. ama bak sonra ağlama"
        );
        assert_eq!(
            kisalt("napıyım yavaş mı yazayım", 200, 2),
            "napıyım yavaş mı yazayım"
        );
    }

    #[test]
    fn cevap_ayiklanir() {
        assert_eq!(
            cevap_ayikla("DÜŞÜNCE: laf sokuyor, sert dön\nCEVAP: sen kendine bak olm"),
            "sen kendine bak olm"
        );
        assert_eq!(cevap_ayikla("düz mesaj"), "düz mesaj");
        assert_eq!(cevap_ayikla("DÜŞÜNCE: bir şey\nsonra düz"), "sonra düz");
    }

    #[test]
    fn kisalt_karakter() {
        let uzun = "bu cümle hiç bitmeyecek gibi devam ediyor ve virgüllerle uzuyor da uzuyor abi";
        let k = kisalt(uzun, 40, 2);
        assert!(k.chars().count() <= 40);
        assert!(!k.ends_with(' '));
        assert_eq!(k, "bu cümle hiç bitmeyecek gibi devam");
    }
}
