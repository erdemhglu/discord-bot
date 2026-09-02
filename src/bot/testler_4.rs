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
