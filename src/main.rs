mod ajanlar;
mod gelisim;
mod gundem;
mod hafiza;
mod komut;
mod loglama;
mod modal;
mod promptlar;
mod seyahat;
mod sohbet_cli;
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
const PATLAMA_SINIRI: usize = 4; // bir turda en çok kaç satır (= ayrı mesaj) gider, fazlası atılır
const YARIM_SATIR_ESIGI: usize = 12; // akış sürerken son yarım satır bundan kısaysa henüz gösterilmez
                                     // stream OLMAYAN yolda satır arası bekleme: hepsi aynı anda düşmesin, yazıyormuş gibi araklansın.
                                     // Ölçülmedi, insan yazma hızından göz kararı seçildi.
const SATIR_GECIKME_TABAN: u64 = 300; // ms
const SATIR_GECIKME_HARF: u64 = 15; // ms, satırın karakteri başına
const SATIR_GECIKME_TAVAN: u64 = 1500; // ms, bekleme bunu aşmaz
const DUSUNCE_DUGMESI: &str = "dusunce_goster"; // gizle kipinde cevap sonundaki düşünce butonunun kimliği

// sohbet cevabı token bütçesi. release'de CEVAP_TAVANI: sıradan bir sohbet cevabı bunun
// çok altında kalır, yalnız tekrar/döngü gibi kaçak durumlarda maliyeti keser.
// 4096 seçildi çünkü reasoning üreten modellerde düşünce tokenleri de bu bütçeden düşer;
// daha dar tavanda uzun düşünce + cevap kırpılabilirdi.
// debug'da daha küçük Some → geliştirme/test turunda token yakmasın diye kapak.
const CEVAP_TAVANI: u32 = 4096;
macro_rules! cevap_butcesi {
    () => {
        if cfg!(debug_assertions) {
            Some(2000u32)
        } else {
            Some(CEVAP_TAVANI)
        }
    };
}
// macro_rules metinsel sırayla görünür: makro, dosya başındaki `mod` bildirimlerinden
// sonra tanımlandığı için alt modüllerden çağrılamaz. Aynı bütçeyi veren tek sarmalayıcı
// (sohbet_cli kullanır), iki yerde iki ayrı tavan tutmayalım diye
fn sohbet_butcesi() -> Option<u32> {
    cevap_butcesi!()
}
// http: toplam süre sınırı yok (uzun düşünme akışları kesilmesin); bağlantı
// kurulamıyorsa ve veri gelmiyorsa ayrı ayrı kesilir
const BAGLANTI_ZAMAN_ASIMI: Duration = Duration::from_secs(15); // tcp/tls el sıkışma
const OKUMA_ZAMAN_ASIMI: Duration = Duration::from_secs(120); // iki veri arası en çok; ilk tokeni de kapsar
const AI_YENIDEN_DENEME: u32 = 2; // ağ hatası / 429 / 5xx'te toplam deneme sayısı bu + 1

// reasoning kapatılamayan modelde (mandatory) mini-çağrıların bütçesi bu tabana çıkarılır;
// yoksa reasoning tamamı yiyip content: null döner (bkz. reasoning_zorunlu_hatasi)
const REASONING_ZORUNLU_TABAN: u32 = 500;
const FAVORI: u64 = 259669117248864257; // bu kişiyi ne olursa olsun sever
const GEZGIN_ARALIGI: Duration = Duration::from_secs(4 * 60 * 60); // ne sıklıkla internette gezer
const RESIM_KLASORU: &str = "resimler"; // şakalarda atılacak görseller
const DURUM_KLASORU: &str = "durum"; // ajanların öğrendikleri buraya yazılır

type Hata = Box<dyn std::error::Error + Send + Sync>;

// kapanış sinyali: döngüler tik başında bakar, yeni tur açmaz; bekçi yeniden başlatmaz
static KAPANIYOR: AtomicBool = AtomicBool::new(false);

// ---------- durum ----------

#[derive(Serialize, Clone)]
struct Mesaj {
    role: &'static str,
    content: String,
    // ekli görselin adresi; istek gövdesi elle kurulduğu için serileştirmeye girmez.
    // Yalnız son kullanıcı mesajında dolu kalır: discord cdn linki ömürlü, eski
    // görseli her turda yeniden yollamak boşuna token yakar.
    #[serde(skip)]
    resim: Option<String>,
}

fn kullanici(metin: impl Into<String>) -> Mesaj {
    Mesaj {
        role: "user",
        content: metin.into(),
        resim: None,
    }
}

// görsel ekli kullanıcı mesajı: metinle birlikte resim de modele gider
fn kullanici_resimli(metin: impl Into<String>, url: impl Into<String>) -> Mesaj {
    Mesaj {
        role: "user",
        content: metin.into(),
        resim: Some(url.into()),
    }
}

fn asistan(metin: impl Into<String>) -> Mesaj {
    Mesaj {
        role: "assistant",
        content: metin.into(),
        resim: None,
    }
}

// mesajı openai uyumlu istek bloğuna çevirir. Resim varsa content düz metin değil
// çok parçalı dizi olur; biçim ajanlar.rs'deki resimci ile aynı, sağlayıcılar bunu anlıyor
fn mesaj_json(m: &Mesaj) -> serde_json::Value {
    match &m.resim {
        None => serde_json::json!({ "role": m.role, "content": m.content }),
        Some(url) => serde_json::json!({
            "role": m.role,
            "content": [
                { "type": "text", "text": m.content },
                { "type": "image_url", "image_url": { "url": url } }
            ]
        }),
    }
}

#[derive(Default)]
struct Sohbet {
    gecmis: Vec<Mesaj>,
    sayac: u32,
    hackli: u32, // 0 değilse hacklenmiş taklidi sürüyor, her cevapta bir azalır
    son_mesaj: Option<MessageId>, // cevaplanacak mesaj (discord yanıtı olarak)
    son_etiketlendi: bool, // son_mesaj etiket/yanıt/ad ile mi geldi (reply-to gerekçesi)
    gelen: u32,  // gelen kullanıcı mesajı sayısı; cevap yazarken yenisi geldi mi anlamak için
    // botun son cevabından beri gelenler (isim, mesaj id); hedef seçiminde kullanılır,
    // bot cevap verince boşalır
    son_gelenler: VecDeque<(String, MessageId)>,
    ruh_hali: String, // "kafa karışıklığı (6)" gibi; boşsa henüz belirlenmedi ya da yoğunluk düşük
}

// düşünme gösterim kipi; !düşünme ile değişir, durum/dusunme.md'de kalır
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum DusunmeKip {
    #[default]
    Goster, // reasoning istenir, cevapla birlikte spoiler içinde gösterilir
    Gizle,  // reasoning istenir ama gösterilmez; düşünürken "Düşünüyorum..." yazar
    Sessiz, // reasoning istenir (arka planda düşünür) ama hiçbir iz göstermez: placeholder/sayaç/buton yok, doğrudan cevap gelir
    Kapali, // reasoning istenmez, istekler düşünmesiz atılır
}

impl DusunmeKip {
    fn dosya_degeri(self) -> &'static str {
        match self {
            DusunmeKip::Goster => "goster",
            DusunmeKip::Gizle => "gizle",
            DusunmeKip::Sessiz => "sessiz",
            DusunmeKip::Kapali => "kapali",
        }
    }

    fn oku() -> Self {
        match hafiza::oku("dusunme.md").trim() {
            "gizle" => DusunmeKip::Gizle,
            "sessiz" => DusunmeKip::Sessiz,
            "kapali" => DusunmeKip::Kapali,
            _ => DusunmeKip::Goster,
        }
    }

    // komut argümanından kip; tanınmıyorsa None
    fn arg_ile(arg: &str) -> Option<Self> {
        match arg {
            "göster" | "goster" | "aç" | "ac" | "on" => Some(DusunmeKip::Goster),
            "gizle" => Some(DusunmeKip::Gizle),
            "sessiz" => Some(DusunmeKip::Sessiz),
            "kapat" | "kapalı" | "kapali" | "off" => Some(DusunmeKip::Kapali),
            _ => None,
        }
    }

    fn ad(self) -> &'static str {
        match self {
            DusunmeKip::Goster => "göster",
            DusunmeKip::Gizle => "gizli",
            DusunmeKip::Sessiz => "sessiz",
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
    // uyku: uyandığında gece yazılanları değerlendirebilsin diye başlangıç anı + ham hafıza boyu
    uyku_basi: i64,
    uyku_basi_hafiza_len: usize,
    son_gece_gozlem: i64, // uykuda zihne son işleme anı (2 saatte bir gözlem)
    stok_haber: Option<ajanlar::Haber>, // uykuda seçilen, uyanınca atılacak haber
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
            // daha önce taranmış sunucular: her yeniden başlangıçta 14 günlük geçmişi
            // yeniden çekmesin diye kalıcı (guild_create her ready'de yeniden gelir)
            taranan: hafiza::oku("taranan.md")
                .lines()
                .filter_map(|l| l.trim().parse::<u64>().ok())
                .map(GuildId::new)
                .collect(),
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

// kanalın meşgul bayrağını bırakır: normal dönüş, erken dönüş ve panikte Drop
// çalışır; bayrak unutulup kanal kalıcı kilitlenemez
struct MesgulGuard<'a> {
    durum: &'a Mutex<Durum>,
    kanal: ChannelId,
}

impl Drop for MesgulGuard<'_> {
    fn drop(&mut self) {
        self.durum
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .mesgul
            .remove(&self.kanal);
    }
}

struct Bot {
    durum: Mutex<Durum>,
    http: reqwest::Client,
    api_adres: String, // chat/completions adresi (openrouter ya da mistral)
    anahtar: String,
    haber_kanali: Option<ChannelId>,
    firecrawl: Option<String>, // yoksa sayfalar düz indirilir
    guild_id: Option<GuildId>, // .env GUILD_ID; ayarlıysa yalnız bu sunucuda çalışır
    izinli_kanallar: Option<HashSet<ChannelId>>, // .env KANALLAR; ayarlıysa yalnız bu kanallarda
}

impl Bot {
    fn durum(&self) -> MutexGuard<'_, Durum> {
        self.durum.lock().unwrap_or_else(|e| e.into_inner())
    }

    // model kullanımını oturum metriğine ekler, kategoriye göre de kırılır (!durum döker)
    fn metrik_ekle(&self, kategori: &'static str, k: Kullanim) {
        log::debug!(
            "api [{kategori}]: giris={} onbellek={} cikis={}",
            k.prompt_tokens,
            k.prompt_tokens_details.cached_tokens,
            k.completion_tokens,
        );
        let mut d = self.durum();
        d.metrik.cagri += 1;
        d.metrik.giris_token += k.prompt_tokens;
        d.metrik.onbellek_token += k.prompt_tokens_details.cached_tokens;
        d.metrik.cikis_token += k.completion_tokens;
        d.metrik.son_cagri_sn = simdi_unix();
        d.metrik.kategoriler.entry(kategori).or_default().topla(k);
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
// sonra boşluk, o da yoksa tam sınırdan sert keser; hiçbir şey atılmaz.
// dilim üzerinde yürür: tur başına ara tahsis yok, yalnız çıkan parça String olur
fn bol(metin: &str, sinir: usize) -> Vec<String> {
    let mut parcalar: Vec<String> = Vec::new();
    let mut kalan = metin.trim();
    while kalan.char_indices().nth(sinir).is_some() {
        let kes = kesim_noktasi(kalan, sinir); // bayt ofseti
        let bas = kalan[..kes].trim();
        if !bas.is_empty() {
            parcalar.push(bas.to_string());
        }
        kalan = kalan[kes..].trim_start();
    }
    if !kalan.is_empty() {
        parcalar.push(kalan.to_string());
    }
    parcalar
}

// ilk sinir karakter içindeki en iyi kesim yerinin BAYT ofseti; çok ufak parça
// çıkmasın diye cümle/boşluk kesimi sınırın dörtte birinden sonra değilse sert kese düşer
fn kesim_noktasi(metin: &str, sinir: usize) -> usize {
    let mut cumle = (0usize, 0usize); // (karakter sırası, bayt ofseti)
    let mut bosluk = (0usize, 0usize);
    let mut bitis = metin.len();
    for (i, (off, c)) in metin.char_indices().enumerate() {
        if i >= sinir {
            bitis = off;
            break;
        }
        if matches!(c, '.' | '!' | '?' | '\n') {
            cumle = (i + 1, off + c.len_utf8());
        } else if c == ' ' {
            bosluk = (i, off);
        }
    }
    let asgari = sinir / 4;
    if cumle.0 > asgari {
        cumle.1
    } else if bosluk.0 > asgari {
        bosluk.1
    } else {
        bitis
    }
}

// discord spoiler'ı; içindeki dik çizgiler kaçırılır ki spoiler bozulmasın
fn spoiler(metin: &str) -> String {
    format!("||{}||", metin.replace('|', "\\|"))
}

// kanalın geçmişine satır ekler ve dosyaya yazar; sohbet bitse, bot yeniden başlasa da kalır
fn kanal_not(d: &mut Durum, kanal: ChannelId, satir: String) {
    kanal_not_coklu(d, kanal, [satir]);
}

// birden çok satırı TEK dosya yazımıyla ekler. Cevap artık satır satır gittiği için
// her satıra ayrı yazım bütün kanal geçmişini tur başına 4-5 kez baştan yazıyordu.
fn kanal_not_coklu(d: &mut Durum, kanal: ChannelId, satirlar: impl IntoIterator<Item = String>) {
    let mut satirlar = satirlar.into_iter().peekable();
    if satirlar.peek().is_none() {
        return;
    }
    let g = d.kanal_gecmisi.entry(kanal).or_default();
    g.extend(satirlar);
    while g.len() > KANAL_GECMIS {
        g.pop_front();
    }
    // ara Vec yok: doğrudan tek String'e birleştirilir
    let mut icerik = String::new();
    for (i, satir) in g.iter().enumerate() {
        if i > 0 {
            icerik.push('\n');
        }
        icerik.push_str(satir);
    }
    hafiza::yaz(&format!("kanallar/{}.md", kanal.get()), &icerik);
}

fn son_mesajlar(d: &Durum, n: usize) -> String {
    let atla = d.hafiza.len().saturating_sub(n);
    let mut s = String::new();
    for (i, satir) in d.hafiza.iter().skip(atla).enumerate() {
        if i > 0 {
            s.push('\n');
        }
        s.push_str(satir);
    }
    s
}

// sohbeti eleştirmen/günlükçü/hoca'nın okuduğu düz döküme çevirir. Bot cevabı çok satırlı
// olabilir (her satır ayrı mesaj gitti, tepki satırı da içeride): önek HER satıra konur,
// yoksa ikinci ve sonraki satırlar gruptaki insanlara aitmiş gibi okunur
fn dokum(gecmis: &[Mesaj], bot_adi: &str) -> String {
    let mut s = String::new();
    let mut ilk = true;
    for m in gecmis {
        for satir in m.content.split('\n') {
            if !ilk {
                s.push('\n');
            }
            ilk = false;
            if m.role == "assistant" {
                s.push_str(bot_adi);
                s.push_str(": ");
            }
            s.push_str(satir);
        }
    }
    s
}

// karşılaştırma için küçük harfe indirir; İ→i̇ dönüşümünün eklediği birleşik
// noktayı atar ki "ŞAHİN"/"şahin" gibi adlar eşleşsin
fn kucult(s: &str) -> String {
    s.to_lowercase().replace('\u{0307}', "")
}

// modelin başa ekleyebildiği ad öneki ve tırnakları soyar; dilim döndürür.
// sıcak yolda (stream'de her edit) metnin tamamı klonlanıp lowercase edilmez:
// önek karşılaştırması yalnız ilk karakterlere, kucult'taki gibi birleşik nokta atılarak
fn soy<'a>(metin: &'a str, bot_adi: &str) -> &'a str {
    let mut m = metin.trim();
    // "isim: metin" kalıbını taklit edip başına kendi adını koyabiliyor
    let onek = format!("{bot_adi}:");
    let karakter = onek.chars().count();
    let bas: String = m
        .chars()
        .take(karakter)
        .flat_map(|c| c.to_lowercase())
        .filter(|&c| c != '\u{0307}')
        .collect();
    if bas.starts_with(&kucult(&onek)) {
        m = match m.char_indices().nth(karakter) {
            Some((off, _)) => m[off..].trim(),
            None => "",
        };
    }
    if m.chars().count() > 1 && m.starts_with('"') && m.ends_with('"') {
        m = &m[1..m.len() - 1];
    }
    m
}

// stream'siz yollar tek mesajla sınırlı: soy + 1900 kapak.
// stream yolu soy'dan sonra bol() ile bölerek gönderir, kırpma yok.
fn temizle(metin: String, bot_adi: &str) -> String {
    let m = soy(&metin, bot_adi);
    // sınırda bayt ofsetini bul, yerinde kes: ara collect yok
    match m.char_indices().nth(MESAJ_SINIRI) {
        Some((off, _)) => m[..off].to_string(),
        None => m.to_string(),
    }
}

// ---------- çıktı protokolü ----------

// Modelin cevabı satır bazlı bir protokoldür: her satır ayrı bir mesaj olarak gider,
// "tepki: 💀" satırı yazı yerine emoji tepkisi olur, tek başına "-" susma işaretidir.
// soy() uygulanmış metin üzerinde çalışılır, burada yeniden soyulmaz.
#[derive(Default, Debug, PartialEq, Eq)]
struct Cevap {
    satirlar: Vec<String>,
    tepki: Option<String>,
    sus: bool,
}

impl Cevap {
    // cevaptan kullanılır hiçbir şey çıkmadı mı (ne söz, ne tepki, ne susma kararı)
    fn bos(&self) -> bool {
        self.satirlar.is_empty() && self.tepki.is_none() && !self.sus
    }

    // sohbet geçmişine ve kanal notuna giren biçim: model kendi protokolünü geri görsün,
    // sonraki turda aynı biçimde yazsın
    fn protokol_metni(&self) -> String {
        let mut s = self.satirlar.join("\n");
        if let Some(t) = &self.tepki {
            if !s.is_empty() {
                s.push('\n');
            }
            s.push_str("tepki: ");
            s.push_str(t);
        }
        s
    }
}

// satır "tepki:" ile mi başlıyor (büyük/küçük harf farketmez, "tepki :" de olur);
// öyleyse iki noktadan sonrası döner
fn tepki_govdesi(satir: &str) -> Option<&str> {
    let (bas, kalan) = satir.split_once(':')?;
    (kucult(bas.trim()) == "tepki").then_some(kalan)
}

// karakter gerçekten emoji mi. "harf değilse emojidir" demek yetmiyor: "—", "…", "→",
// tipografik tırnak da o testi geçiyor ve discord'a Unicode tepki olarak gidince istek
// 400 ile dönüyor. Bilinen emoji blokları sayılır, gerisi tepki olmaz.
fn emoji_basi(c: char) -> bool {
    matches!(c as u32,
        0xA9 | 0xAE | 0x2122 | 0x3030 | 0x303D | 0x3297 | 0x3299
        | 0x2600..=0x27BF   // çeşitli semboller + dingbat
        | 0x2B00..=0x2BFF   // ok/yıldız/kare sembolleri (⭐ gibi)
        | 0x1F000..=0x1FAFF // asıl emoji blokları (bayraklar, ten tonu dahil)
    )
}

// dizinin devamına takılabilecekler: varyasyon seçici, ZWJ, keycap
fn emoji_devami(c: char) -> bool {
    emoji_basi(c) || matches!(c as u32, 0xFE0F | 0xFE0E | 0x200D | 0x20E3)
}

// metindeki ilk emoji dizisi; peşine takılan varyasyon seçici / ZWJ de alınır.
// ":kekw:" gibi özel emoji biçiminde ve emoji hiç yoksa None.
fn emoji_ayikla(metin: &str) -> Option<String> {
    let (bas, _) = metin.char_indices().find(|(_, c)| emoji_basi(*c))?;
    Some(
        metin[bas..]
            .chars()
            .take_while(|c| emoji_devami(*c))
            .take(8)
            .collect(),
    )
}

// satır tek başına susma işareti mi
fn sus_isareti(satir: &str) -> bool {
    matches!(satir, "-" | "\"-\"" | "'-'") || matches!(kucult(satir).as_str(), "[sus]" | "(sus)")
}

// "1. " / "2) " gibi numara önekinden sonrası (rakamlar tek baytlık, dilim güvenli).
// Tek başına elenmez: "3. sınıftayım", "2. el araba" Türkçe'de sıra sayısıdır, madde değil.
fn numara_oneki(s: &str) -> Option<&str> {
    let basamak = s.chars().take_while(char::is_ascii_digit).count();
    if basamak == 0 {
        return None;
    }
    let kalan = &s[basamak..];
    kalan
        .strip_prefix(". ")
        .or_else(|| kalan.strip_prefix(") "))
        .map(str::trim_start)
}

// "yapay zeka yazmış" izlerini siler: baştaki madde öneki ve kalın/altı çizili markdown
// işaretleri. Numara öneki burada değil `cevap_parcala`'da elenir (gerçek liste mi sıra
// sayısı mı, ancak cevabın tamamına bakınca anlaşılır). Backtick'in İÇİ de korunur:
// `__init__` gibi kod parçası gerçek bilgi taşır.
fn slop_temizle(satir: &str) -> String {
    let mut s = satir.trim();
    for onek in ["- ", "* ", "• "] {
        if let Some(k) = s.strip_prefix(onek) {
            s = k.trim_start();
            break;
        }
    }
    let mut cikti = String::with_capacity(s.len());
    // backtick ile bölünce tek indeksli parçalar kod içidir, dokunulmaz
    for (i, parca) in s.split('`').enumerate() {
        if i > 0 {
            cikti.push('`');
        }
        if i % 2 == 0 {
            cikti.push_str(&parca.replace("**", "").replace("__", ""));
        } else {
            cikti.push_str(parca);
        }
    }
    cikti.trim().to_string()
}

// Model cevabını protokole göre çözer. Kısa satır elenmez: "he", "yok", "la" da
// doğal bir tepkidir.
fn cevap_parcala(metin: &str) -> Cevap {
    let mut c = Cevap::default();
    let mut satirlar: Vec<String> = Vec::new();
    let mut atlanan = 0usize;
    // numara öneki ancak GERÇEK liste ise elenir: iki ya da daha çok numaralı satır varsa
    // model madde yazmış demektir. Tek satırdaki "3. sınıftayım" sıra sayısıdır, yenmez.
    let liste = metin
        .split('\n')
        .filter(|s| numara_oneki(s.trim()).is_some())
        .count()
        >= 2;
    for ham in metin.split('\n') {
        let mut satir = ham.trim();
        if satir.is_empty() {
            continue;
        }
        if sus_isareti(satir) {
            c.sus = true;
            continue;
        }
        if let Some(govde) = tepki_govdesi(satir) {
            // ilk tepki kazanır; emoji çözülemezse satır yine de mesaj olarak gitmez
            if c.tepki.is_none() {
                c.tepki = emoji_ayikla(govde);
            }
            continue;
        }
        // önceki mesajın kırıntısı ("'cım" gibi) gitmesin
        if satir.starts_with('\'') {
            continue;
        }
        if liste {
            if let Some(k) = numara_oneki(satir) {
                satir = k;
            }
        }
        let temiz = slop_temizle(satir);
        if temiz.is_empty() {
            continue;
        }
        // aynı turda birebir aynı laf iki mesaj olarak gitmesin ("he\nhe")
        if satirlar.contains(&temiz) {
            continue;
        }
        if satirlar.len() >= PATLAMA_SINIRI {
            atlanan += 1;
            continue;
        }
        satirlar.push(temiz);
    }
    if atlanan > 0 {
        log::debug!("cevap: patlama sınırı aşıldı, {atlanan} satır düştü");
    }
    // 1900'ü aşan satır tek mesaja sığmaz: bölünür ve düzleştirilir
    c.satirlar = satirlar.iter().flat_map(|s| bol(s, MESAJ_SINIRI)).collect();
    c
}

// son 4 bot satırından (tepki satırları sayılmaz) ikisi soruyla bittiyse üst üste
// soru sormuş demektir; cevapla bunu talimata çevirir
fn soru_fazla_mi(d: &Durum, kanal: ChannelId) -> bool {
    let onek = format!("{}: ", d.bot_adi);
    d.kanal_gecmisi
        .get(&kanal)
        .map(|g| {
            g.iter()
                .rev()
                .filter_map(|l| l.strip_prefix(&onek))
                .filter(|l| tepki_govdesi(l).is_none())
                .take(4)
                .filter(|l| l.trim_end().ends_with('?'))
                .count()
        })
        .unwrap_or(0)
        >= 2
}

// geçici sayılan durum kodları: geri çekilip yeniden denemeye değer
// (429 hız sınırı, 5xx sunucu tarafı); 401/404 gibi kalıcı hatalar denemeye girmez
fn durum_denenebilir(d: reqwest::StatusCode) -> bool {
    matches!(d.as_u16(), 429 | 500 | 502 | 503 | 504)
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

// sağlayıcı "reasoning kapatılamaz" diye 400 dönmüş mü (bazı GLM reasoning varyantları gibi)
fn reasoning_zorunlu_hatasi(govde_metni: &str) -> bool {
    let m = govde_metni.to_lowercase();
    m.contains("reasoning") && (m.contains("mandatory") || m.contains("cannot be disabled"))
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

// hedef seçiminden kişi adını çözer: önce JSON, olmazsa ilk satır/kelime
fn hedef_ayikla(cevap: &str, bilinenler: &[String]) -> Option<String> {
    #[derive(Deserialize)]
    struct Hedef {
        #[serde(default)]
        hedef: String,
    }
    let aday = serde_json::from_str::<Hedef>(json_ayikla(cevap))
        .map(|h| h.hedef.trim().to_string())
        .unwrap_or_else(|_| {
            cevap
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches('"')
                .to_string()
        });
    if aday.is_empty() {
        return None;
    }
    // bilinen adlardan birine benziyorsa onu kullan (model süslemiş olabilir)
    bilinenler
        .iter()
        .find(|b| b.eq_ignore_ascii_case(&aday))
        .cloned()
        .or(Some(aday))
}

// ruh hali cevabından durum+yoğunluğu çözer; yoğunluk düşükse (nötr) hiç yansıtmaya değmez
fn ruh_hali_ayikla(cevap: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct RuhHali {
        #[serde(default)]
        durum: String,
        #[serde(default)]
        yogunluk: i32,
    }
    let r: RuhHali = serde_json::from_str(json_ayikla(cevap)).ok()?;
    let durum = r.durum.trim();
    if durum.is_empty() {
        return None;
    }
    let yogunluk = r.yogunluk.clamp(1, 10);
    if yogunluk < 3 {
        return None; // nötr/belirsiz: talimata eklemeye değmez, düz kişilik konuşsun
    }
    Some(format!("{durum} ({yogunluk})"))
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
    #[serde(default)]
    prompt_tokens_details: OnbellekDetay,
}

// openai uyumlu sağlayıcıların prompt cache isabetini bildirdiği alan; yoksa 0
#[derive(Deserialize, Default, Clone, Copy, Debug)]
struct OnbellekDetay {
    #[serde(default)]
    cached_tokens: u64,
}

impl Kullanim {
    fn topla(&mut self, diger: Kullanim) {
        self.prompt_tokens += diger.prompt_tokens;
        self.completion_tokens += diger.completion_tokens;
        self.prompt_tokens_details.cached_tokens += diger.prompt_tokens_details.cached_tokens;
    }
}

// oturum boyu biriken model kullanım metriği; !durum bunu gösterir.
// kategoriler çağrı türüne göre kırılım verir (sohbet/isteklilik/profilci/...).
#[derive(Default, Clone, Debug)]
struct Metrik {
    cagri: u32,
    giris_token: u64,
    onbellek_token: u64, // giris_token içinden önbellekten karşılanan (sağlayıcı bildirdiyse)
    cikis_token: u64,
    son_cagri_sn: i64,
    kategoriler: HashMap<&'static str, Kullanim>,
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
    tampon: Vec<u8>,        // henüz satıra bölünmemiş baytlar
    kuyruk: Vec<Parca>,     // çözülmüş, verilmeyi bekleyen parçalar
    kullanim: Kullanim,     // son chunk'tan toplanan token sayacı
    kategori: &'static str, // token metriği kırılımı için (!durum)
    done: bool,             // [DONE] görüldü mü (temiz kapanış işareti)
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
    Bos,                // akıştan kullanılır bir şey çıkmadı
    Sus,                // model susmayı seçti ("-"): hiçbir şey gitmez, geçmişe de girmez
}

// gonder_akis'in cevap bağlamı; argüman yığını yerine tek yapı
struct AkisBaglam<'a> {
    bot_adi: &'a str,
    yanit: Option<MessageId>,
    // emoji tepkisinin düşeceği mesaj; yanit koşullu olduğu için ayrı alan
    tepki_hedefi: Option<MessageId>,
    gecmis: &'a [Mesaj],
    talimat: &'a str,
    butce: Option<u32>,
}

impl Bot {
    // düşünme kapalıysa modelin reasoning üretmesini istekte kapatır (token harcamasın).
    // her sağlayıcının dili farklı: openrouter "reasoning", qwen tarzı router'lar
    // "enable_thinking" anlar, mistral'de düğme yok (ikisini birden yollamak bazı
    // sağlayıcıları bozuyordu, adres bazlı seçilir). Alanları gerçekten eklediyse true
    // döner (bazı modeller yine de reasoning'i kapatmaya izin vermiyor, çağıran taraf
    // bunu geri alıp bir kez daha dener).
    // herhalukarda=true: kullanıcının kipine bakmadan kapatır — sor_ham (stream olmayan)
    // reasoning_content alanını hiç okumaz/göstermez, o yüzden arka plan ajanları (profilci,
    // hoca, günlükçü, gezgin, isteklilik, ruh_hali) kip "gizle" iken bile boşuna düşünüp küçük
    // max_tokens bütçesini tüketmesin, content: null dönüp "modelden boş yanıt geldi" hatasına
    // düşmesin. sor_ham_akis (stream, sohbet) kullanıcı kipine bakmaya devam eder: "gizle"
    // düşünce sayacı, "göster" tam metin gösterir, o yüzden false geçer.
    fn reasoning_kapat(&self, govde: &mut serde_json::Value, herhalukarda: bool) -> bool {
        if !herhalukarda && self.durum().dusunme != DusunmeKip::Kapali {
            return false;
        }
        let Some(o) = govde.as_object_mut() else {
            return false;
        };
        let adres = self.api_adres.to_lowercase();
        if adres.contains("openrouter") {
            o.insert("reasoning".into(), serde_json::json!({ "enabled": false }));
        } else if !adres.contains("mistral") {
            o.insert("enable_thinking".into(), serde_json::json!(false));
        } else {
            return false; // mistral: hiçbir alan eklenmedi, geri alacak bir şey yok
        }
        true
    }

    // reasoning_kapat'ın eklediği alanları geri alır (mandatory-reasoning hatasında yeniden deneme için)
    fn reasoning_alanlarini_kaldir(govde: &mut serde_json::Value) {
        if let Some(o) = govde.as_object_mut() {
            o.remove("reasoning");
            o.remove("enable_thinking");
        }
    }

    // max_tokens taban altındaysa yükseltir; reasoning zorunlu modelde bütçesiz kalmasın diye.
    // max_tokens hiç yoksa (bütçesiz çağrı) dokunmaz. Değiştirdiyse true döner (log için).
    fn butce_tabanini_uygula(govde: &mut serde_json::Value, taban: u32) -> bool {
        match govde.get("max_tokens").and_then(serde_json::Value::as_u64) {
            Some(mevcut) if (mevcut as u32) < taban => {
                govde["max_tokens"] = serde_json::json!(taban);
                true
            }
            _ => false,
        }
    }

    // açıklayıcı hata ile ham istek; her şey buradan geçer. Ağ hatası / 429 / 5xx'te geri
    // çekilip yeniden dener; bazı modeller (ör. bazı GLM reasoning varyantları) reasoning'i
    // kapatmaya izin vermiyor ("mandatory"/"cannot be disabled" 400) — o durumda alanları
    // kaldırıp açık haliyle yeniden dener, model her turda başarısız olup sohbeti tıkamasın.
    // kategori yalnız token metriğinde kırılım için (!durum), isteğe hiçbir etkisi yok.
    async fn sor_ham(
        &self,
        mut govde: serde_json::Value,
        kategori: &'static str,
    ) -> Result<String, Hata> {
        let mut kapatildi = self.reasoning_kapat(&mut govde, true);
        let mut son_hata: Hata = "istek hiç yapılamadı".into();
        for deneme in 0..=AI_YENIDEN_DENEME {
            if deneme > 0 {
                sleep(Duration::from_secs(u64::from(deneme) * 2)).await;
                log::warn!("ai [sor_ham]: {son_hata} — {}. deneme", deneme + 1);
            }
            let cevap = match self
                .http
                .post(&self.api_adres)
                .bearer_auth(&self.anahtar)
                .json(&govde)
                .send()
                .await
            {
                Ok(c) => c,
                Err(e) if e.is_connect() || e.is_timeout() || e.is_request() => {
                    son_hata = e.into();
                    continue;
                }
                Err(e) => return Err(e.into()),
            };
            let durum = cevap.status();
            let govde_metni = cevap.text().await.unwrap_or_default();
            if !durum.is_success() {
                // 404 çoğunlukla "bu isimde model yok" demek; gövdedeki mesajı ve modeli göster
                let model = govde.get("model").and_then(|m| m.as_str()).unwrap_or("?");
                let hata: Hata =
                    format!("{durum} (model: {model}): {}", kirp_hata(&govde_metni)).into();
                if deneme < AI_YENIDEN_DENEME && kapatildi && reasoning_zorunlu_hatasi(&govde_metni)
                {
                    log::warn!(
                        "ai [sor_ham]: model reasoning kapatılmasına izin vermiyor, açık yeniden deneniyor"
                    );
                    Self::reasoning_alanlarini_kaldir(&mut govde);
                    kapatildi = false;
                    if Self::butce_tabanini_uygula(&mut govde, REASONING_ZORUNLU_TABAN) {
                        log::warn!(
                            "ai [sor_ham]: küçük bütçe reasoning'e yetmeyebilir, {REASONING_ZORUNLU_TABAN}'a çıkarıldı"
                        );
                    }
                    son_hata = hata;
                    continue;
                }
                if deneme < AI_YENIDEN_DENEME && durum_denenebilir(durum) {
                    son_hata = hata;
                    continue;
                }
                return Err(hata);
            }
            let yanit: Yanit = serde_json::from_str(&govde_metni)?;
            if let Some(k) = yanit.usage {
                self.metrik_ekle(kategori, k);
            }
            let metin = yanit
                .choices
                .into_iter()
                .next()
                .and_then(|s| s.message.content)
                .filter(|s| !s.trim().is_empty());
            match metin {
                Some(metin) => return Ok(metin.trim().to_string()),
                // kapatılamayan reasoning küçük bütçeyi yiyip content: null bırakmış olabilir;
                // bütçeyi (varsa) tabana çıkarıp bir kez daha dene, taban da yetmediyse pes et
                None if deneme < AI_YENIDEN_DENEME => {
                    let buyudu = Self::butce_tabanini_uygula(&mut govde, REASONING_ZORUNLU_TABAN);
                    log::warn!(
                        "ai [sor_ham]: modelden boş yanıt geldi{}",
                        if buyudu {
                            format!(
                                ", bütçe {REASONING_ZORUNLU_TABAN}'a çıkarılıp yeniden deneniyor"
                            )
                        } else {
                            String::new()
                        }
                    );
                    son_hata = "modelden boş yanıt geldi".into();
                }
                None => return Err("modelden boş yanıt geldi".into()),
            }
        }
        Err(son_hata)
    }

    async fn sor(
        &self,
        sistem: &str,
        gecmis: &[Mesaj],
        max_tokens: u32,
        kategori: &'static str,
    ) -> Result<String, Hata> {
        self.sor_bolumlu(sistem, "", gecmis, Some(max_tokens), kategori)
            .await
    }

    // sistem mesajı iki blok: sabit blok destekleyen sağlayıcılarda cache_control ile
    // işaretli (anthropic/gemini); değişken blok her seferinde yeniden okunur.
    // butce None ise max_tokens gitmez, model bütçesiz konuşur.
    async fn sor_bolumlu(
        &self,
        sabit: &str,
        degisken: &str,
        gecmis: &[Mesaj],
        butce: Option<u32>,
        kategori: &'static str,
    ) -> Result<String, Hata> {
        let model = self.durum().model.clone();
        let mut mesajlar = vec![sistem_json(sabit, degisken, &self.api_adres)];
        mesajlar.extend(gecmis.iter().map(mesaj_json));
        let mut govde = serde_json::json!({
            "model": model,
            "messages": mesajlar,
            "temperature": 0.7,
        });
        if let Some(t) = butce {
            govde["max_tokens"] = serde_json::json!(t);
        }
        self.sor_ham(govde, kategori).await
    }

    // stream istek: hata kontrolü sor_ham ile aynı, gövdeye stream eklenir.
    // butce None ise max_tokens gitmez, model bütçesiz konuşur (release sohbet yolu).
    async fn sor_ham_akis(
        &self,
        sabit: &str,
        degisken: &str,
        gecmis: &[Mesaj],
        butce: Option<u32>,
        kategori: &'static str,
    ) -> Result<AkisOkuyucu, Hata> {
        let model = self.durum().model.clone();
        let mut mesajlar = vec![sistem_json(sabit, degisken, &self.api_adres)];
        mesajlar.extend(gecmis.iter().map(mesaj_json));
        let mut govde = serde_json::json!({
            "model": model,
            "messages": mesajlar,
            "temperature": 0.7,
            "stream": true,
            // son chunk'ta usage gelsin (token sayacı)
            "stream_options": { "include_usage": true },
        });
        if let Some(t) = butce {
            govde["max_tokens"] = serde_json::json!(t);
        }
        let mut kapatildi = self.reasoning_kapat(&mut govde, false);
        // yeniden deneme yalnız akış açılmadan önce: parça gelmeye başladıysa okuyucu dönmüş
        // olur, sonrası gonder_akis'in yarım-kaldı yoludur
        let mut son_hata: Hata = "istek hiç yapılamadı".into();
        for deneme in 0..=AI_YENIDEN_DENEME {
            if deneme > 0 {
                sleep(Duration::from_secs(u64::from(deneme) * 2)).await;
                log::warn!("ai [sor_ham_akis]: {son_hata} — {}. deneme", deneme + 1);
            }
            let cevap = match self
                .http
                .post(&self.api_adres)
                .bearer_auth(&self.anahtar)
                .json(&govde)
                .send()
                .await
            {
                Ok(c) => c,
                Err(e) if e.is_connect() || e.is_timeout() || e.is_request() => {
                    son_hata = e.into();
                    continue;
                }
                Err(e) => return Err(e.into()),
            };
            let durum = cevap.status();
            if !durum.is_success() {
                let govde_metni = cevap.text().await.unwrap_or_default();
                let model = govde.get("model").and_then(|m| m.as_str()).unwrap_or("?");
                let hata: Hata =
                    format!("{durum} (model: {model}): {}", kirp_hata(&govde_metni)).into();
                if deneme < AI_YENIDEN_DENEME && kapatildi && reasoning_zorunlu_hatasi(&govde_metni)
                {
                    log::warn!(
                        "ai [sor_ham_akis]: model reasoning kapatılmasına izin vermiyor, açık yeniden deneniyor"
                    );
                    Self::reasoning_alanlarini_kaldir(&mut govde);
                    kapatildi = false;
                    if Self::butce_tabanini_uygula(&mut govde, REASONING_ZORUNLU_TABAN) {
                        log::warn!(
                            "ai [sor_ham_akis]: küçük bütçe reasoning'e yetmeyebilir, {REASONING_ZORUNLU_TABAN}'a çıkarıldı"
                        );
                    }
                    son_hata = hata;
                    continue;
                }
                if deneme < AI_YENIDEN_DENEME && durum_denenebilir(durum) {
                    son_hata = hata;
                    continue;
                }
                return Err(hata);
            }
            return Ok(AkisOkuyucu {
                cevap,
                tampon: Vec::new(),
                kuyruk: Vec::new(),
                kullanim: Kullanim::default(),
                kategori,
                done: false,
                bitti: false,
            });
        }
        Err(son_hata)
    }

    // sohbet cevabının sistem mesajı: kim konuşuyor, ne konuşuluyor bakıp
    // hafızadan yalnız ilgili parçaları getirir; uret ve uret_akis ortak kullanır
    fn sohbet_sistemi(&self, gecmis: &[Mesaj], talimat: &str) -> (String, String, String) {
        let mut katilimcilar: Vec<String> = Vec::new();
        let mut metinler: Vec<String> = Vec::new();
        for m in gecmis.iter().filter(|m| m.role == "user") {
            match m.content.split_once(": ") {
                Some((isim, metin)) => {
                    // contains için geçici String üretilmez; dilimle karşılaştırılır
                    if !katilimcilar.iter().any(|k| k.as_str() == isim) {
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
    // kategori yalnız token metriğinde kırılım için (!durum), isteğe hiçbir etkisi yok.
    async fn uret(
        &self,
        gecmis: &[Mesaj],
        talimat: &str,
        butce: Option<u32>,
        kategori: &'static str,
    ) -> Result<String, Hata> {
        let (sabit, degisken, bot_adi) = self.sohbet_sistemi(gecmis, talimat);
        let cevap = self
            .sor_bolumlu(&sabit, &degisken, gecmis, butce, kategori)
            .await?;
        Ok(temizle(cevap, &bot_adi))
    }

    // sohbet cevabını akış olarak açar; parçalar geldikçe okuyucudan okunur
    async fn uret_akis(
        &self,
        gecmis: &[Mesaj],
        talimat: &str,
        butce: Option<u32>,
        kategori: &'static str,
    ) -> Result<(AkisOkuyucu, String), Hata> {
        let (sabit, degisken, bot_adi) = self.sohbet_sistemi(gecmis, talimat);
        let okuyucu = self
            .sor_ham_akis(&sabit, &degisken, gecmis, butce, kategori)
            .await?;
        Ok((okuyucu, bot_adi))
    }

    // kişiliksiz, düz analiz: ajanlar bunu kullanır
    async fn analiz(
        &self,
        metin: &str,
        talimat: &str,
        max_tokens: u32,
        kategori: &'static str,
    ) -> Result<String, Hata> {
        let girdi = kullanici(format!("{metin}\n\n---\n\n{talimat}"));
        self.sor(ANALIST, &[girdi], max_tokens, kategori).await
    }

    // "bu konuşmaya katılmak istiyor muyum?" mini değerlendirmesi (0-10 puan).
    // etiket/yanıt her zaman cevaplanır, buraya hiç gelmez; hata durumunda None (yedek zar devrede).
    // profil+dizin sabit blokta (cache_control): ana sohbetteki aynı içerikle örtüşür, 6 saatte
    // bir değişir; her mesajda tekrar tekrar tam fiyatına yollanmasın diye ayrı tutulur.
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
        let sabit = format!("{ANALIST}\n\nGRUP PROFİLİ\n{profil}\n\nKİŞİ DİZİNİ\n{dizin}");
        let degisken = ISTEKLILIK.replace("{ad}", &bot_adi);
        let girdi = kullanici(format!("SON MESAJLAR\n{baglam}"));
        match self
            .sor_bolumlu(&sabit, &degisken, &[girdi], Some(80), "isteklilik")
            .await
        {
            Ok(c) => isteklilik_puan(&c),
            Err(e) => {
                log::debug!("isteklilik: çağrı başarısız: {e}");
                None
            }
        }
    }

    // üst üste farklı kişiler yazınca cevap kime dönsün? bekleyen isimler arasından seçer.
    // talimat sabit blokta (cache_control): profil/dizin içermez ama en azından kendi başına sabit kalır.
    async fn hedef_sec(&self, bekleyenler: &[String]) -> Option<String> {
        let (baglam, bot_adi) = {
            let d = self.durum();
            (son_mesajlar(&d, 12), d.bot_adi.clone())
        };
        let sabit = format!("{ANALIST}\n\n{}", HEDEF_SEC.replace("{ad}", &bot_adi));
        let degisken = format!("BEKLEYENLER\n- {}", bekleyenler.join("\n- "));
        let girdi = kullanici(format!("SON MESAJLAR\n{baglam}"));
        match self
            .sor_bolumlu(&sabit, &degisken, &[girdi], Some(40), "hedef_sec")
            .await
        {
            Ok(c) => hedef_ayikla(&c, bekleyenler),
            Err(e) => {
                log::debug!("hedef_sec: çağrı başarısız: {e}");
                None
            }
        }
    }

    // bu sohbetin ruh halini belirler: ucuz mini çağrı, `cevapla` yalnız sohbet açılırken ve
    // birkaç turda bir çağırır (her mesajda değil). Nötr/düşük yoğunluk ya da hata → None,
    // çağıran taraf bunu "belirgin bir ruh hali yok" olarak ele alır.
    async fn ruh_hali_belirle(&self, gecmis: &[Mesaj]) -> Option<String> {
        if gecmis.is_empty() {
            return None;
        }
        // görsel bu mini çağrıya girmez: 40 token'lık ruh hali analizine tam resim yükü
        // yollamak token yakar, vision desteklemeyen bir route'ta da çağrıyı hataya düşürür
        let gecmis: Vec<Mesaj> = gecmis
            .iter()
            .map(|m| Mesaj {
                resim: None,
                ..m.clone()
            })
            .collect();
        let degisken = RUH_HALI.replace("{ad}", &self.durum().bot_adi);
        match self
            .sor_bolumlu(ANALIST, &degisken, &gecmis, Some(40), "ruh_hali")
            .await
        {
            Ok(c) => ruh_hali_ayikla(&c),
            Err(e) => {
                log::debug!("ruh_hali: çağrı başarısız: {e}");
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
                    if matches!(kip, DusunmeKip::Goster | DusunmeKip::Gizle) {
                        dusunce.push_str(&p.dusunce);
                    }
                    if ilk || son_yazma.elapsed() >= AKIS_DUZENLEME {
                        // soy dilim döndürür: her edit'te metnin tamamı klonlanmaz
                        let yerlesim =
                            akis_gorunum(kip, &dusunce, soy(&metin, baglam.bot_adi), false);
                        // "ilk" ancak gerçekten bir şey yazılınca harcanır: ilk deltalar
                        // yarım satır olduğu için yerleşim boş dönebiliyor, mesaj o zaman
                        // 1,2 sn beklemeden ilk anlamlı içerikle açılsın
                        if !yerlesim.is_empty() {
                            ilk = false;
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
        self.metrik_ekle(okuyucu.kategori, okuyucu.kullanim);
        log::debug!(
            "akis [{kanal}]: parça={parca_sayisi} ilk={ilk_parca_ms:?}ms toplam={}ms done={}",
            baslangic.elapsed().as_millis(),
            okuyucu.done,
        );
        if !okuyucu.done && akis_hatasi.is_none() {
            log::warn!("akis [{kanal}]: [DONE] gelmeden kapandı, yarım kalmış olabilir");
        }

        // yeni mesaj gelse de akış tamamlanır: cevap snapshot'a gider, yeni mesaj
        // sıradaki turda ele alınır (sil-baştan yok)
        let mut cevap = cevap_parcala(soy(&metin, baglam.bot_adi));
        // model susmayı seçti: açılmış geçici mesajlar silinir, kayda hiçbir şey girmez.
        // "tepki: 💀" + "-" birlikte gelirse susma değil: emoji yine de düşmeli
        if cevap.sus && cevap.satirlar.is_empty() && cevap.tepki.is_none() {
            log::debug!("akis [{kanal}]: sus");
            sil_mesajlar(ctx, gonderilenler).await;
            return match akis_hatasi {
                Some(e) => Err(e),
                None => Ok(AkisSonuc::Sus),
            };
        }
        if cevap.bos() {
            sil_mesajlar(ctx, gonderilenler).await;
            return match akis_hatasi {
                Some(e) => Err(e),
                None => Ok(AkisSonuc::Bos),
            };
        }
        // aynı lafı iki kez etmesin: tekrar eden satırlar düşer; hiç söz kalmadıysa ve
        // tepki de yoksa bir kez yeniden üretir, yine tekrarsa susar
        let tekrarlar: Vec<String> = cevap
            .satirlar
            .iter()
            .filter(|s| self.tekrar_mi(kanal, s))
            .cloned()
            .collect();
        if !tekrarlar.is_empty() {
            cevap.satirlar.retain(|s| !tekrarlar.contains(s));
            log::debug!("akis [{kanal}]: {} tekrar satırı düştü", tekrarlar.len());
        }
        if cevap.satirlar.is_empty() && cevap.tepki.is_none() {
            let t2 = format!(
                "{}\n\nAz önce aynen şunu yazdın: \"{}\". Aynısını ya da benzerini yazma; başka bir açıdan gir ya da konuyu değiştir.",
                baglam.talimat,
                tekrarlar.join(" / ")
            );
            let yeni = match self.uret(baglam.gecmis, &t2, baglam.butce, "sohbet").await {
                Ok(y) => cevap_parcala(&y),
                Err(e) => {
                    log::debug!("akis [{kanal}]: tekrar sonrası yeniden üretim başarısız: {e}");
                    Cevap::default()
                }
            };
            // yalnız tepki dönmesi boş sayılmaz: emoji gidecek bir şeydir
            if (yeni.satirlar.is_empty() && yeni.tepki.is_none())
                || yeni.satirlar.iter().any(|s| self.tekrar_mi(kanal, s))
            {
                sil_mesajlar(ctx, gonderilenler).await;
                return Ok(AkisSonuc::Bos);
            }
            cevap = yeni;
        }
        let yerlesim = akis_yerlesim(kip, &dusunce, &cevap.satirlar);
        yaz_akis(ctx, kanal, &mut gonderilenler, &yerlesim, baglam.yanit).await;

        // emoji tepkisi: yazı yerine ya da yazının yanında, cevaplanan mesaja düşer.
        // hata akışı durdurmaz, tepki süs
        if let (Some(emoji), Some(hedef)) = (&cevap.tepki, baglam.tepki_hedefi) {
            if let Err(e) = ctx
                .http
                .create_reaction(kanal, hedef, &ReactionType::Unicode(emoji.clone()))
                .await
            {
                log::warn!("tepki eklenemedi ({kanal}): {e}");
            }
        }

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

        // gönderilenler kayda geçer; thinking kayda girmez, hoca ve eleştirmen yalnız cevabı görür.
        // her satır ayrı bir mesaj olduğu için kayda da ayrı ayrı girer
        let mut d = self.durum();
        for satir in &cevap.satirlar {
            d.kendi_mesajlarim.push_back(satir.clone());
        }
        while d.kendi_mesajlarim.len() > 50 {
            d.kendi_mesajlarim.pop_front();
        }
        let mut notlar: Vec<String> = cevap
            .satirlar
            .iter()
            .map(|s| format!("{}: {s}", baglam.bot_adi))
            .collect();
        if let Some(emoji) = &cevap.tepki {
            // tohum tutarlılığı: model geçmişte kendi protokol biçimini görsün
            notlar.push(format!("{}: tepki: {emoji}", baglam.bot_adi));
        }
        kanal_not_coklu(&mut d, kanal, notlar);
        drop(d);
        if let Some(e) = akis_hatasi {
            log::warn!("akis [{kanal}]: yarıda kesildi, elimizdeki gönderildi: {e}");
        }
        Ok(AkisSonuc::Gonderildi(cevap.protokol_metni()))
    }

    // stream OLMAYAN yolların ortak göndericisi: cevabı protokole göre satırlara böler,
    // her satırı ayrı mesaj olarak sırayla yollar (araya küçük yazma gecikmesi girer,
    // hepsi aynı anda düşmesin). Sus ya da boş cevapta hiçbir şey gitmez, None döner.
    // ping yalnız ilk satıra takılır (hoş geldin mesajı yeni üyeyi etiketler).
    async fn gonder_satirlar(
        &self,
        ctx: &Context,
        kanal: ChannelId,
        ham: &str,
        yanit: Option<MessageId>,
        tepki_hedefi: Option<MessageId>,
        ping: Option<UserId>,
    ) -> Option<String> {
        let bot_adi = self.durum().bot_adi.clone();
        let cevap = cevap_parcala(soy(ham, &bot_adi));
        self.gonder_cevap(ctx, kanal, cevap, yanit, tepki_hedefi, ping)
            .await
    }

    // gonder_satirlar'ın gövdesi, çözülmüş Cevap üzerinden: elinde zaten ayıklanmış
    // (tekrar elenmiş) bir cevap olan yollar metne geri dönüp yeniden çözmesin
    async fn gonder_cevap(
        &self,
        ctx: &Context,
        kanal: ChannelId,
        mut cevap: Cevap,
        yanit: Option<MessageId>,
        tepki_hedefi: Option<MessageId>,
        ping: Option<UserId>,
    ) -> Option<String> {
        let bot_adi = self.durum().bot_adi.clone();
        // tepkinin düşeceği mesaj yoksa (açılış yolları) tepki yok sayılır: yoksa kanala
        // hiçbir şey gitmediği halde "gönderildi" denip sohbet açılıyordu
        if tepki_hedefi.is_none() {
            cevap.tepki = None;
        }
        if cevap.satirlar.is_empty() && cevap.tepki.is_none() {
            log::debug!(
                "gonder_satirlar [{kanal}]: gidecek bir şey yok (sus={})",
                cevap.sus
            );
            return None;
        }
        for (i, satir) in cevap.satirlar.iter().enumerate() {
            if i > 0 {
                let bekle = (SATIR_GECIKME_TABAN
                    + SATIR_GECIKME_HARF * satir.chars().count() as u64)
                    .min(SATIR_GECIKME_TAVAN);
                let _ = kanal.broadcast_typing(&ctx.http).await;
                sleep(Duration::from_millis(bekle)).await;
            }
            let (p, y) = if i == 0 { (ping, yanit) } else { (None, None) };
            // etiket protokol çözüldükten SONRA takılır: metne baştan yapıştırılınca
            // "<@id> -" susma işareti, "<@id> tepki: 💀" da tepki satırı sayılmıyordu
            match p {
                Some(u) => {
                    self.gonder(ctx, kanal, &format!("<@{u}> {satir}"), p, None, y)
                        .await
                }
                None => self.gonder(ctx, kanal, satir, p, None, y).await,
            }
        }
        if let (Some(emoji), Some(hedef)) = (&cevap.tepki, tepki_hedefi) {
            if let Err(e) = ctx
                .http
                .create_reaction(kanal, hedef, &ReactionType::Unicode(emoji.clone()))
                .await
            {
                log::warn!("tepki eklenemedi ({kanal}): {e}");
            }
            // satırları gonder() kaydetti, tepki kaydı burada (protokol biçimiyle)
            let mut d = self.durum();
            kanal_not(&mut d, kanal, format!("{bot_adi}: tepki: {emoji}"));
        }
        Some(cevap.protokol_metni())
    }
}

// akış sürerken cevabın görünen kısmı: tamamlanmış satırlar (ardında \n olan) + son yarım
// satır ancak YARIM_SATIR_ESIGI karakteri geçtiyse. Böylece "tep" yarım hâlde mesaj olup
// bir sonraki düzenlemede silinmez, kısa satır için boşuna edit atılmaz.
fn akis_kesiti(cevap: &str, bitti: bool) -> &str {
    if bitti {
        return cevap;
    }
    let (tam, yarim) = match cevap.rfind('\n') {
        Some(i) => (&cevap[..i], &cevap[i + 1..]),
        None => ("", cevap),
    };
    if yarim.trim().chars().count() >= YARIM_SATIR_ESIGI {
        cevap
    } else {
        tam
    }
}

// thinking fazı: cevap henüz başlamadıysa ve model düşünüyorsa placeholder gider;
// gizlede canlı kelime sayacı, göstergede düz "Düşünüyorum...". Sessiz ve kapalıda hiçbir
// şey gitmez (sessizde reasoning arka planda çalışır ama iz bırakmaz, kapalıda zaten yok).
// Cevap başlayınca aynı mesaj düzenlenerek stream edilir. bitti=false ise akış sürüyor:
// yalnız tamamlanmış satırlar gösterilir.
fn akis_gorunum(kip: DusunmeKip, dusunce: &str, cevap: &str, bitti: bool) -> Vec<String> {
    let kesit = akis_kesiti(cevap, bitti);
    if kesit.trim().is_empty() && !dusunce.trim().is_empty() {
        return match kip {
            DusunmeKip::Gizle => vec![dusunce_sayaci(dusunce)],
            DusunmeKip::Goster => vec!["Düşünüyorum...".to_string()],
            DusunmeKip::Sessiz | DusunmeKip::Kapali => Vec::new(),
        };
    }
    akis_yerlesim(kip, dusunce, &cevap_parcala(kesit).satirlar)
}

// düşünce blokları + satır mesajları. Satırlar dışarıdan gelir: son yerleşimde
// gonder_akis tekrar elemesinden geçmiş hâllerini verir
fn akis_yerlesim(kip: DusunmeKip, dusunce: &str, satirlar: &[String]) -> Vec<String> {
    let dusunce = tek_satir(dusunce);
    let mut v: Vec<String> = Vec::new();
    if kip == DusunmeKip::Goster && !dusunce.is_empty() {
        // göster: hem spoiler hem kod bloğu
        for p in bol(&dusunce, MESAJ_SINIRI - 4) {
            v.push(spoiler(&p));
        }
        v.extend(kod_bloklari(&dusunce));
    }
    // gizle/sessiz/kapalı kiplerde düşünce yerleşime girmez; gizlede cevap sonunda
    // "Düşünce Sürecini Göster" butonu gider (gonder_akis ekler), sessizde hiç buton yok
    v.extend(satirlar.iter().cloned());
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
    // typing edit döngüsünde yinelenmez: her tur atmak discord hız sınırına takılır;
    // "yazıyor" göstergesini cevapla, model çağrısından önce bir kez yollar
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

// cache_control anthropic'in uydurduğu bir alan. OpenRouter'a giden her istekte güvenle
// eklenebilir: openrouter kendi birleşik şemasının bir parçası olarak kabul eder, hangi modelde
// gerçekten önbellekleyeceğine (claude, gemini, ...) kendi tarafında karar verir, desteklemeyen
// modelde sessizce yok sayar — model adını burada tahmin etmeye gerek yok. Mistral'in native
// API'si ya da `API_ADRES` ile verilen özel bir router aynı garantiyi vermez: bilinmeyen alanla
// isteği tümden reddedebilir, o yüzden yalnız openrouter adresine gidiyorsa eklenir.
fn onbellek_destekler(api_adres: &str) -> bool {
    api_adres.contains("openrouter.ai")
}

// sistem mesajını OpenAI uyumlu bloğa çevirir: değişken boşsa düz metin, değilse sabit+değişken
// iki metin bloğu; sabit blok yalnız openrouter'a giderken cache_control ile işaretlenir
fn sistem_json(sabit: &str, degisken: &str, api_adres: &str) -> serde_json::Value {
    if degisken.is_empty() {
        return serde_json::json!({ "role": "system", "content": sabit });
    }
    let mut ilk = serde_json::json!({ "type": "text", "text": sabit });
    if onbellek_destekler(api_adres) {
        ilk["cache_control"] = serde_json::json!({ "type": "ephemeral" });
    }
    serde_json::json!({ "role": "system", "content": [
        ilk,
        { "type": "text", "text": degisken }
    ]})
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
        // Açılış zaten gönderildi ve kanal geçmişine SATIR SATIR düştü (her satır ayrı
        // mesaj); tohumun sonundaki bot bloğunda o satırlar temizlenir, yoksa model kendi
        // açılışını hem tek tek hem de birleşik hâlde iki kez görür. Araya haber linki
        // gibi başka bir bot mesajı girmiş olabilir, o yüzden blok boyunca taranır.
        let parcalar: Vec<&str> = a.split('\n').map(str::trim).collect();
        let mut i = s.gecmis.len();
        while i > 0 && s.gecmis[i - 1].role == "assistant" {
            i -= 1;
            if parcalar.contains(&s.gecmis[i].content.trim()) {
                s.gecmis.remove(i);
            }
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
                    }
                }
            }
            // üst üste soru sormasın: tavanı kod ölçer, uygulamayı model yapar
            if soru_fazla_mi(&self.durum(), kanal) {
                talimat = format!("{talimat}\n\nBu sefer soru sorma; düz laf et ya da sus.");
                log::debug!("cevapla [{kanal}]: soru tavanı doldu");
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
            // süresi dolan haber sohbetleri: kimse yorum yazmadıysa (aktivite pencere
            // boyunca yok ya da kayıt zaten düşmüş) sessizce kapanır, harita şişmez
            let haber_dolen: Vec<ChannelId> = d
                .haber_bekleyen
                .iter()
                .filter(|(k, t)| {
                    simdi >= **t
                        && d.son_aktivite
                            .get(k)
                            .is_none_or(|a| simdi.duration_since(*a) >= YORUM_SURESI)
                })
                .map(|(k, _)| *k)
                .collect();
            for kanal in haber_dolen {
                d.haber_bekleyen.remove(&kanal);
                d.sohbetler.remove(&kanal);
                d.son_aktivite.remove(&kanal);
                log::debug!("haber [{kanal}]: yorum gelmedi, sohbet sessizce kapandı");
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
            .uret(
                &[kullanici("isim seçme vakti")],
                ISIM_SEC,
                Some(12),
                "isim_sec",
            )
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
                "isim_duyuru",
            )
            .await
        {
            Ok(duyuru) => match self
                .gonder_satirlar(ctx, kanal, &duyuru, None, None, None)
                .await
            {
                Some(p) => {
                    sohbet_baslat(&mut self.durum(), kanal, Some(p));
                }
                None => log::debug!("isim: duyuruda model sustu, atlandı"),
            },
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
async fn haber_dongusu(bot: Arc<Bot>, ctx: Context) {
    loop {
        sleep(HABER_ARALIGI).await;
        if KAPANIYOR.load(Ordering::SeqCst) {
            return;
        }
        if !uyku::uyanik_mi(&bot.durum()) {
            // uykuda haber atılmaz ama seçilir: uyanınca "sabah haberi" olarak stoklanır
            let stok_bos = bot.durum().stok_haber.is_none();
            if stok_bos {
                match bot.haberci().await {
                    Ok(h) => {
                        bot.durum().stok_haber = Some(h);
                        log::debug!("haber: uykuda stoklandı");
                    }
                    Err(e) => log::debug!("haber: uyku stoku seçilemedi: {e}"),
                }
            }
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

        // uykuda stoklanan haber varsa önce o gider ("sabah haberi")
        let stok = bot.durum().stok_haber.take();
        match stok {
            Some(h) => {
                bot.haber_gonder(&ctx, kanal, h).await;
            }
            None => {
                bot.haber_at(&ctx, kanal).await;
            }
        }
    }
}

impl Bot {
    // küçük, uydurma ama inandırıcı bir yazılım derdi atar, "nasıl çözerim" diye sorar
    async fn sorun_at(&self, ctx: &Context, kanal: ChannelId) {
        let son = son_mesajlar(&self.durum(), 30);
        match self
            .uret(&[kullanici(son)], SORUN, Some(160), "sorun")
            .await
        {
            Ok(laf) => match self
                .gonder_satirlar(ctx, kanal, &laf, None, None, None)
                .await
            {
                Some(p) => {
                    sohbet_baslat(&mut self.durum(), kanal, Some(p));
                }
                None => log::debug!("sorun: model sustu, atlandı"),
            },
            Err(e) => log::error!("ai [sorun]: {e}"),
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
        self.haber_gonder(ctx, kanal, h).await
    }

    // seçilmiş haberi paylaşır: tur haberi de uykuda stoklanan da buradan gider
    async fn haber_gonder(&self, ctx: &Context, kanal: ChannelId, h: ajanlar::Haber) -> bool {
        let link = if h.url.starts_with("https://") || h.url.starts_with("http://") {
            h.url.clone()
        } else {
            format!("https://news.ycombinator.com/item?id={}", h.id)
        };
        let girdi = match self
            .uret(
                &[kullanici(h.title.clone())],
                HABER_TANIT,
                Some(200),
                "haber_tanit",
            )
            .await
        {
            Ok(g) => g,
            Err(e) => {
                log::error!("ai [haber_tanit]: {e}");
                return false;
            }
        };
        // tanıtım satır satır gider, link ayrı mesaj olarak peşinden (insanlar da öyle atar)
        let Some(protokol) = self
            .gonder_satirlar(ctx, kanal, &girdi, None, None, None)
            .await
        else {
            log::debug!("haber: tanıtımda model sustu, haber atlandı");
            return false;
        };
        self.gonder(ctx, kanal, &link, None, None, None).await;

        let mut d = self.durum();
        sohbet_baslat(&mut d, kanal, Some(protokol));
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
            self.uret(
                &[kullanici("şaka başlıyor")],
                HACK_GIRIS,
                Some(150),
                "hack_giris",
            )
            .await
        } else {
            self.resimci(&resim).await
        };
        let metin = match metin {
            Ok(m) => m,
            Err(e) => {
                log::error!("ai [saka]: {e}");
                return;
            }
        };
        // görsel tek mesajda gider: protokolden yalnız ilk satır alınır, süsler temizlenir.
        // soy() diğer bütün yollarda olduğu gibi burada da uygulanır (ad öneki, tırnak)
        let bot_adi = self.durum().bot_adi.clone();
        let cevap = cevap_parcala(soy(&metin, &bot_adi));
        let Some(ilk) = cevap.satirlar.first().cloned() else {
            log::debug!("saka: model sustu, şaka atlandı");
            return;
        };
        self.gonder(ctx, kanal, &ilk, None, Some(&resim), None)
            .await;

        let mut d = self.durum();
        let s = sohbet_baslat(&mut d, kanal, Some(ilk));
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
        if KAPANIYOR.load(Ordering::SeqCst) {
            return;
        }
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

        let laf = match bot.uret(&[kullanici(son)], talimat, Some(120), "laf").await {
            Ok(l) => l,
            Err(e) => {
                log::error!("ai [durtme]: {e}");
                continue;
            }
        };
        match bot
            .gonder_satirlar(&ctx, kanal, &laf, None, None, None)
            .await
        {
            Some(p) => {
                sohbet_baslat(&mut bot.durum(), kanal, Some(p));
            }
            None => log::debug!("durtme: model sustu, atlandı"),
        }
    }
}

// arada bir resimler/ klasöründen bir görsel atar; bazen de hacklenmiş taklidiyle
async fn saka_dongusu(bot: Arc<Bot>, ctx: Context) {
    loop {
        sleep(SAKA_ARALIGI).await;
        if KAPANIYOR.load(Ordering::SeqCst) {
            return;
        }
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
        if KAPANIYOR.load(Ordering::SeqCst) {
            return;
        }
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
        if KAPANIYOR.load(Ordering::SeqCst) {
            return;
        }
        // uykuda mesajlar zihne işlenmeye devam eder: 2 saatte bir gece gözlemi kuyruğa düşer
        {
            let mut d = bot.durum();
            if !uyku::uyanik_mi(&d) && simdi_unix() - d.son_gece_gozlem >= 2 * 3600 {
                let son = son_mesajlar(&d, 300);
                d.son_gece_gozlem = simdi_unix();
                d.bellek_kuyruk.push_back((
                    son,
                    "gece gözlemi (bot uykuda)".to_string(),
                    "gece".to_string(),
                    false,
                ));
            }
        }
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
        if KAPANIYOR.load(Ordering::SeqCst) {
            return;
        }
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
        let (bekleyen, uyandi) = {
            let mut d = self.durum();
            let uyanik = uyku::uyanik_mi(&d);
            if uyanik == d.uyuyor {
                log::info!("uyku: {}", if uyanik { "uyandı" } else { "uyudu" });
            }
            let uyandi = uyanik && d.uyuyor;
            let uyudu = !uyanik && !d.uyuyor;
            d.uyuyor = !uyanik;
            if uyudu {
                // gece mesajlarını kesebilmek için başlangıç işaretleri
                d.uyku_basi = simdi_unix();
                d.uyku_basi_hafiza_len = d.hafiza.len();
            }
            if uyandi {
                (std::mem::take(&mut d.bekleyen_etiketler), true)
            } else {
                (Vec::new(), false)
            }
        };

        // geçiş yoksa bu tikte uyanış işi yok
        if !uyandi {
            return;
        }

        if !bekleyen.is_empty() {
            // uyurken etiketlendiyse kesin dönüş: liste hata durumunda geri konur
            let Some(&(kanal, _)) = bekleyen.last() else {
                return;
            };
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
                    "uyandim",
                )
                .await
            {
                Ok(c) => match self.gonder_satirlar(ctx, kanal, &c, None, None, None).await {
                    Some(p) => {
                        sohbet_baslat(&mut self.durum(), kanal, Some(p));
                    }
                    None => log::debug!("uyandim: model sustu, atlandı"),
                },
                Err(e) => {
                    log::error!("ai [uyandim]: {e}");
                    let mut d = self.durum();
                    for b in bekleyen {
                        d.bekleyen_etiketler.push(b);
                    }
                }
            }
            return;
        }

        // etiket yoksa gece yazılanları değerlendir: ilgini çeken varsa sabah sözüyle dön
        let gece: Vec<String> = {
            let d = self.durum();
            d.hafiza
                .iter()
                .skip(d.uyku_basi_hafiza_len)
                .cloned()
                .collect()
        };
        if !gece.is_empty() {
            self.uyanis_degerlendir(ctx, &gece).await;
        }
    }

    // gece yazılanlardan botu ilgilendirenleri seçer; eşiğin üstündeyse sabah sözüyle döner
    async fn uyanis_degerlendir(&self, ctx: &Context, gece: &[String]) {
        let gece_metni = gece.join("\n");
        let talimat = {
            let d = self.durum();
            UYANIS.replace("{ad}", &d.bot_adi)
        };
        #[derive(Deserialize)]
        struct UyanisSonuc {
            #[serde(default)]
            ilgi: i32,
            #[serde(default)]
            konu: String,
        }
        let sonuc = match self.analiz(&gece_metni, &talimat, 100, "uyanis").await {
            Ok(c) => serde_json::from_str::<UyanisSonuc>(json_ayikla(&c)),
            Err(e) => {
                log::debug!("uyanis: değerlendirme çağrısı başarısız: {e}");
                return;
            }
        };
        let Ok(s) = sonuc else {
            log::debug!("uyanis: sonuç çözülemedi");
            return;
        };
        log::debug!("uyanis: ilgi={} konu={}", s.ilgi, s.konu);
        if s.ilgi < 5 {
            return;
        }
        let Some(kanal) = self.durum().son_kanal else {
            return;
        };
        if self.durum().sohbetler.contains_key(&kanal) {
            return;
        }
        let talimat = {
            let d = self.durum();
            UYANIS_CEVAP
                .replace("{ad}", &d.bot_adi)
                .replace("{konu}", &s.konu)
        };
        match self
            .uret(
                &[kullanici(format!("sen uyurken yazılanlar:\n{gece_metni}"))],
                &talimat,
                Some(250),
                "uyanis_cevap",
            )
            .await
        {
            Ok(c) => match self.gonder_satirlar(ctx, kanal, &c, None, None, None).await {
                Some(p) => {
                    sohbet_baslat(&mut self.durum(), kanal, Some(p));
                }
                None => log::debug!("uyanis_cevap: model sustu, atlandı"),
            },
            Err(e) => log::error!("ai [uyanis_cevap]: {e}"),
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
        // ekteki ilk görsel modele gider; sırf resim atılmış mesaj da (metni boş) işlenir
        let resim = msg
            .attachments
            .iter()
            .find(|e| {
                e.content_type
                    .as_deref()
                    .is_some_and(|t| t.starts_with("image/"))
            })
            .map(|e| e.url.clone());
        if ham_metin.trim().is_empty() && resim.is_none() {
            return;
        }
        // komutlar: ! ya da / ile başlar; bilinmeyen komut normal mesaj sayılır.
        // Süslenmemiş HAM metne bakılır: resim ekli mesajda metin "[resim] !durum" oluyor
        // ve komut sessizce yutuluyordu
        let kelime = ham_metin.trim();
        if kelime.starts_with('!') || kelime.starts_with('/') {
            let mut parcalar = kelime[1..].split_whitespace();
            let komut = parcalar.next().unwrap_or("").to_lowercase();
            let arg = parcalar.collect::<Vec<_>>().join(" ");
            if self.bot.komut(&ctx, &msg, &komut, &arg).await {
                return;
            }
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
        let (dogrudan_cevapla, etiketlendi, degerlendir) = {
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
            (etiketlendi || devam_eden_diyalog, etiketlendi, degerlendir)
        };

        // 2. faz (kilitsiz): isteklilik değerlendirmesi — her mesaja atlamaz,
        // konu/personalık/ilgi tartılır; etiket ve sürmekte olan diyalog zaten doğrudan cevaplanır
        let mut katil = dogrudan_cevapla;
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

        if cevap_ver {
            self.bot.cevapla(&ctx, kanal).await;
        }
    }

    // slash komutlar embed kart açar (/durum /yardim /zihin); zihin kartındaki
    // menü/butonlar detay modallarına götürür; modal gönderimleri kısa onay alır
    // (gösterimlik, girdi toplanmaz); düşünce butonu eski akışında
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(c) => {
                let yanit = {
                    let d = self.bot.durum();
                    match c.data.name.as_str() {
                        "durum" => modal::durum_mesaji(&d),
                        "zihin" => modal::zihin_mesaji(&d),
                        _ => modal::yardim_mesaji(), // "yardim" ya da bilinmeyen
                    }
                };
                if let Err(e) = c
                    .create_response(&ctx.http, CreateInteractionResponse::Message(yanit))
                    .await
                {
                    log::warn!("komut kartı gönderilemedi [{}]: {e}", c.data.name);
                }
            }
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

impl Handler {
    // gizle kipindeki "Düşünce Sürecini Göster" butonu: düşünenin deposundan
    // bulur, yalnız tıklayana görünen ephemeral kod bloğu olarak açar
    async fn dusunce_dugmesi(&self, ctx: &Context, c: ComponentInteraction) {
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

impl Bot {
    // Sağlayıcıyı ortam değişkenlerinden seçer (.env'i main yükler), durum klasörlerini
    // açar, diskteki durumu yükler ve botu kurar. Discord'a bağlanmaz, token istemez:
    // hem main hem CLI sohbet modu buradan geçer.
    fn kur() -> Result<Arc<Bot>, Hata> {
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
        // ikisi de isteğe bağlı: ayarlanmazsa bot eskisi gibi eriştiği her sunucuda/kanalda çalışır
        let guild_id = match std::env::var("GUILD_ID") {
            Ok(v) if !v.trim().is_empty() => Some(GuildId::new(v.trim().parse()?)),
            _ => None,
        };
        let izinli_kanallar = match std::env::var("KANALLAR") {
            Ok(v) if !v.trim().is_empty() => {
                let mut s = HashSet::new();
                for parca in v.split(',') {
                    let parca = parca.trim();
                    if parca.is_empty() {
                        continue;
                    }
                    s.insert(ChannelId::new(parca.parse()?));
                }
                Some(s)
            }
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
        Ok(Arc::new(Bot {
            durum: Mutex::new(durum),
            // bağlantı kurma hızlı elensin (BAGLANTI_ZAMAN_ASIMI); toplam süre sınırı yok, yalnız
            // iki veri arası en çok OKUMA_ZAMAN_ASIMI (P0: eskiden tek 60sn'lik timeout uzun
            // düşünme akışını ortasında kesebiliyordu)
            http: reqwest::Client::builder()
                .connect_timeout(BAGLANTI_ZAMAN_ASIMI)
                .read_timeout(OKUMA_ZAMAN_ASIMI)
                .build()?,
            api_adres,
            anahtar,
            haber_kanali,
            firecrawl: std::env::var("FIRECRAWL_KEY")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            guild_id,
            izinli_kanallar,
        }))
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
    // `cargo run -- sohbet`: discord'a hiç bağlanmadan terminalden konuşma tezgâhı.
    // Token istemez, yalnız model anahtarı gerekir; anahtar yoksa tek satır hata + çıkış 1
    if std::env::args().nth(1).as_deref() == Some("sohbet") {
        let bot = match Bot::kur() {
            Ok(b) => b,
            Err(e) => {
                eprintln!("sohbet modu açılamadı: {e}");
                std::process::exit(1);
            }
        };
        bot.sohbet_cli().await;
        return Ok(());
    }
    let token = ayar("DISCORD_TOKEN")?;
    let bot = Bot::kur()?;

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
        // döngüler yeni tur açmasın, bekçi yeniden başlatmasın
        KAPANIYOR.store(true, Ordering::SeqCst);
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
    fn bol_cok_baytlida_sinirda_paniklemez() {
        // kesim_noktasi bayt ofseti döndürür; çok baytlı karakter sınırda bile
        // dilim char hizasında kalır, parça birleşince aslına eşittir
        let m = "üğşçöı ğüşçöı ğüşçöı ğüşçöı ğüşçöı";
        let p = bol(m, 8);
        for parca in &p {
            assert!(parca.chars().count() <= 8);
        }
        assert_eq!(p.join(" ").replace(' ', ""), m.replace(' ', ""));
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
        let v = akis_gorunum(DusunmeKip::Goster, &dusunce, &cevap, true);
        assert!(v.len() >= 5);
        for (i, m) in v.iter().enumerate() {
            assert!(m.chars().count() <= MESAJ_SINIRI, "parça {i} çok uzun");
        }
        // önce spoiler blokları, sonra kod blokları, en sonda cevap parçaları
        assert!(v[0].starts_with("||") && v[0].ends_with("||"));
        assert!(v.iter().any(|m| m.starts_with("```")));
        assert!(!v[v.len() - 1].starts_with("||"));
        // gizle: düşünce yerleşime hiç girmez
        let v = akis_gorunum(DusunmeKip::Gizle, &dusunce, &cevap, true);
        assert!(v
            .iter()
            .all(|m| !m.starts_with("||") && !m.starts_with("```")));
    }

    #[test]
    fn gorunum_dusuncesiz() {
        let v = akis_gorunum(DusunmeKip::Goster, "", "kısa cevap", true);
        assert_eq!(v, vec!["kısa cevap"]);
    }

    #[test]
    fn dusunme_kip_ayristirilir() {
        assert_eq!(DusunmeKip::arg_ile("göster"), Some(DusunmeKip::Goster));
        assert_eq!(DusunmeKip::arg_ile("aç"), Some(DusunmeKip::Goster));
        assert_eq!(DusunmeKip::arg_ile("gizle"), Some(DusunmeKip::Gizle));
        assert_eq!(DusunmeKip::arg_ile("sessiz"), Some(DusunmeKip::Sessiz));
        assert_eq!(DusunmeKip::arg_ile("kapat"), Some(DusunmeKip::Kapali));
        assert_eq!(DusunmeKip::arg_ile("kapalı"), Some(DusunmeKip::Kapali));
        assert_eq!(DusunmeKip::arg_ile("saçma"), None);
        assert_eq!(DusunmeKip::Goster.dosya_degeri(), "goster");
        assert_eq!(DusunmeKip::Sessiz.dosya_degeri(), "sessiz");
    }

    #[test]
    fn gorunum_dusunurken_placeholder() {
        // göster: düz placeholder
        let v = akis_gorunum(DusunmeKip::Goster, "hmm düşünüyorum", "", true);
        assert_eq!(v, vec!["Düşünüyorum..."]);
        // gizle: canlı kelime sayacı
        let v = akis_gorunum(DusunmeKip::Gizle, "bir iki üç dört beş", "", true);
        assert_eq!(v, vec!["Düşünüyorum... Şu ana kadar 5 kelime düşündüm."]);
        // sessiz: arka planda düşünür ama placeholder yok (kapalıyla aynı görünüm)
        let v = akis_gorunum(DusunmeKip::Sessiz, "hmm düşünüyorum", "", true);
        assert!(v.is_empty());
        // kapalıyken placeholder yok
        let v = akis_gorunum(DusunmeKip::Kapali, "", "", true);
        assert!(v.is_empty());
    }

    #[test]
    fn gorunum_cevap_basladi() {
        // göster: thinking hem spoiler hem kod bloğu + cevap
        let v = akis_gorunum(DusunmeKip::Goster, "düşündüm", "cevap bu", true);
        assert_eq!(v.len(), 3);
        assert!(v[0].starts_with("||") && v[0].ends_with("||"));
        assert!(v[1].starts_with("```"));
        assert_eq!(v[2], "cevap bu");
        // gizle: yalnız cevap (butonu gonder_akis ekler)
        let v = akis_gorunum(DusunmeKip::Gizle, "düşündüm", "cevap bu", true);
        assert_eq!(v, vec!["cevap bu"]);
        // sessiz: yalnız cevap, hiç iz yok (buton da eklenmez)
        let v = akis_gorunum(DusunmeKip::Sessiz, "düşündüm", "cevap bu", true);
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
    fn hedef_ayiklanir() {
        let bilinenler = vec!["Emin".to_string(), "Zeynep".to_string()];
        assert_eq!(
            hedef_ayikla(r#"{"hedef": "Zeynep"}"#, &bilinenler),
            Some("Zeynep".into())
        );
        // düz metin de olur, bilinen adla eşleşir
        assert_eq!(hedef_ayikla("emin", &bilinenler), Some("Emin".into()));
        // bilinmeyen ad olduğu gibi döner
        assert_eq!(
            hedef_ayikla(r#"{"hedef": "Misafir"}"#, &bilinenler),
            Some("Misafir".into())
        );
        assert_eq!(hedef_ayikla("", &bilinenler), None);
    }

    #[test]
    fn butce_taban_altindaysa_yukselir() {
        let mut govde = serde_json::json!({"max_tokens": 20});
        assert!(Bot::butce_tabanini_uygula(&mut govde, 500));
        assert_eq!(govde["max_tokens"], 500);
        // taban üstündeyse dokunmaz
        let mut govde = serde_json::json!({"max_tokens": 800});
        assert!(!Bot::butce_tabanini_uygula(&mut govde, 500));
        assert_eq!(govde["max_tokens"], 800);
        // max_tokens hiç yoksa (bütçesiz çağrı) dokunmaz
        let mut govde = serde_json::json!({});
        assert!(!Bot::butce_tabanini_uygula(&mut govde, 500));
        assert!(govde.get("max_tokens").is_none());
    }

    #[test]
    fn reasoning_zorunlu_hatasi_tanir() {
        assert!(reasoning_zorunlu_hatasi(
            r#"{"error":{"message":"Reasoning is mandatory for this endpoint and cannot be disabled.","code":400}}"#
        ));
        assert!(!reasoning_zorunlu_hatasi(
            r#"{"error":{"message":"model not found","code":404}}"#
        ));
        assert!(!reasoning_zorunlu_hatasi("rate limit exceeded"));
    }

    #[test]
    fn ruh_hali_ayiklanir() {
        assert_eq!(
            ruh_hali_ayikla(r#"{"durum": "kafa karışıklığı", "yogunluk": 6}"#),
            Some("kafa karışıklığı (6)".into())
        );
        // clamp: 15 -> 10
        assert_eq!(
            ruh_hali_ayikla(r#"{"durum": "öfke", "yogunluk": 15}"#),
            Some("öfke (10)".into())
        );
        // düşük yoğunluk: nötr sayılır, hiç yansıtılmaz
        assert_eq!(
            ruh_hali_ayikla(r#"{"durum": "huzur", "yogunluk": 2}"#),
            None
        );
        assert_eq!(ruh_hali_ayikla(r#"{"durum": "", "yogunluk": 8}"#), None);
        assert_eq!(ruh_hali_ayikla("bozuk cevap"), None);
    }

    #[test]
    fn onbellek_destek_openrouter_adresine_gore() {
        // openrouter'a giden her istek: model claude/gemini/gpt/glm/grok fark etmez,
        // openrouter kendi tarafında hangisinde işe yarayacağına karar verir
        assert!(onbellek_destekler(
            "https://openrouter.ai/api/v1/chat/completions"
        ));
        // mistral'in native api'si ve özel bir router (API_ADRES) aynı garantiyi vermez
        assert!(!onbellek_destekler(
            "https://api.mistral.ai/v1/chat/completions"
        ));
        assert!(!onbellek_destekler(
            "http://localhost:8080/v1/chat/completions"
        ));
    }

    #[test]
    fn sistem_json_onbellek_yalniz_openrouterda() {
        let or = "https://openrouter.ai/api/v1/chat/completions";
        let claude = sistem_json("sabit", "degisken", or);
        assert!(claude["content"][0]["cache_control"].is_object());
        // openrouter üzerinden gpt/glm/grok da işaretlenir, karar openrouter'a bırakılır
        let glm = sistem_json("sabit", "degisken", or);
        assert!(glm["content"][0]["cache_control"].is_object());
        let mistral = sistem_json(
            "sabit",
            "degisken",
            "https://api.mistral.ai/v1/chat/completions",
        );
        assert!(mistral["content"][0]["cache_control"].is_null());
        // değişken boşsa adres ne olursa olsun düz metin (blok yok)
        let duz = sistem_json("sabit", "", or);
        assert!(duz["content"].is_string());
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
        // ikisinde de bir tavan var artık; debug'daki release'den küçük/eşit olmalı
        assert!(b.is_some_and(|t| t <= CEVAP_TAVANI));
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
            kategori: "test",
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

    #[test]
    fn soy_ad_onekini_alir() {
        assert_eq!(soy("cicikus: selam", "cicikus"), "selam");
        // büyük/küçük harf duyarsız
        assert_eq!(soy("Cicikus: selam", "cicikus"), "selam");
        // önek yoksa metin aynen kalır
        assert_eq!(soy("selam", "cicikus"), "selam");
    }

    #[test]
    fn soy_turkce_adlarda_paniklemez() {
        // İ→i̇ lowercase'de bayt sayısı değişir; bayt dilimi burada paniklerdi
        assert_eq!(soy("Çöp: selam", "çöp"), "selam");
        assert_eq!(soy("İsim: merhaba", "İsim"), "merhaba");
        assert_eq!(soy("ŞAHİN: tamam", "şahin"), "tamam");
    }

    #[test]
    fn soy_tirnak_soyar() {
        assert_eq!(soy("\"selam\"", "bot"), "selam");
        assert_eq!(soy("\"", "bot"), "\""); // tek tırnak kalır
        assert_eq!(soy("\"selam", "bot"), "\"selam"); // kapanmamışsa dokunma
    }

    #[test]
    fn soy_birlesik_kalip() {
        assert_eq!(soy("bot: \"selam dünya\"", "bot"), "selam dünya");
    }

    #[test]
    fn cevap_satirlara_bolunur() {
        let c = cevap_parcala("ilk satır\n\nikinci satır");
        assert_eq!(c.satirlar, vec!["ilk satır", "ikinci satır"]);
        assert!(c.tepki.is_none() && !c.sus);
        assert!(cevap_parcala("").bos());
    }

    #[test]
    fn cevap_tepki_ayiklar() {
        assert_eq!(cevap_parcala("tepki: 💀").tepki.as_deref(), Some("💀"));
        // büyük harf ve araya boşluk da tanınır
        assert_eq!(cevap_parcala("Tepki : 💀").tepki.as_deref(), Some("💀"));
        // emojiden sonrası atılır, satır mesaj olarak gitmez
        let c = cevap_parcala("tepki: 😂 aynen");
        assert_eq!(c.tepki.as_deref(), Some("😂"));
        assert!(c.satirlar.is_empty());
        // özel emoji biçimi çözülmez ama satır yine mesaj olmaz
        let c = cevap_parcala("tepki: :kekw:");
        assert!(c.tepki.is_none() && c.satirlar.is_empty());
        // tepki ile laf birlikte olabilir; ilk tepki kazanır
        let c = cevap_parcala("hahaha\ntepki: 💀\ntepki: 😂");
        assert_eq!(c.satirlar, vec!["hahaha"]);
        assert_eq!(c.tepki.as_deref(), Some("💀"));
        // içinde iki nokta geçen düz satır tepki sanılmaz
        assert_eq!(cevap_parcala("saat 3: gidiyoruz").satirlar.len(), 1);
        // "yazı gitmesin ama gülelim": sus işareti tepkiyi düşürmez
        let c = cevap_parcala("tepki: 💀\n-");
        assert!(c.sus && c.satirlar.is_empty());
        assert_eq!(c.tepki.as_deref(), Some("💀"));
    }

    #[test]
    fn tepki_emoji_olmayani_almaz() {
        // tipografik işaretler emoji değil: discord'a gidince istek 400 ile dönüyordu
        for satir in ["tepki: —", "tepki: …", "tepki: →", "tepki: ¯\\_(ツ)_/¯"] {
            assert!(
                cevap_parcala(satir).tepki.is_none(),
                "{satir} emoji sayıldı"
            );
        }
        // tırnak içindeki emoji yine bulunur, tırnak dizinin başına takılmaz
        assert_eq!(cevap_parcala("tepki: “👍”").tepki.as_deref(), Some("👍"));
        // varyasyon seçicili ve semboller bloğundaki emoji geçer
        assert_eq!(cevap_parcala("tepki: ⭐").tepki.as_deref(), Some("⭐"));
    }

    #[test]
    fn dokum_her_bot_satirina_onek_koyar() {
        // bot cevabı çok satırlı: alt satırlar da bota ait, eleştirmen insana saymasın
        let g = vec![kullanici("emin: naber"), asistan("iyidir\ntepki: 💀")];
        assert_eq!(
            dokum(&g, "kaju"),
            "emin: naber\nkaju: iyidir\nkaju: tepki: 💀"
        );
    }

    #[test]
    fn acilis_tohuma_iki_kez_girmez() {
        let kanal = ChannelId::new(7);
        let mut d = Durum {
            bot_adi: "kaju".into(),
            ..Durum::default()
        };
        // açılış satır satır gönderildi, araya link mesajı da girdi (diske dokunmadan kur)
        let g: VecDeque<String> = ["emin: selam", "kaju: bir", "kaju: iki", "kaju: https://a.b"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        d.kanal_gecmisi.insert(kanal, g);
        let s = sohbet_baslat(&mut d, kanal, Some("bir\niki".to_string()));
        let dizi: Vec<&str> = s.gecmis.iter().map(|m| m.content.as_str()).collect();
        assert_eq!(dizi, vec!["emin: selam", "https://a.b", "bir\niki"]);
    }

    #[test]
    fn cevap_sus_isareti() {
        let c = cevap_parcala("-");
        assert!(c.sus && c.satirlar.is_empty() && !c.bos());
        assert!(cevap_parcala("\"-\"").sus);
        assert!(cevap_parcala("'-'").sus);
        assert!(cevap_parcala("[sus]").sus);
        assert!(cevap_parcala("(sus)").sus);
        assert!(!cevap_parcala("yok artık").sus);
    }

    #[test]
    fn cevap_patlama_siniri() {
        let c = cevap_parcala("bir\niki\nüç\ndört\nbeş");
        assert_eq!(c.satirlar.len(), PATLAMA_SINIRI);
        assert_eq!(c.satirlar.last().unwrap(), "dört");
    }

    #[test]
    fn cevap_kirinti_gider_kisa_kalir() {
        // önceki mesajın kırıntısı atılır
        assert!(cevap_parcala("'cım").satirlar.is_empty());
        // kısa satır artık elenmiyor: "he", "yok", "la" doğal tepkidir
        assert_eq!(cevap_parcala("he").satirlar, vec!["he"]);
        assert_eq!(cevap_parcala("yok\nla").satirlar, vec!["yok", "la"]);
    }

    #[test]
    fn slop_onekleri_silinir() {
        assert_eq!(slop_temizle("- madde"), "madde");
        assert_eq!(slop_temizle("* madde"), "madde");
        assert_eq!(slop_temizle("• madde"), "madde");
        assert_eq!(slop_temizle("**kalın** laf"), "kalın laf");
        assert_eq!(slop_temizle("__altı__ çizili"), "altı çizili");
        // backtick dokunulmaz, kod parçası bilgi taşır — İÇİ de korunur
        assert_eq!(slop_temizle("`kod` çalışmıyor"), "`kod` çalışmıyor");
        assert_eq!(
            slop_temizle("`__init__` fonksiyonu"),
            "`__init__` fonksiyonu"
        );
        // numara gibi duran gerçek sayı bozulmaz
        assert_eq!(slop_temizle("3.14 sayısı"), "3.14 sayısı");
        // parçalama da aynı temizliği uygular
        assert_eq!(cevap_parcala("- bir\n- iki").satirlar, vec!["bir", "iki"]);
    }

    #[test]
    fn numara_oneki_yalniz_gercek_listede_silinir() {
        // birden çok numaralı satır = model madde yazmış, önek gider
        let c = cevap_parcala("1. şunu yap\n2) sonra bunu");
        assert_eq!(c.satirlar, vec!["şunu yap", "sonra bunu"]);
        // tek satırdaki numara Türkçe sıra sayısıdır, anlam düşmemeli
        assert_eq!(
            cevap_parcala("3. sınıftayım").satirlar,
            vec!["3. sınıftayım"]
        );
        assert_eq!(
            cevap_parcala("2. el araba aldım").satirlar,
            vec!["2. el araba aldım"]
        );
        assert_eq!(numara_oneki("12) madde"), Some("madde"));
        assert_eq!(numara_oneki("3.14 sayısı"), None);
    }

    #[test]
    fn ayni_satir_iki_kez_gitmez() {
        assert_eq!(cevap_parcala("he\nhe").satirlar, vec!["he"]);
    }

    #[test]
    fn cevap_uzun_satir_bolunur() {
        let uzun = "a".repeat(MESAJ_SINIRI + 100);
        let c = cevap_parcala(&uzun);
        assert_eq!(c.satirlar.len(), 2);
        for s in &c.satirlar {
            assert!(s.chars().count() <= MESAJ_SINIRI);
        }
    }

    #[test]
    fn protokol_metni_geri_yazilir() {
        assert_eq!(
            cevap_parcala("hahaha\ntepki: 💀").protokol_metni(),
            "hahaha\ntepki: 💀"
        );
        assert_eq!(cevap_parcala("tepki: 💀").protokol_metni(), "tepki: 💀");
        assert_eq!(cevap_parcala("bir\niki").protokol_metni(), "bir\niki");
    }

    #[test]
    fn akis_kesiti_yarim_satiri_bekletir() {
        // akış sürerken tamamlanmamış kısa satır görünmez ("tep" → "tepki: 💀" olabilir)
        assert_eq!(akis_kesiti("selam\ntep", false), "selam");
        // yeterince uzun yarım satır yerleşime girer
        let m = "selam\nbu satır yeterince uzun";
        assert_eq!(akis_kesiti(m, false), m);
        // tek satırlık kısa akış henüz gösterilmez
        assert_eq!(akis_kesiti("kısa", false), "");
        // akış bitince her şey görünür
        assert_eq!(akis_kesiti("selam\ntep", true), "selam\ntep");
    }

    #[test]
    fn gorunum_satirlari_mesaja_cevirir() {
        // her satır ayrı mesaj
        let v = akis_gorunum(DusunmeKip::Kapali, "", "bir\niki", true);
        assert_eq!(v, vec!["bir", "iki"]);
        // akış sürerken yarım satır beklemede
        let v = akis_gorunum(DusunmeKip::Kapali, "", "hahaha\ntep", false);
        assert_eq!(v, vec!["hahaha"]);
        // tepki satırı mesaj olmaz
        let v = akis_gorunum(DusunmeKip::Kapali, "", "hahaha\ntepki: 💀", true);
        assert_eq!(v, vec!["hahaha"]);
        // sus işareti hiç yerleşime girmez
        assert!(akis_gorunum(DusunmeKip::Kapali, "", "-", true).is_empty());
    }

    #[test]
    fn soru_tavani_dolar() {
        let kanal = ChannelId::new(3);
        let mut d = Durum {
            bot_adi: "kaju".into(),
            ..Durum::default()
        };
        let dolu: VecDeque<String> = [
            "emin: naber",
            "kaju: iyidir sen?",
            "emin: iyi",
            "kaju: ne yapıyosun?",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        d.kanal_gecmisi.insert(kanal, dolu);
        assert!(soru_fazla_mi(&d, kanal));
        // tepki satırları sayılmaz; araya düz laf girince tavan dolmaz
        let seyrek: VecDeque<String> = ["kaju: iyidir sen?", "kaju: tepki: 💀", "kaju: aynen"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        d.kanal_gecmisi.insert(kanal, seyrek);
        assert!(!soru_fazla_mi(&d, kanal));
        // geçmişi olmayan kanal
        assert!(!soru_fazla_mi(&d, ChannelId::new(99)));
    }

    #[test]
    fn mesaj_json_resimli_dizi_olur() {
        // resimsiz: düz metin content
        let j = mesaj_json(&kullanici("emin: selam"));
        assert_eq!(j["role"], "user");
        assert_eq!(j["content"], "emin: selam");
        // resimli: metin + image_url parçaları (resimci ile aynı biçim)
        let j = mesaj_json(&kullanici_resimli(
            "emin: [resim] şuna bak",
            "https://cdn.discordapp.com/a.png",
        ));
        assert_eq!(j["content"][0]["type"], "text");
        assert_eq!(j["content"][0]["text"], "emin: [resim] şuna bak");
        assert_eq!(j["content"][1]["type"], "image_url");
        assert_eq!(
            j["content"][1]["image_url"]["url"],
            "https://cdn.discordapp.com/a.png"
        );
        // asistan mesajında resim hiç olmaz
        assert_eq!(mesaj_json(&asistan("he"))["content"], "he");
    }

    #[test]
    fn soy_dilim_dondurur() {
        // soy artık &str alır, &str döndürür; tam metin klonu gerekmez
        let metin = String::from("cicikus: merhaba dünya");
        let dilim: &str = soy(&metin, "cicikus");
        assert_eq!(dilim, "merhaba dünya");
    }
}
