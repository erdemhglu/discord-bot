    #[test]
    fn yanit_icerigi_dusunceden_json_alir() {
        let dolu = Icerik {
            content: Some(" {\"puan\": 4} ".into()),
            reasoning: None,
            reasoning_content: None,
        };
        assert_eq!(
            yanit_icerigi(&dolu, "isteklilik").as_deref(),
            Some("{\"puan\": 4}")
        );
        // content boş, cevap düşüncede: JSON bekleyen çağrı alır
        let gomulu = Icerik {
            content: None,
            reasoning: None,
            reasoning_content: Some(
                "düşünüyorum... sonuç: {\"puan\": 7, \"sebep\": \"bana soruldu\"} bitti".into(),
            ),
        };
        assert_eq!(
            yanit_icerigi(&gomulu, "isteklilik").as_deref(),
            Some("{\"puan\": 7, \"sebep\": \"bana soruldu\"}")
        );
        // düzyazı çağrısı düşünceyi içerik saymaz (hoca düşünce dökümünü huy sanmasın)
        assert_eq!(yanit_icerigi(&gomulu, "hoca"), None);
        // düşüncede JSON yoksa yine boş
        let duz = Icerik {
            content: Some(String::new()),
            reasoning: Some("sadece düşünce, { yarım".into()),
            reasoning_content: None,
        };
        assert_eq!(yanit_icerigi(&duz, "gunlukcu"), None);
        assert_eq!(dusunce_uzunlugu(&duz), 23);
    }

    #[test]
    fn butce_buyut_ikiye_katlar_tabani_gozetir() {
        let mut g = serde_json::json!({ "max_tokens": 1200 });
        assert_eq!(Bot::butce_buyut(&mut g, 1500), Some(2400));
        let mut k = serde_json::json!({ "max_tokens": 80 });
        assert_eq!(Bot::butce_buyut(&mut k, 1500), Some(1500));
        // bütçesiz çağrıya dokunmaz
        let mut yok = serde_json::json!({ "model": "x" });
        assert_eq!(Bot::butce_buyut(&mut yok, 1500), None);
        assert!(yok.get("max_tokens").is_none());
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
