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
    debug: bool, // !debug: kararlar (isteklilik, hedef, sus/tepki) kanala düşer; durum/debug.md
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
            debug: hafiza::oku("debug.md").trim() == "acik",
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
