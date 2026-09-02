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

