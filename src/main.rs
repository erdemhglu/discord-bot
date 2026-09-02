mod ajanlar;
mod gelisim;
mod gundem;
mod hafiza;
mod komut;
mod loglama;
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
const SOHBET_ZAMAN_ASIMI: Duration = Duration::from_secs(30 * 60); // bu kadar sessiz kalan sohbet kendiliğinden kapanır
const SANS: f64 = 0.35; // artık kullanılmıyor: yerini isteklilik değerlendirmesi aldı (yedek zar)
const ISTEK_ESIGI: u8 = 6; // isteklilik puanı bu eşiğin üstündeyse sohbete girer
const DEGERLENDIRME_ARALIGI: Duration = Duration::from_secs(2 * 60); // kanal başına en sık isteklilik çağrısı
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
const AKIS_DUZENLEME: Duration = Duration::from_millis(1200); // stream düzenlemeleri bundan sık olmaz (discord edit sınırı)
const DUSUNCE_DUGMESI: &str = "dusunce_goster"; // gizle kipinde cevap sonundaki düşünce butonunun kimliği

// sohbet cevabı token bütçesi: derleme durumuna göre değişir.
// release'de None → model sonuna kadar konuşur, bütçe yok.
// debug'da Some → geliştirme/test turunda token yakmasın diye küçük kapak.
macro_rules! cevap_butcesi {
    () => {
        if cfg!(debug_assertions) {
            Some(2000u32)
        } else {
            None
        }
    };
}
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
}

// düşünme gösterim kipi; !düşünme ile değişir, durum/dusunme.md'de kalır
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum DusunmeKip {
    #[default]
    Goster, // reasoning istenir, cevapla birlikte spoiler içinde gösterilir
    Gizle,  // reasoning istenir ama gösterilmez; düşünürken "Düşünüyorum..." yazar
    Kapali, // reasoning istenmez, istekler düşünmesiz atılır
}

impl DusunmeKip {
    fn dosya_degeri(self) -> &'static str {
        match self {
            DusunmeKip::Goster => "goster",
            DusunmeKip::Gizle => "gizle",
            DusunmeKip::Kapali => "kapali",
        }
    }

    fn oku() -> Self {
        match hafiza::oku("dusunme.md").trim() {
            "gizle" => DusunmeKip::Gizle,
            "kapali" => DusunmeKip::Kapali,
            _ => DusunmeKip::Goster,
        }
    }

    // komut argümanından kip; tanınmıyorsa None
    fn arg_ile(arg: &str) -> Option<Self> {
        match arg {
            "göster" | "goster" | "aç" | "ac" | "on" => Some(DusunmeKip::Goster),
            "gizle" => Some(DusunmeKip::Gizle),
            "kapat" | "kapalı" | "kapali" | "off" => Some(DusunmeKip::Kapali),
            _ => None,
        }
    }

    fn ad(self) -> &'static str {
        match self {
            DusunmeKip::Goster => "göster",
            DusunmeKip::Gizle => "gizli",
            DusunmeKip::Kapali => "kapalı",
        }
    }
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
    son_aktivite: HashMap<ChannelId, Instant>, // sohbetin son canlandığı an (zaman aşımı kapatır)
    son_degerlendirme: HashMap<ChannelId, Instant>, // kanal başına son isteklilik çağrısı (rate limit)
    haber_bekleyen: HashMap<ChannelId, Instant>,
    atilan_haberler: HashSet<u64>,
    taranan: HashSet<GuildId>,
    // gündem ve uyku
    gundem: String, // gezgin: son okudukları ve düşündükleri
    // zihin eşlemeleri: görünen ad (küçük harf) → id, id → kullanıcı adı
    ad_id: HashMap<String, u64>,
    kullanici_adlari: HashMap<u64, String>,
    // bellek döngüsünün işleyeceği kuyruk: (döküm, kaynak, kanal adı, eleştirmen de çalışsın mı)
    bellek_kuyruk: VecDeque<(String, String, String, bool)>,
    planlar: Vec<uyku::Plan>,
    uyuyor: bool,
    uyanik_zorla: i64, // !uyan sonrası bu ana kadar uyku planı işlemez (unix)
    kanal_gecmisi: HashMap<ChannelId, VecDeque<String>>, // kanal başına son satırlar, diskte de durur
    bekleyen_etiketler: Vec<(ChannelId, String)>,        // uyurken etiketlenmişse uyanınca döner
    gelisim: gelisim::Gelisim,                           // evre, sayaçlar, seçtiği isim
    kullanici_adi: String, // discord kullanıcı adı; bot_adi seçilen isim olabilir
    model: String,         // kullanılan model; !model ile değişir, durum/model.md'de kalır
    dusunme: DusunmeKip,   // düşünme kipi; !düşünme ile değişir, durum/dusunme.md'de kalır
    // gizle kipinde butonla gösterilmek üzere son cevapların düşüncesi (mesaj id → düşünce)
    dusunce_deposu: HashMap<MessageId, String>,
    dusunce_sirasi: VecDeque<MessageId>,
    metrik: Metrik,         // oturum boyu model kullanım toplamı
    son_yol_mesaji: i64,    // seyahatte en son hangi gün yoldan yazdı
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
            dusunme: DusunmeKip::oku(),
            ..Durum::default()
        }
    }

    // gizle kipinde butonun bulması için düşünceyi son cevabın mesajına bağlar;
    // depo sınırlı, eskiden başlayarak düşer
    fn dusunce_bagla(&mut self, mesaj: MessageId, dusunce: String) {
        self.dusunce_deposu.insert(mesaj, dusunce);
        self.dusunce_sirasi.push_back(mesaj);
        while self.dusunce_sirasi.len() > 50 {
            if let Some(eski) = self.dusunce_sirasi.pop_front() {
                self.dusunce_deposu.remove(&eski);
            }
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

    // model kullanımını oturum metriğine ekler
    fn metrik_ekle(&self, k: Kullanim) {
        let mut d = self.durum();
        d.metrik.cagri += 1;
        d.metrik.giris_token += k.prompt_tokens;
        d.metrik.cikis_token += k.completion_tokens;
        d.metrik.son_cagri_sn = simdi_unix();
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

// uzun metni en çok sinir karakterlik parçalara böler: önce cümle sınırı,
// sonra boşluk, o da yoksa tam sınırdan sert keser; hiçbir şey atılmaz
fn bol(metin: &str, sinir: usize) -> Vec<String> {
    let mut parcalar: Vec<String> = Vec::new();
    let mut kalan = metin.trim().to_string();
    while kalan.chars().count() > sinir {
        let kes = kesim_noktasi(&kalan, sinir);
        let bas: String = kalan.chars().take(kes).collect();
        kalan = kalan.chars().skip(kes).collect();
        let bas = bas.trim().to_string();
        if !bas.is_empty() {
            parcalar.push(bas);
        }
        kalan = kalan.trim_start().to_string();
    }
    if !kalan.is_empty() {
        parcalar.push(kalan);
    }
    parcalar
}

// ilk sinir karakter içindeki en iyi kesim yeri; çok ufak parça çıkmasın diye
// cümle/boşluk kesimi sınırın dörtte birinden sonra değilse sert kese düşer
fn kesim_noktasi(metin: &str, sinir: usize) -> usize {
    let mut cumle = 0;
    let mut bosluk = 0;
    for (i, c) in metin.chars().take(sinir).enumerate() {
        if matches!(c, '.' | '!' | '?' | '\n') {
            cumle = i + 1;
        } else if c == ' ' {
            bosluk = i;
        }
    }
    let asgari = sinir / 4;
    if cumle > asgari {
        cumle
    } else if bosluk > asgari {
        bosluk
    } else {
        sinir
    }
}

// discord spoiler'ı; içindeki dik çizgiler kaçırılır ki spoiler bozulmasın
fn spoiler(metin: &str) -> String {
    format!("||{}||", metin.replace('|', "\\|"))
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

// modelin başa ekleyebildiği ad öneki ve tırnakları soyar
fn soy(mut metin: String, bot_adi: &str) -> String {
    metin = metin.trim().to_string();
    // "isim: metin" kalıbını taklit edip başına kendi adını koyabiliyor
    let onek = format!("{bot_adi}:");
    if metin.to_lowercase().starts_with(&onek.to_lowercase()) {
        metin = metin[onek.len()..].trim().to_string();
    }
    if metin.len() > 1 && metin.starts_with('"') && metin.ends_with('"') {
        metin = metin[1..metin.len() - 1].to_string();
    }
    metin
}

// stream'siz yollar tek mesajla sınırlı: soy + 1900 kapak.
// stream yolu soy'dan sonra bol() ile bölerek gönderir, kırpma yok.
fn temizle(metin: String, bot_adi: &str) -> String {
    let mut metin = soy(metin, bot_adi);
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

// ```json ... ``` gibi süslerin içinden json'u çıkarır
fn json_ayikla(metin: &str) -> &str {
    match (metin.find('{'), metin.rfind('}')) {
        (Some(b), Some(s)) if s > b => &metin[b..=s],
        _ => metin,
    }
}

// isteklilik cevabından 0-10 puanı çözer; bozuksa None
fn isteklilik_puan(cevap: &str) -> Option<u8> {
    #[derive(Deserialize)]
    struct Deger {
        #[serde(default)]
        puan: i32,
    }
    let d: Deger = serde_json::from_str(json_ayikla(cevap)).ok()?;
    Some(d.puan.clamp(0, 10) as u8)
}

// ---------- yapay zeka ----------

#[derive(Deserialize)]
struct Yanit {
    choices: Vec<Secenek>,
    #[serde(default)]
    usage: Option<Kullanim>,
}
#[derive(Deserialize)]
struct Secenek {
    message: Icerik,
}
#[derive(Deserialize)]
struct Icerik {
    content: Option<String>,
}

// sağlayıcının döndürdüğü token sayacı; maliyet görünürlüğü için toplanır
#[derive(Deserialize, Default, Clone, Copy, Debug)]
struct Kullanim {
    #[serde(default)]
    prompt_tokens: u64,
    #[serde(default)]
    completion_tokens: u64,
}

impl Kullanim {
    fn topla(&mut self, diger: Kullanim) {
        self.prompt_tokens += diger.prompt_tokens;
        self.completion_tokens += diger.completion_tokens;
    }
}

// oturum boyu biriken model kullanım metriği; !durum bunu gösterir
#[derive(Default, Clone, Copy, Debug)]
struct Metrik {
    cagri: u32,
    giris_token: u64,
    cikis_token: u64,
    son_cagri_sn: i64,
}

// stream parçası: reasoning modellerde düşünce de gelir, düz modellerde yalnız content
#[derive(Default, Clone, PartialEq)]
struct Parca {
    metin: String,
    dusunce: String,
}

#[derive(Deserialize)]
struct AkisYaniti {
    #[serde(default)]
    choices: Vec<AkisSecenegi>,
    // include_usage ile son chunk'ta gelir; choices boş olabilir
    #[serde(default)]
    usage: Option<Kullanim>,
}
#[derive(Deserialize)]
struct AkisSecenegi {
    delta: AkisParcasi,
}
#[derive(Deserialize, Default)]
struct AkisParcasi {
    #[serde(default)]
    content: Option<String>,
    // openrouter "reasoning" der, openai uyumlu router'lar "reasoning_content"
    #[serde(default)]
    reasoning: Option<String>,
    #[serde(default)]
    reasoning_content: Option<String>,
}

// bir SSE satırından çıkanlar: içerik parçası ve/veya kullanım sayacı
#[derive(Default)]
struct SseVeri {
    parca: Option<Parca>,
    kullanim: Option<Kullanim>,
    done: bool,
}

// tek bir "data: ..." SSE satırını çözer; keepalive/bozuk satırlarda None
fn sse_ayikla(satir: &str) -> Option<SseVeri> {
    let veri = satir.trim().strip_prefix("data:")?.trim();
    if veri == "[DONE]" {
        return Some(SseVeri {
            done: true,
            ..Default::default()
        });
    }
    let yanit: AkisYaniti = serde_json::from_str(veri).ok()?;
    let kullanim = yanit.usage;
    let parca = yanit.choices.into_iter().next().and_then(|s| {
        let metin = s.delta.content.unwrap_or_default();
        let dusunce = [s.delta.reasoning, s.delta.reasoning_content]
            .into_iter()
            .flatten()
            .find(|s| !s.is_empty())
            .unwrap_or_default();
        (!metin.is_empty() || !dusunce.is_empty()).then_some(Parca { metin, dusunce })
    });
    (parca.is_some() || kullanim.is_some()).then_some(SseVeri {
        parca,
        kullanim,
        done: false,
    })
}

// stream isteğinin okuyucusu; her çağrıda sıradaki parçayı verir, akış bitince None
struct AkisOkuyucu {
    cevap: reqwest::Response,
    tampon: Vec<u8>,    // henüz satıra bölünmemiş baytlar
    kuyruk: Vec<Parca>, // çözülmüş, verilmeyi bekleyen parçalar
    kullanim: Kullanim, // son chunk'tan toplanan token sayacı
    done: bool,         // [DONE] görüldü mü (temiz kapanış işareti)
    bitti: bool,
}

impl AkisOkuyucu {
    async fn sonraki(&mut self) -> Result<Option<Parca>, Hata> {
        loop {
            if let Some(p) = self.kuyruk.pop() {
                return Ok(Some(p));
            }
            if self.bitti {
                if self.tampon.is_empty() {
                    return Ok(None);
                }
                // sonda satır sonu olmayan parça kalabilir
                let satir = String::from_utf8_lossy(&self.tampon).into_owned();
                self.tampon.clear();
                if let Some(v) = sse_ayikla(&satir) {
                    self.veri_uygula(&v);
                    if let Some(p) = v.parca {
                        return Ok(Some(p));
                    }
                }
                continue;
            }
            match self.cevap.chunk().await? {
                Some(p) => {
                    self.tampon.extend_from_slice(&p);
                    self.satirlari_isle();
                }
                None => self.bitti = true,
            }
        }
    }

    // done/usage yan etkilerini uygular; parça kuyruğa değil, döndürülür
    fn veri_uygula(&mut self, v: &SseVeri) {
        if v.done {
            self.done = true;
        }
        if let Some(k) = v.kullanim {
            self.kullanim.topla(k);
        }
    }

    // yalnız tam satırlar işlenir; eksik sondaki baytlar tamponda bekler
    // (utf-8 karakter chunk ortasında bölünse bile satır tamamlanınca çözülür)
    fn satirlari_isle(&mut self) {
        let mut sinir = 0;
        let mut veriler = Vec::new();
        for (i, b) in self.tampon.iter().enumerate() {
            if *b == b'\n' {
                let satir = String::from_utf8_lossy(&self.tampon[sinir..i]);
                if let Some(v) = sse_ayikla(&satir) {
                    veriler.push(v);
                }
                sinir = i + 1;
            }
        }
        self.tampon.drain(..sinir);
        let once = self.kuyruk.len();
        for v in veriler {
            self.veri_uygula(&v);
            if let Some(p) = v.parca {
                self.kuyruk.push(p);
            }
        }
        // yeni eklenenler geliş sırasında; pop ilk geleni versin diye ters çevrilir
        self.kuyruk[once..].reverse();
    }
}

// stream gönderiminin sonucu
enum AkisSonuc {
    Gonderildi(String), // son metin gönderildi
    Eski, // üretim sırasında yeni mesaj geldi; açılanlar silindi, güncel bağlamla yeniden üret
    Bos,  // akıştan kullanılır bir şey çıkmadı
}

// gonder_akis'in cevap bağlamı; argüman yığını yerine tek yapı
struct AkisBaglam<'a> {
    bot_adi: &'a str,
    yanit: Option<MessageId>,
    gelen: u32,
    gecmis: &'a [Mesaj],
    talimat: &'a str,
    butce: Option<u32>,
}

impl Bot {
    // düşünme kapalıysa modelin reasoning üretmesini istekte kapatır (token harcamasın);
    // openrouter "reasoning", qwen tarzı router'lar "enable_thinking" anlar
    fn reasoning_kapat(&self, govde: &mut serde_json::Value) {
        if self.durum().dusunme != DusunmeKip::Kapali {
            return;
        }
        let Some(o) = govde.as_object_mut() else {
            return;
        };
        o.insert("reasoning".into(), serde_json::json!({ "enabled": false }));
        o.insert("enable_thinking".into(), serde_json::json!(false));
    }

    // openrouter'a ham istek; her şey buradan geçer
    async fn sor_ham(&self, mut govde: serde_json::Value) -> Result<String, Hata> {
        self.reasoning_kapat(&mut govde);
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
        if let Some(k) = yanit.usage {
            self.metrik_ekle(k);
        }
        let metin = yanit
            .choices
            .into_iter()
            .next()
            .and_then(|s| s.message.content)
            .ok_or("modelden boş yanıt geldi")?;
        Ok(metin.trim().to_string())
    }

    async fn sor(&self, sistem: &str, gecmis: &[Mesaj], max_tokens: u32) -> Result<String, Hata> {
        self.sor_bolumlu(sistem, "", gecmis, Some(max_tokens)).await
    }

    // sistem mesajı iki blok: sabit blok cache_control ile işaretli (anthropic/gemini önbelleğe alır,
    // openai zaten öneki kendi önbellekler); değişken blok her seferinde yeniden okunur.
    // butce None ise max_tokens gitmez, model bütçesiz konuşur.
    async fn sor_bolumlu(
        &self,
        sabit: &str,
        degisken: &str,
        gecmis: &[Mesaj],
        butce: Option<u32>,
    ) -> Result<String, Hata> {
        let mut mesajlar = vec![sistem_json(sabit, degisken)];
        mesajlar.extend(
            gecmis
                .iter()
                .map(|m| serde_json::json!({ "role": m.role, "content": m.content })),
        );
        let mut govde = serde_json::json!({
            "model": self.durum().model.clone(),
            "messages": mesajlar,
            "temperature": 0.7,
        });
        if let Some(t) = butce {
            govde["max_tokens"] = serde_json::json!(t);
        }
        self.sor_ham(govde).await
    }

    // stream istek: hata kontrolü sor_ham ile aynı, gövdeye stream eklenir.
    // butce None ise max_tokens gitmez, model bütçesiz konuşur (release sohbet yolu).
    async fn sor_ham_akis(
        &self,
        sabit: &str,
        degisken: &str,
        gecmis: &[Mesaj],
        butce: Option<u32>,
    ) -> Result<AkisOkuyucu, Hata> {
        let mut mesajlar = vec![sistem_json(sabit, degisken)];
        mesajlar.extend(
            gecmis
                .iter()
                .map(|m| serde_json::json!({ "role": m.role, "content": m.content })),
        );
        let mut govde = serde_json::json!({
            "model": self.durum().model.clone(),
            "messages": mesajlar,
            "temperature": 0.7,
            "stream": true,
            // son chunk'ta usage gelsin (token sayacı)
            "stream_options": { "include_usage": true },
        });
        if let Some(t) = butce {
            govde["max_tokens"] = serde_json::json!(t);
        }
        self.reasoning_kapat(&mut govde);
        let cevap = self
            .http
            .post(&self.api_adres)
            .bearer_auth(&self.anahtar)
            .json(&govde)
            .send()
            .await?;
        let durum = cevap.status();
        if !durum.is_success() {
            let govde_metni = cevap.text().await?;
            let model = govde.get("model").and_then(|m| m.as_str()).unwrap_or("?");
            return Err(format!("{durum} (model: {model}): {}", kirp_hata(&govde_metni)).into());
        }
        Ok(AkisOkuyucu {
            cevap,
            tampon: Vec::new(),
            kuyruk: Vec::new(),
            kullanim: Kullanim::default(),
            done: false,
            bitti: false,
        })
    }

    // sohbet cevabının sistem mesajı: kim konuşuyor, ne konuşuluyor bakıp
    // hafızadan yalnız ilgili parçaları getirir; uret ve uret_akis ortak kullanır
    fn sohbet_sistemi(&self, gecmis: &[Mesaj], talimat: &str) -> (String, String, String) {
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
        let d = self.durum();
        let getirilen = hafiza::getir(&katilimcilar, &d.ad_id, &anahtar, &d.hafiza, SOHBET_BOYU);
        let (sabit, degisken) = sistem_metni(&d, talimat, &getirilen);
        (sabit, degisken, d.bot_adi.clone())
    }

    // kişilikle konuşur: sohbet, hoş geldin, laf atma, haber tanıtma, şakalar.
    // butce None ise max_tokens gitmez; sohbet cevapları bunu cevap_butcesi! ile belirler.
    async fn uret(
        &self,
        gecmis: &[Mesaj],
        talimat: &str,
        butce: Option<u32>,
    ) -> Result<String, Hata> {
        let (sabit, degisken, bot_adi) = self.sohbet_sistemi(gecmis, talimat);
        let cevap = self.sor_bolumlu(&sabit, &degisken, gecmis, butce).await?;
        Ok(temizle(cevap, &bot_adi))
    }

    // sohbet cevabını akış olarak açar; parçalar geldikçe okuyucudan okunur
    async fn uret_akis(
        &self,
        gecmis: &[Mesaj],
        talimat: &str,
        butce: Option<u32>,
    ) -> Result<(AkisOkuyucu, String), Hata> {
        let (sabit, degisken, bot_adi) = self.sohbet_sistemi(gecmis, talimat);
        let okuyucu = self.sor_ham_akis(&sabit, &degisken, gecmis, butce).await?;
        Ok((okuyucu, bot_adi))
    }

    // kişiliksiz, düz analiz: ajanlar bunu kullanır
    async fn analiz(&self, metin: &str, talimat: &str, max_tokens: u32) -> Result<String, Hata> {
        let girdi = kullanici(format!("{metin}\n\n---\n\n{talimat}"));
        self.sor(ANALIST, &[girdi], max_tokens).await
    }

    // "bu konuşmaya katılmak istiyor muyum?" mini değerlendirmesi (0-10 puan).
    // etiket/yanıt her zaman cevaplanır, buraya hiç gelmez; hata durumunda None (yedek zar devrede)
    async fn isteklilik(&self) -> Option<u8> {
        let (baglam, profil, dizin, bot_adi) = {
            let d = self.durum();
            (
                son_mesajlar(&d, 12),
                d.profil.clone(),
                d.dizin.clone(),
                d.bot_adi.clone(),
            )
        };
        if baglam.trim().is_empty() {
            return None;
        }
        let talimat = ISTEKLILIK.replace("{ad}", &bot_adi);
        let girdi =
            format!("GRUP PROFİLİ\n{profil}\n\nKİŞİ DİZİNİ\n{dizin}\n\nSON MESAJLAR\n{baglam}");
        match self.analiz(&girdi, &talimat, 80).await {
            Ok(c) => isteklilik_puan(&c),
            Err(e) => {
                log::debug!("isteklilik: çağrı başarısız: {e}");
                None
            }
        }
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
                Err(e) => log::warn!("görsel okunamadı ({}): {e}", yol.display()),
            }
        }
        if let Err(e) = kanal.send_message(&ctx.http, mesaj).await {
            log::error!("gönderilemedi ({kanal}): {e}");
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

    // sohbet cevabını akışla gönderir: mesaj erken belirir, aralıklarla düzenlenir.
    // thinking kırpılmadan spoiler bloklarında durur; 1900'ü aşan cevap yeni mesaja bölünür.
    // yanıt bağı yalnız ilk mesajda, mention yalnız onunla gider.
    async fn gonder_akis(
        &self,
        ctx: &Context,
        kanal: ChannelId,
        mut okuyucu: AkisOkuyucu,
        baglam: AkisBaglam<'_>,
    ) -> Result<AkisSonuc, Hata> {
        let mut metin = String::new();
        let mut dusunce = String::new();
        let mut gonderilenler: Vec<Message> = Vec::new();
        let mut son_yazma = Instant::now();
        let mut ilk = true;
        let mut akis_hatasi: Option<Hata> = None;
        // kip cevap boyunca sabit kalır; stream ortasında değişirse bir sonraki cevapta geçer
        let kip = self.durum().dusunme;
        let baslangic = Instant::now();
        let mut parca_sayisi: u32 = 0;
        let mut ilk_parca_ms: Option<u128> = None;

        loop {
            match okuyucu.sonraki().await {
                Ok(Some(p)) => {
                    parca_sayisi += 1;
                    if ilk_parca_ms.is_none() {
                        ilk_parca_ms = Some(baslangic.elapsed().as_millis());
                    }
                    metin.push_str(&p.metin);
                    if kip != DusunmeKip::Kapali {
                        dusunce.push_str(&p.dusunce);
                    }
                    if ilk || son_yazma.elapsed() >= AKIS_DUZENLEME {
                        ilk = false;
                        let yerlesim =
                            akis_gorunum(kip, &dusunce, &soy(metin.clone(), baglam.bot_adi));
                        if !yerlesim.is_empty() {
                            yaz_akis(ctx, kanal, &mut gonderilenler, &yerlesim, baglam.yanit).await;
                            son_yazma = Instant::now();
                        }
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    akis_hatasi = Some(e);
                    break;
                }
            }
        }

        // kullanım metriği ve akış özeti
        self.metrik_ekle(okuyucu.kullanim);
        log::debug!(
            "akis [{kanal}]: parça={parca_sayisi} ilk={ilk_parca_ms:?}ms toplam={}ms done={}",
            baslangic.elapsed().as_millis(),
            okuyucu.done,
        );
        if !okuyucu.done && akis_hatasi.is_none() {
            log::warn!("akis [{kanal}]: [DONE] gelmeden kapandı, yarım kalmış olabilir");
        }

        // üretim sırasında yeni mesaj geldiyse eski hedefe cevap yollama
        if yeni_mesaj_var(&self.durum(), kanal, baglam.gelen) {
            sil_mesajlar(ctx, gonderilenler).await;
            return Ok(AkisSonuc::Eski);
        }

        let mut cevap = soy(metin, baglam.bot_adi);
        // boş ya da önceki mesajın kırıntısı ("'cım" gibi) gitmesin
        if cevap.chars().count() < 3 || cevap.starts_with('\'') {
            sil_mesajlar(ctx, gonderilenler).await;
            return match akis_hatasi {
                Some(e) => Err(e),
                None => Ok(AkisSonuc::Bos),
            };
        }
        // aynı lafı iki kez etmesin: bir kez yeniden üret, yine aynıysa susar
        if self.tekrar_mi(kanal, &cevap) {
            let t2 = format!("{}\n\nAz önce aynen şunu yazdın: \"{cevap}\". Aynısını ya da benzerini yazma; başka bir açıdan gir ya da konuyu değiştir.", baglam.talimat);
            match self.uret(baglam.gecmis, &t2, baglam.butce).await {
                Ok(y) if !self.tekrar_mi(kanal, &y) && y.chars().count() >= 3 => cevap = y,
                _ => {
                    sil_mesajlar(ctx, gonderilenler).await;
                    return Ok(AkisSonuc::Bos);
                }
            }
        }
        let yerlesim = akis_gorunum(kip, &dusunce, &cevap);
        yaz_akis(ctx, kanal, &mut gonderilenler, &yerlesim, baglam.yanit).await;

        // gizlede düşünce mesajda görünmez; cevap sonuna buton konur, tıklayana
        // ephemeral kod bloğu olarak açılır (interaction_create bakar)
        if kip == DusunmeKip::Gizle {
            let dusunce_tek = tek_satir(&dusunce);
            if !dusunce_tek.is_empty() {
                if let Some(son) = gonderilenler.last_mut() {
                    self.durum().dusunce_bagla(son.id, dusunce_tek);
                    let dugme = CreateButton::new(DUSUNCE_DUGMESI)
                        .label("Düşünce Sürecini Göster")
                        .style(ButtonStyle::Secondary);
                    if let Err(e) = son
                        .edit(
                            &ctx.http,
                            EditMessage::new()
                                .components(vec![CreateActionRow::Buttons(vec![dugme])]),
                        )
                        .await
                    {
                        log::warn!("düşünce butonu eklenemedi ({kanal}): {e}");
                    }
                }
            }
        }

        // gönderilenler kayda geçer; thinking kayda girmez, hoca ve eleştirmen yalnız cevabı görür
        let mut d = self.durum();
        d.kendi_mesajlarim.push_back(cevap.clone());
        if d.kendi_mesajlarim.len() > 50 {
            d.kendi_mesajlarim.pop_front();
        }
        let satir = format!("{}: {}", d.bot_adi, cevap);
        kanal_not(&mut d, kanal, satir);
        drop(d);
        if let Some(e) = akis_hatasi {
            log::warn!("akis yarıda kesildi, elimizdeki gönderildi: {e}");
        }
        Ok(AkisSonuc::Gonderildi(cevap))
    }
}

// thinking fazı: cevap henüz başlamadıysa ve model düşünüyorsa placeholder gider;
// gizlede canlı kelime sayacı, göstergede düz "Düşünüyorum...". Cevap başlayınca
// aynı mesaj düzenlenerek stream edilir
fn akis_gorunum(kip: DusunmeKip, dusunce: &str, cevap: &str) -> Vec<String> {
    if cevap.trim().is_empty() && !dusunce.trim().is_empty() {
        return match kip {
            DusunmeKip::Gizle => vec![dusunce_sayaci(dusunce)],
            DusunmeKip::Goster => vec!["Düşünüyorum...".to_string()],
            DusunmeKip::Kapali => Vec::new(),
        };
    }
    let dusunce = tek_satir(dusunce);
    let mut v: Vec<String> = Vec::new();
    if kip == DusunmeKip::Goster && !dusunce.is_empty() {
        // göster: hem spoiler hem kod bloğu
        for p in bol(&dusunce, MESAJ_SINIRI - 4) {
            v.push(spoiler(&p));
        }
        v.extend(kod_bloklari(&dusunce));
    }
    // gizle ve kapalı kiplerde düşünce yerleşime girmez; gizlede cevap sonunda
    // "Düşünce Sürecini Göster" butonu gider (gonder_akis ekler)
    v.extend(bol(cevap, MESAJ_SINIRI));
    v
}

// gizlede düşünürken canlı sayaç: kaçıncı kelimede olduğu görünür
fn dusunce_sayaci(dusunce: &str) -> String {
    let n = dusunce.split_whitespace().count();
    format!("Düşünüyorum... Şu ana kadar {n} kelime düşündüm.")
}

// thinking'in kod bloğu biçimi; 1900'ü aşarsa birden çok blok
fn kod_bloklari(metin: &str) -> Vec<String> {
    bol(metin, MESAJ_SINIRI - 10)
        .into_iter()
        .map(|p| format!("```\n{p}\n```"))
        .collect()
}

// butonla açılan ephemeral düşünce: tek mesaja sığacak şekilde kod bloğu
fn dusunce_gosterim(metin: &str) -> String {
    let not = "\n_(düşünce uzun, kısaltıldı)_";
    let sinir = MESAJ_SINIRI - 12 - not.chars().count();
    let toplam = metin.chars().count();
    let govde: String = metin.chars().take(sinir).collect();
    let mut s = format!("```\n{govde}\n```");
    if toplam > sinir {
        s.push_str(not);
    }
    s
}

// thinking'de her düşünce için newline atılmasın; tek akıcı satıra indirgenir
fn tek_satir(metin: &str) -> String {
    metin.split_whitespace().collect::<Vec<_>>().join(" ")
}

// yerleşimi açık mesajlarla uzlaştırır: değişenler düzenlenir, eksikler açılır,
// metin kısalırsa (ad öneki soyulması gibi) fazla mesajlar silinir
async fn yaz_akis(
    ctx: &Context,
    kanal: ChannelId,
    gonderilenler: &mut Vec<Message>,
    yerlesim: &[String],
    yanit: Option<MessageId>,
) {
    let _ = kanal.broadcast_typing(&ctx.http).await;
    for (i, icerik) in yerlesim.iter().enumerate() {
        match gonderilenler.get_mut(i) {
            Some(m) if m.content != *icerik => {
                if let Err(e) = m
                    .edit(&ctx.http, EditMessage::new().content(icerik.clone()))
                    .await
                {
                    log::warn!("düzenlenemedi ({kanal}): {e}");
                }
            }
            Some(_) => {}
            None => {
                let mut izin = CreateAllowedMentions::new();
                let mut mesaj = CreateMessage::new().content(icerik);
                if i == 0 {
                    if let Some(id) = yanit {
                        izin = izin.replied_user(true);
                        mesaj = mesaj.reference_message((kanal, id));
                    }
                }
                match kanal
                    .send_message(&ctx.http, mesaj.allowed_mentions(izin))
                    .await
                {
                    Ok(m) => gonderilenler.push(m),
                    Err(e) => {
                        log::error!("gönderilemedi ({kanal}): {e}");
                        break;
                    }
                }
            }
        }
    }
    while gonderilenler.len() > yerlesim.len() {
        if let Some(m) = gonderilenler.pop() {
            let _ = m.delete(&ctx.http).await;
        }
    }
}

async fn sil_mesajlar(ctx: &Context, mesajlar: Vec<Message>) {
    for m in mesajlar {
        let _ = m.delete(&ctx.http).await;
    }
}

// sistem mesajını OpenAI uyumlu bloğa çevirir: değişken boşsa düz metin,
// değilse sabit blok cache_control ile işaretli iki metin bloğu
fn sistem_json(sabit: &str, degisken: &str) -> serde_json::Value {
    if degisken.is_empty() {
        serde_json::json!({ "role": "system", "content": sabit })
    } else {
        serde_json::json!({ "role": "system", "content": [
            { "type": "text", "text": sabit, "cache_control": { "type": "ephemeral" } },
            { "type": "text", "text": degisken }
        ]})
    }
}

// her cevabın sistem mesajı iki parça: SABİT (kişilik, huy, profil, dizin...; ajan çalışınca
// değişir, prompt cache buradan kazanır) ve DEĞİŞKEN (getirilenler, saat, görev)
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
    d.son_aktivite.insert(kanal, Instant::now());
    d.sohbetler.entry(kanal).or_insert(s)
}

fn sohbet_bitir(d: &mut Durum, kanal: ChannelId) -> Option<Sohbet> {
    d.haber_bekleyen.remove(&kanal);
    d.sohbetler.remove(&kanal)
}

fn yeni_mesaj_var(d: &Durum, kanal: ChannelId, uretilen_gelen: u32) -> bool {
    d.sohbetler
        .get(&kanal)
        .is_some_and(|s| s.gelen > uretilen_gelen)
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
                } else {
                    ""
                };
                d.mesgul.insert(kanal);
                talimat
            };

            // Kısa bir okuma payı bırak; peş peşe yazılanları tek bağlamda gör.
            sleep(Duration::from_millis(150 + (rand::random::<u64>() % 200))).await;
            let (gecmis, yanit, gelen, son_metin) = {
                let d = self.durum();
                let Some(s) = d.sohbetler.get(&kanal) else {
                    drop(d);
                    self.durum().mesgul.remove(&kanal);
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
                (s.gecmis.clone(), s.son_mesaj, s.gelen, son_metin)
            };
            log::debug!(
                "cevapla [{kanal}]: tur başı, geçmiş {} satır, gelen={gelen}",
                gecmis.len()
            );
            // istendiyse internete bak (haber, araştır, link) ve bulduklarını göreve ekle
            let mut talimat = talimat.to_string();
            if let Some(bulgu) = self.arastir(&son_metin).await {
                talimat = format!(
                    "{talimat}\n\nİNTERNETTEN ŞİMDİ ÇEKTİKLERİN (istendiği için baktın; kendi ağzınla anlat, liste yapma, \"kaynak\" deme):\n{bulgu}"
                );
            }
            // Model çağrısı sürerken yazıyor göstergesi görünsün; stream mesajı ilk delta ile açılır.
            let _ = kanal.broadcast_typing(&ctx.http).await;
            let butce = cevap_butcesi!();
            let (okuyucu, bot_adi) = match self.uret_akis(&gecmis, &talimat, butce).await {
                Ok(x) => x,
                Err(e) => {
                    log::error!("ai hatası: {e}");
                    self.durum().mesgul.remove(&kanal);
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
                        gelen,
                        gecmis: &gecmis,
                        talimat: &talimat,
                        butce,
                    },
                )
                .await
            {
                Ok(AkisSonuc::Gonderildi(c)) => c,
                Ok(AkisSonuc::Eski) => {
                    self.durum().mesgul.remove(&kanal);
                    continue;
                }
                Ok(AkisSonuc::Bos) => {
                    // akıştan kullanılır bir şey çıkmadı; bir kez stream'siz dene
                    if yeni_mesaj_var(&self.durum(), kanal, gelen) {
                        self.durum().mesgul.remove(&kanal);
                        continue;
                    }
                    match self.uret(&gecmis, &talimat, butce).await {
                        Ok(c)
                            if c.chars().count() >= 3
                                && !c.starts_with('\'')
                                && !self.tekrar_mi(kanal, &c) =>
                        {
                            self.gonder(ctx, kanal, &c, None, None, yanit).await;
                            c
                        }
                        Ok(_) => {
                            self.durum().mesgul.remove(&kanal);
                            return;
                        }
                        Err(e) => {
                            log::error!("ai hatası: {e}");
                            self.durum().mesgul.remove(&kanal);
                            return;
                        }
                    }
                }
                Err(e) => {
                    log::error!("ai hatası: {e}");
                    self.durum().mesgul.remove(&kanal);
                    return;
                }
            };

            {
                let mut d = self.durum();
                d.mesgul.remove(&kanal);
                if let Some(s) = d.sohbetler.get_mut(&kanal) {
                    s.gecmis.push(asistan(cevap));
                    s.sayac += 1;
                    s.hackli = s.hackli.saturating_sub(1);
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

impl Bot {
    // sessiz kalan sohbetleri kapatır: veda mesajı yok, kanal yasağı yok.
    // kapanan sohbetin dökümü günlükçüye ve eleştirmene gider (bellek adımında kuyruğa taşınacak)
    async fn zaman_asimi_kapat(&self, ctx: &Context) {
        let kapananlar: Vec<(ChannelId, Sohbet)> = {
            let mut d = self.durum();
            // sohbeti kalmayan aktivite kayıtlarını temizle
            let aciklar: HashSet<ChannelId> = d.sohbetler.keys().copied().collect();
            d.son_aktivite.retain(|kanal, _| aciklar.contains(kanal));
            let simdi = Instant::now();
            let kapanacak: Vec<ChannelId> = d
                .son_aktivite
                .iter()
                .filter(|(k, t)| {
                    !d.mesgul.contains(k) && simdi.duration_since(**t) >= SOHBET_ZAMAN_ASIMI
                })
                .map(|(k, _)| *k)
                .collect();
            let mut kapananlar = Vec::new();
            for kanal in kapanacak {
                if let Some(s) = sohbet_bitir(&mut d, kanal) {
                    d.son_aktivite.remove(&kanal);
                    kapananlar.push((kanal, s));
                }
            }
            kapananlar
        };
        for (kanal, s) in kapananlar {
            let bot_adi = self.durum().bot_adi.clone();
            let dokum_metni = dokum(&s.gecmis, &bot_adi);
            let kanal_adi = kanal.name(ctx).await.unwrap_or_else(|_| kanal.to_string());
            log::debug!("sohbet [{kanal}]: zaman aşımıyla sessizce kapandı");
            // ajanlar inline değil, bellek döngüsünde işlenir (elestirmen de çalışsın)
            self.durum().bellek_kuyruk.push_back((
                dokum_metni,
                "biten sohbet".to_string(),
                kanal_adi,
                true,
            ));
            self.durum().gelisim.sohbet += 1;
            self.gelisim_kontrol(ctx).await;
        }
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
                log::info!("gelisim: {} evresine geçti", gelisim::evre(&d.gelisim).ad);
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
            .uret(&[kullanici("isim seçme vakti")], ISIM_SEC, Some(12))
            .await
        {
            Ok(c) => c,
            Err(e) => return log::error!("isim: {e}"),
        };
        let Some(isim) = gelisim::isim_temizle(&cevap) else {
            return log::warn!("isim: seçim çözülemedi: {cevap}");
        };
        for gid in ctx.cache.guilds() {
            if let Err(e) = gid.edit_nickname(&ctx.http, Some(&isim)).await {
                log::warn!("isim: takma ad değiştirilemedi ({gid}): {e}");
            }
        }
        {
            let mut d = self.durum();
            d.gelisim.isim = Some(isim.clone());
            d.bot_adi = isim.clone();
            gelisim::kaydet(&d.gelisim);
        }
        log::info!("gelisim: yeni isim {isim}");

        let Some(kanal) = varsayilan_kanal(self, ctx) else {
            return;
        };
        match self
            .uret(
                &[kullanici("ismini seçtin")],
                &ISIM_DUYURU.replace("{isim}", &isim),
                Some(150),
            )
            .await
        {
            Ok(duyuru) => {
                self.gonder(ctx, kanal, &duyuru, None, None, None).await;
                sohbet_baslat(&mut self.durum(), kanal, Some(duyuru));
            }
            Err(e) => log::error!("isim: {e}"),
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
        d.ad_id.entry(isim.to_lowercase()).or_insert(*id);
        hatirla(&mut d, isim, metin);
    }
    log::debug!("{}: {} mesaj okundu", guild.name, toplam.len());
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
        // gözlem de kuyruktan işlenir (elestirmen gerekmez)
        bot.durum().bellek_kuyruk.push_back((
            son,
            "6 saatlik gözlem, bot konuşmamış olabilir".to_string(),
            "gozlem".to_string(),
            false,
        ));
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
        match self.uret(&[kullanici(son)], SORUN, Some(160)).await {
            Ok(laf) => {
                self.gonder(ctx, kanal, &laf, None, None, None).await;
                sohbet_baslat(&mut self.durum(), kanal, Some(laf));
            }
            Err(e) => log::error!("ai hatası: {e}"),
        }
    }

    // seçilmiş bir haberi kanala atar ve yorum bekleme sohbeti açar
    async fn haber_at(&self, ctx: &Context, kanal: ChannelId) -> bool {
        let h = match self.haberci().await {
            Ok(h) => h,
            Err(e) => {
                log::warn!("haberci: {e}");
                return false;
            }
        };
        let link = if h.url.starts_with("https://") || h.url.starts_with("http://") {
            h.url.clone()
        } else {
            format!("https://news.ycombinator.com/item?id={}", h.id)
        };
        let girdi = match self
            .uret(&[kullanici(h.title.clone())], HABER_TANIT, Some(200))
            .await
        {
            Ok(g) => g,
            Err(e) => {
                log::error!("ai hatası: {e}");
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
            self.uret(&[kullanici("şaka başlıyor")], HACK_GIRIS, Some(150))
                .await
        } else {
            self.resimci(&resim).await
        };
        let metin = match metin {
            Ok(m) => m,
            Err(e) => {
                log::error!("ai hatası: {e}");
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
    if d.sohbetler.contains_key(&k) || d.profil.is_empty() {
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

        let laf = match bot.uret(&[kullanici(son)], talimat, Some(120)).await {
            Ok(l) => l,
            Err(e) => {
                log::error!("ai hatası: {e}");
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

// 10 dakikada bir: kapanan sohbetlerin ve gözlemlerin kuyruğunu zihne işler.
// uyku kontrolüne takılmaz; gece birikenler de sabaha kalmadan kaydedilir
async fn bellek_dongusu(bot: Arc<Bot>) {
    loop {
        sleep(Duration::from_secs(10 * 60)).await;
        loop {
            let isi = {
                let mut d = bot.durum();
                if d.bellek_kuyruk.len() > 50 {
                    log::warn!(
                        "bellek: kuyruk şişti ({}), en eski atılıyor",
                        d.bellek_kuyruk.len()
                    );
                    d.bellek_kuyruk.pop_front();
                }
                d.bellek_kuyruk.pop_front()
            };
            let Some((dokum_metni, kaynak, kanal_adi, elestir)) = isi else {
                break;
            };
            let dokum_kopya = dokum_metni.clone();
            bot.gunlukcu(dokum_metni, &kaynak, &kanal_adi).await;
            if elestir {
                bot.elestirmen(dokum_kopya).await;
            }
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
        bot.zaman_asimi_kapat(&ctx).await;
    }
}

impl Bot {
    // uyudu/uyandı geçişini işler; uyanınca uyurken gelen etiketlere döner
    async fn uyku_gecisi(&self, ctx: &Context) {
        let bekleyen = {
            let mut d = self.durum();
            let uyanik = uyku::uyanik_mi(&d);
            if uyanik == d.uyuyor {
                log::info!("uyku: {}", if uyanik { "uyandı" } else { "uyudu" });
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
                Some(200),
            )
            .await
        {
            Ok(c) => {
                self.gonder(ctx, kanal, &c, None, None, None).await;
                sohbet_baslat(&mut self.durum(), kanal, Some(c));
            }
            Err(e) => log::error!("ai hatası: {e}"),
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

        // ready yeniden bağlanınca tekrar gelir, döngüler bir kere başlasın
        if !self.baslatildi.swap(true, Ordering::SeqCst) {
            tokio::spawn(haber_dongusu(self.bot.clone(), ctx.clone()));
            tokio::spawn(durtme_dongusu(self.bot.clone(), ctx.clone()));
            tokio::spawn(saka_dongusu(self.bot.clone(), ctx.clone()));
            tokio::spawn(gezgin_dongusu(self.bot.clone()));
            tokio::spawn(bellek_dongusu(self.bot.clone()));
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
            )
            .await
        {
            Ok(s) => s,
            Err(e) => {
                log::error!("ai hatası: {e}");
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

        // 1. faz (kilit): kayıtlar + bayrak kararları
        let (acik, etiketlendi, degerlendir) = {
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
            // sohbet açık ya da etiket varsa değerlendirmeye gerek yok
            let degerlendir = if !acik && !etiketlendi {
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
            (acik, etiketlendi, degerlendir)
        };

        // 2. faz (kilitsiz): isteklilik değerlendirmesi — her mesaja atlamaz,
        // konu/personalık/ilgi tartılır; etiket ve açık sohbet zaten doğrudan cevaplanır
        let mut katil = acik || etiketlendi;
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
                Some(puan) => {
                    log::debug!("isteklilik [{kanal}]: puan={puan} eşik={esik}");
                    katil = i32::from(puan) >= esik;
                }
                None => {
                    // çağrı başarısız: eski yedek zar
                    katil = rand::random::<f64>() < SANS;
                    log::debug!("isteklilik [{kanal}]: çağrı yok, yedek zar={katil}");
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
                s.gecmis.push(kullanici(format!("{isim}: {metin}")));
                s.son_mesaj = Some(msg.id);
                s.gelen += 1;
                if s.gecmis.len() > SOHBET_BOYU {
                    s.gecmis.drain(..s.gecmis.len() - SOHBET_BOYU);
                }
                d.son_aktivite.insert(kanal, Instant::now());
            }
            kanal_not(&mut d, kanal, format!("{isim}: {metin}"));
            acik
        };

        if cevap_ver {
            self.bot.cevapla(&ctx, kanal).await;
        }
    }

    // gizle kipindeki "Düşünce Sürecini Göster" butonu: düşünenin deposundan
    // bulur, yalnız tıklayana görünen ephemeral kod bloğu olarak açar
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let Interaction::Component(c) = interaction else {
            return;
        };
        if c.data.custom_id != DUSUNCE_DUGMESI {
            return;
        }
        let dusunce = self.bot.durum().dusunce_deposu.get(&c.message.id).cloned();
        let Some(dusunce) = dusunce else {
            let _ = c
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .ephemeral(true)
                            .content("düşünce bulunamadı (bot yeniden başlamış olabilir)"),
                    ),
                )
                .await;
            return;
        };
        let icerik = dusunce_gosterim(&dusunce);
        if let Err(e) = c
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .ephemeral(true)
                        .content(icerik),
                ),
            )
            .await
        {
            log::warn!("düşünce ephemeral yanıtı gönderilemedi: {e}");
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
    loglama::kur();
    // panikler log'a backtrace ile düşsün; spawn'lu döngülerde sessiz ölüm kalmasın
    std::panic::set_hook(Box::new(|bilgi| {
        let iz = std::backtrace::Backtrace::force_capture();
        log::error!("PANİK: {bilgi}\n{iz}");
    }));
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
    // API_ADRES varsa sağlayıcının varsayılan adresini ezer: openai uyumlu
    // kendi router'ına (ör. yerel ağ) yönlendirmek için
    let api_adres = match std::env::var("API_ADRES") {
        Ok(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => api_adres.to_string(),
    };
    log::info!("sağlayıcı: {api_adres} · model: {model}");
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
    log::info!("model: {}", durum.model);
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
        log::info!("kapanıyor");
        yonetici.shutdown_all().await;
    });

    client.start().await?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn yeni_mesaj_eski_cevabi_gecersiz_kilar() {
        let kanal = ChannelId::new(7);
        let mut d = Durum::default();
        d.sohbetler.insert(
            kanal,
            Sohbet {
                gelen: 3,
                ..Sohbet::default()
            },
        );
        assert!(yeni_mesaj_var(&d, kanal, 2));
        assert!(!yeni_mesaj_var(&d, kanal, 3));
    }

    #[test]
    fn bol_cumle_siniri() {
        let m = "birinci cümle burada. ikinci cümle şurada. üçüncüsü de ötede.";
        let p = bol(m, 30);
        assert!(p.len() >= 2);
        for parca in &p {
            assert!(parca.chars().count() <= 30);
        }
        let birlesik: String = p.join(" ");
        assert_eq!(birlesik.replace(' ', ""), m.replace(' ', ""));
    }

    #[test]
    fn bol_bosluga_duser() {
        // noktalama yoksa boşlukta keser
        let m = "aaaa bbbb cccc dddd eeee";
        let p = bol(m, 12);
        assert_eq!(p, vec!["aaaa bbbb", "cccc dddd", "eeee"]);
    }

    #[test]
    fn bol_sert_keser() {
        // hiç boşluk yoksa tam sınırdan keser, hiçbir şey atmaz
        let m = "a".repeat(50);
        let p = bol(&m, 20);
        assert_eq!(p.len(), 3);
        assert_eq!(p.iter().map(|s| s.chars().count()).sum::<usize>(), 50);
    }

    #[test]
    fn kisaysa_degmez() {
        assert_eq!(bol("kısa", 100), vec!["kısa"]);
        assert_eq!(bol("  ", 100), Vec::<String>::new());
    }

    #[test]
    fn spoiler_kacar() {
        assert_eq!(spoiler("düşünce"), "||düşünce||");
        assert_eq!(spoiler("a|b"), "||a\\|b||");
    }

    #[test]
    fn sse_ayristirilir() {
        let v = sse_ayikla(r#"data: {"choices":[{"delta":{"content":"sel","reasoning":"düş"}}]}"#)
            .unwrap();
        let p = v.parca.unwrap();
        assert_eq!(p.metin, "sel");
        assert_eq!(p.dusunce, "düş");
        assert!(v.kullanim.is_none());
        // reasoning yoksa (mistral tarzı) content yine gelir
        let p = sse_ayikla(r#"data: {"choices":[{"delta":{"content":"merhaba"}}]}"#)
            .unwrap()
            .parca
            .unwrap();
        assert_eq!(p.metin, "merhaba");
        assert!(p.dusunce.is_empty());
        // openai uyumlu router'lar reasoning_content kullanır
        let p = sse_ayikla(
            r#"data: {"choices":[{"delta":{"content":"","reasoning_content":"qwen düşüncesi"}}]}"#,
        )
        .unwrap()
        .parca
        .unwrap();
        assert_eq!(p.dusunce, "qwen düşüncesi");
        assert!(p.metin.is_empty());
        // [DONE] işareti ayrı yakalanır
        let v = sse_ayikla("data: [DONE]").unwrap();
        assert!(v.done && v.parca.is_none());
        // usage son chunk'ta choices boşken gelir
        let v = sse_ayikla(
            r#"data: {"choices":[],"usage":{"prompt_tokens":7,"completion_tokens":13}}"#,
        )
        .unwrap();
        assert!(v.parca.is_none());
        assert_eq!(v.kullanim.unwrap().prompt_tokens, 7);
        assert_eq!(v.kullanim.unwrap().completion_tokens, 13);
        assert!(sse_ayikla(": keepalive").is_none());
        assert!(sse_ayikla("data: bozuk json").is_none());
        assert!(sse_ayikla(r#"data: {"choices":[{"delta":{}}]}"#).is_none());
    }

    #[test]
    fn gorunum_bolunur() {
        let dusunce = "düşün ".repeat(700); // ~4200 karakter, birden çok blok ister
        let cevap = "kelime ".repeat(400); // ~2800 karakter, birden çok mesaj ister
        let v = akis_gorunum(DusunmeKip::Goster, &dusunce, &cevap);
        assert!(v.len() >= 5);
        for (i, m) in v.iter().enumerate() {
            assert!(m.chars().count() <= MESAJ_SINIRI, "parça {i} çok uzun");
        }
        // önce spoiler blokları, sonra kod blokları, en sonda cevap parçaları
        assert!(v[0].starts_with("||") && v[0].ends_with("||"));
        assert!(v.iter().any(|m| m.starts_with("```")));
        assert!(!v[v.len() - 1].starts_with("||"));
        // gizle: düşünce yerleşime hiç girmez
        let v = akis_gorunum(DusunmeKip::Gizle, &dusunce, &cevap);
        assert!(v
            .iter()
            .all(|m| !m.starts_with("||") && !m.starts_with("```")));
    }

    #[test]
    fn gorunum_dusuncesiz() {
        let v = akis_gorunum(DusunmeKip::Goster, "", "kısa cevap");
        assert_eq!(v, vec!["kısa cevap"]);
    }

    #[test]
    fn dusunme_kip_ayristirilir() {
        assert_eq!(DusunmeKip::arg_ile("göster"), Some(DusunmeKip::Goster));
        assert_eq!(DusunmeKip::arg_ile("aç"), Some(DusunmeKip::Goster));
        assert_eq!(DusunmeKip::arg_ile("gizle"), Some(DusunmeKip::Gizle));
        assert_eq!(DusunmeKip::arg_ile("kapat"), Some(DusunmeKip::Kapali));
        assert_eq!(DusunmeKip::arg_ile("kapalı"), Some(DusunmeKip::Kapali));
        assert_eq!(DusunmeKip::arg_ile("saçma"), None);
        assert_eq!(DusunmeKip::Goster.dosya_degeri(), "goster");
    }

    #[test]
    fn gorunum_dusunurken_placeholder() {
        // göster: düz placeholder
        let v = akis_gorunum(DusunmeKip::Goster, "hmm düşünüyorum", "");
        assert_eq!(v, vec!["Düşünüyorum..."]);
        // gizle: canlı kelime sayacı
        let v = akis_gorunum(DusunmeKip::Gizle, "bir iki üç dört beş", "");
        assert_eq!(v, vec!["Düşünüyorum... Şu ana kadar 5 kelime düşündüm."]);
        // kapalıyken placeholder yok
        let v = akis_gorunum(DusunmeKip::Kapali, "", "");
        assert!(v.is_empty());
    }

    #[test]
    fn gorunum_cevap_basladi() {
        // göster: thinking hem spoiler hem kod bloğu + cevap
        let v = akis_gorunum(DusunmeKip::Goster, "düşündüm", "cevap bu");
        assert_eq!(v.len(), 3);
        assert!(v[0].starts_with("||") && v[0].ends_with("||"));
        assert!(v[1].starts_with("```"));
        assert_eq!(v[2], "cevap bu");
        // gizle: yalnız cevap (butonu gonder_akis ekler)
        let v = akis_gorunum(DusunmeKip::Gizle, "düşündüm", "cevap bu");
        assert_eq!(v, vec!["cevap bu"]);
    }

    #[test]
    fn isteklilik_puani_ayiklanir() {
        assert_eq!(
            isteklilik_puan(r#"{"puan": 7, "sebep": "bana soruldu"}"#),
            Some(7)
        );
        assert_eq!(isteklilik_puan("```json\n{\"puan\": 3}\n```"), Some(3));
        assert_eq!(isteklilik_puan(r#"{"puan": 25}"#), Some(10)); // clamp
        assert_eq!(isteklilik_puan(r#"{"puan": -4}"#), Some(0));
        assert_eq!(isteklilik_puan("puan veremem"), None);
    }

    #[test]
    fn dusunce_sayaci_artar() {
        assert_eq!(
            dusunce_sayaci("tek"),
            "Düşünüyorum... Şu ana kadar 1 kelime düşündüm."
        );
        assert_eq!(
            dusunce_sayaci("a b\nc  d"),
            "Düşünüyorum... Şu ana kadar 4 kelime düşündüm."
        );
    }

    #[test]
    fn dusunce_gosterim_kod_bloku() {
        let s = dusunce_gosterim("düşünce metni");
        assert!(s.starts_with("```\n") && s.ends_with("\n```"));
        assert!(s.contains("düşünce metni"));
        // uzun düşünce kısaltılır ve not düşer
        let uzun = "a".repeat(5000);
        let s = dusunce_gosterim(&uzun);
        assert!(s.chars().count() <= MESAJ_SINIRI);
        assert!(s.contains("kısaltıldı"));
    }

    #[test]
    fn tek_satira_indirgenir() {
        assert_eq!(tek_satir("a\nb\n\nc"), "a b c");
        assert_eq!(tek_satir("  boşluk   ve\nsatır  "), "boşluk ve satır");
    }

    // cevap bütçesi derleme durumuna göre değişir: ikisi de Option<u32>;
    // hangi değerin geldiği profile'a bağlı, burada tip ve tutarlılık denenir
    #[test]
    fn cevap_butcesi_tutarli() {
        let b: Option<u32> = cevap_butcesi!();
        if cfg!(debug_assertions) {
            assert!(b.is_some());
        } else {
            assert!(b.is_none());
        }
    }

    // sahte bir SSE sunucusundan gerçek reqwest akışı okur: utf-8 chunk
    // ortasında bölünse, reasoning ve content karışık gelse de doğru birikir
    #[tokio::test]
    async fn akis_okuyucu_sse_ayiklar() {
        use std::io::{Read, Write};
        let dinleyici = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let adres = dinleyici.local_addr().unwrap();
        std::thread::spawn(move || {
            let (mut baglanti, _) = dinleyici.accept().unwrap();
            let mut gelen = Vec::new();
            let mut tek = [0u8; 512];
            while !gelen.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = baglanti.read(&mut tek).unwrap_or(0);
                if n == 0 {
                    break;
                }
                gelen.extend_from_slice(&tek[..n]);
            }
            let govde = concat!(
                "data: {\"choices\":[{\"delta\":{\"reasoning\":\"önce düşün\"}}]}\n\n",
                "data: {\"choices\":[{\"delta\":{\"content\":\"Güne\"}}]}\n\n",
                "data: {\"choices\":[{\"delta\":{\"content\":\"ş bugün güzel\"}}]}\n\n",
                "data: [DONE]\n\n",
            );
            let cevap = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\n\r\n{govde}",
                govde.len()
            );
            baglanti.write_all(cevap.as_bytes()).unwrap();
            baglanti.flush().unwrap();
        });
        let cevap = reqwest::Client::new()
            .post(format!("http://{adres}/"))
            .json(&serde_json::json!({"stream": true}))
            .send()
            .await
            .unwrap();
        let mut okuyucu = AkisOkuyucu {
            cevap,
            tampon: Vec::new(),
            kuyruk: Vec::new(),
            kullanim: Kullanim::default(),
            done: false,
            bitti: false,
        };
        let mut metin = String::new();
        let mut dusunce = String::new();
        while let Some(p) = okuyucu.sonraki().await.unwrap() {
            metin.push_str(&p.metin);
            dusunce.push_str(&p.dusunce);
        }
        assert_eq!(dusunce, "önce düşün");
        assert_eq!(metin, "Güneş bugün güzel");
    }
}
