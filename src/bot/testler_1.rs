
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

