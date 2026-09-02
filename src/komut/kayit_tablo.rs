// Slash komut yöneticisi: her komut adı+açıklama+seçenekler+çalıştırıcıyı tek yerde
// tutan bir kayıt tablosu (`tanimlar`). `modal::komutlari_kayit` bu tablodan Discord'a
// kayıt listesi çıkarır, `interaction_create` (main.rs) gelen `Interaction::Command`'ı
// isme göre bu tabloda bulup çalıştırır. `!`/metin komutları yok — bot yalnız slash
// komutlarla yönetilir (mesajlar yalnız sohbet/hafıza akışına girer).
//
// Discord ilk yanıtı 3 sn içinde ister: yerel/hızlı komutlar doğrudan embed döner,
// ağ/model çağrısı yapan komutlar önce `ertele` ile onay verip `sonucu_bildir` ile
// sonucu düzenler (haber/sorun/saka/ajanlar/gez/uyan/uyu zaten kendi mesajlarını
// `Bot::gonder` üzerinden kanala basıyor; buradaki embed yalnız kısa bir "tamam" notu).

use super::*;
use std::future::Future;
use std::pin::Pin;
use std::sync::OnceLock;

pub const YARDIM: &str = "\
komutlar (hepsi slash):
`/sifirla [hepsi]` kanal yasağını ve açık sohbeti sıfırlar
`/haber` şimdi haber atar · `/sorun` kod derdi sorar · `/gez` gündem gezintisi yapar
`/saka` / `/hack` görsel şakası / hacklenmiş taklidi
`/ajanlar` profilci ve hocayı şimdi çalıştırır
`/uyan` uykuyu keser · `/uyu [saat]` test için uyutur
`/durum` evre, sayaçlar, model, düşünme, uyku, seyahat
`/zihin [test]` kişi/konu/olay kartı + menü/butonlarla detay modalları; `test` son 30 satırı hemen günlükçüye verir (zihin zinciri teşhisi)
`/dusunme [kip]` düşünme kipi (göster: cevapla spoiler'da · gizle: düşünürken \"Düşünüyorum...\", cevap sonra · sessiz: arka planda düşünür, hiç iz göstermez · kapat: istekler reasoning'siz)
`/model [id]` modeli gösterir/değiştirir (yalnız favori)
`/debug [durum]` karar izleri kanala düşer: isteklilik puanı/sebebi, hedef, ruh hali, sus/tepki, sohbet kapanışı
`/ayarlar` butonlu ayar paneli: düşünme kipi, debug, uyku";

// ---------- komut kayıt tablosu ----------

type KomutFn = for<'a> fn(
    &'a Bot,
    &'a Context,
    &'a CommandInteraction,
) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

pub struct KomutTanimi {
    pub ad: &'static str,
    pub aciklama: &'static str,
    pub secenekler: fn() -> Vec<CreateCommandOption>,
    pub calistir: KomutFn,
}

macro_rules! komut_gir {
    ($ad:expr, $aciklama:expr, $secenekler:expr, $f:expr) => {
        KomutTanimi {
            ad: $ad,
            aciklama: $aciklama,
            secenekler: $secenekler,
            calistir: |b, c, i| Box::pin($f(b, c, i)),
        }
    };
}

pub fn tanimlar() -> &'static [KomutTanimi] {
    static TABLO: OnceLock<Vec<KomutTanimi>> = OnceLock::new();
    TABLO.get_or_init(|| {
        vec![
            komut_gir!(
                "durum",
                "Botun şu anki halini kart olarak gösterir",
                Vec::new,
                k_durum
            ),
            komut_gir!(
                "yardim",
                "Komut listesini kart olarak gösterir",
                Vec::new,
                k_yardim
            ),
            komut_gir!(
                "zihin",
                "Botun bildiklerini interaktif kart + menü/butonlarla gösterir",
                secenekler_zihin,
                k_zihin
            ),
            komut_gir!(
                "ayarlar",
                "Butonlu ayar paneli: düşünme kipi, debug, uyku",
                Vec::new,
                k_ayarlar
            ),
            komut_gir!(
                "sifirla",
                "Kanal yasağını ve açık sohbeti sıfırlar",
                secenekler_sifirla,
                k_sifirla
            ),
            komut_gir!("haber", "Şimdi haber atar", Vec::new, k_haber),
            komut_gir!("sorun", "Kod derdi sorar", Vec::new, k_sorun),
            komut_gir!("gez", "Gündem gezintisi yapar", Vec::new, k_gez),
            komut_gir!("saka", "Görsel şakası yapar", Vec::new, k_saka),
            komut_gir!("hack", "Hacklenmiş taklidi yapar", Vec::new, k_hack),
            komut_gir!(
                "ajanlar",
                "Profilci ve hocayı şimdi çalıştırır",
                Vec::new,
                k_ajanlar
            ),
            komut_gir!("uyan", "Uykuyu keser", Vec::new, k_uyan),
            komut_gir!("uyu", "Test için uyutur", secenekler_uyu, k_uyu),
            komut_gir!(
                "dusunme",
                "Düşünme kipini gösterir/değiştirir",
                secenekler_dusunme,
                k_dusunme
            ),
            komut_gir!(
                "model",
                "Modeli gösterir/değiştirir (yalnız favori)",
                secenekler_model,
                k_model
            ),
            komut_gir!(
                "debug",
                "Karar izlerini kanala düşürür/kapatır",
                secenekler_debug,
                k_debug
            ),
        ]
    })
}

// ---------- seçenek tanımları ----------

