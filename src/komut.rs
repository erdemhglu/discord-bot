// Slash komut yöneticisinin gövdesi src/komut/*.rs'e bölündü (kayıt tablosu / kart
// komutları / eylem komutları / ayar komutları / paylaşılan yardımcılar). main.rs'teki
// bot/*.rs bölünmesiyle aynı gerekçe ve aynı yöntem: `include!` (gerçek `mod` değil) ki
// `use super::*` ve görünürlük hiç değişmesin — dosyalar bu modülde yazılmış gibi derlenir.
include!("komut/kayit.rs");
include!("komut/kartlar.rs");
include!("komut/eylemler.rs");
include!("komut/ayarlar.rs");
include!("komut/kalan.rs");

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn tablo_adlari_benzersiz() {
        let mut adlar: Vec<&str> = tanimlar().iter().map(|k| k.ad).collect();
        let once = adlar.len();
        adlar.sort_unstable();
        adlar.dedup();
        assert_eq!(adlar.len(), once, "komut tablosunda tekrarlanan ad var");
    }

    #[test]
    fn secenekler_panik_atmaz() {
        for k in tanimlar() {
            let _ = (k.secenekler)();
        }
    }

    #[test]
    fn dusunme_secenekleri_arg_ile_ile_uyumlu() {
        // slash seçenek değerleri (goster/gizle/sessiz/kapat) DusunmeKip::arg_ile'nin
        // tanıdığı dizgelerle birebir aynı olmalı, yoksa seçilen kip hiç uygulanmaz
        for deger in ["goster", "gizle", "sessiz", "kapat"] {
            assert!(
                DusunmeKip::arg_ile(deger).is_some(),
                "arg_ile {deger} değerini tanımıyor"
            );
        }
    }
}
