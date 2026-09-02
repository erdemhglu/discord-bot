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
// stream'siz ajan çağrılarında reasoning açık yeniden denemenin bütçe tabanı: 2×mevcut ya da bu
const REASONING_BUTCE_TABANI: u32 = 1500;
const FAVORI: u64 = 259669117248864257; // bu kişiyi ne olursa olsun sever
const GEZGIN_ARALIGI: Duration = Duration::from_secs(4 * 60 * 60); // ne sıklıkla internette gezer
const RESIM_KLASORU: &str = "resimler"; // şakalarda atılacak görseller
const DURUM_KLASORU: &str = "durum"; // ajanların öğrendikleri buraya yazılır
                                     // sürüm: Cargo.toml + derlemede build.rs'in git'ten aldığı commit ve tarih.
                                     // !durum'da görünür, yeniden başlayınca kanala duyurulur (hangi kod koşuyor belli olsun)
const SURUM: &str = env!("CARGO_PKG_VERSION");
const SURUM_COMMIT: &str = env!("SURUM_COMMIT");
const SURUM_TARIH: &str = env!("SURUM_TARIH");

fn surum_metni() -> String {
    format!("v{SURUM} ({SURUM_COMMIT}, {SURUM_TARIH})")
}

type Hata = Box<dyn std::error::Error + Send + Sync>;

// kapanış sinyali: döngüler tik başında bakar, yeni tur açmaz; bekçi yeniden başlatmaz
static KAPANIYOR: AtomicBool = AtomicBool::new(false);

// ---------- durum ----------

