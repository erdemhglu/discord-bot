    /// Verifies `response_content` reads JSON from `content`, falls back to extracting embedded JSON from the reasoning/thought when `content` is empty, but only for calls that expect JSON (not plain-prose ones).
    #[test]
    fn response_content_reads_json_from_thought() {
        let filled = Content {
            content: Some(" {\"puan\": 4} ".into()),
            reasoning: None,
            reasoning_content: None,
        };
        assert_eq!(
            response_content(&filled, "isteklilik").as_deref(),
            Some("{\"puan\": 4}")
        );
        // content empty, the reply is in the thought: a call expecting JSON picks it up
        let embedded = Content {
            content: None,
            reasoning: None,
            reasoning_content: Some(
                "düşünüyorum... sonuç: {\"puan\": 7, \"sebep\": \"bana soruldu\"} bitti".into(),
            ),
        };
        assert_eq!(
            response_content(&embedded, "isteklilik").as_deref(),
            Some("{\"puan\": 7, \"sebep\": \"bana soruldu\"}")
        );
        // a plain-prose call doesn't treat the thought as content (the coach shouldn't
        // mistake a chain of thought for the temperament summary)
        assert_eq!(response_content(&embedded, "hoca"), None);
        // no JSON in the thought: still empty
        let plain = Content {
            content: Some(String::new()),
            reasoning: Some("sadece düşünce, { yarım".into()),
            reasoning_content: None,
        };
        assert_eq!(response_content(&plain, "gunlukcu"), None);
        assert_eq!(thought_length(&plain), 23);
    }

    /// Verifies `Bot::grow_budget` doubles `max_tokens`, clamps at the floor, and leaves an unbudgeted call alone.
    #[test]
    fn grow_budget_doubles_and_respects_floor() {
        let mut body = serde_json::json!({ "max_tokens": 1200 });
        assert_eq!(Bot::grow_budget(&mut body, 1500), Some(2400));
        let mut small_body = serde_json::json!({ "max_tokens": 80 });
        assert_eq!(Bot::grow_budget(&mut small_body, 1500), Some(1500));
        // leaves an unbudgeted call alone
        let mut no_budget = serde_json::json!({ "model": "x" });
        assert_eq!(Bot::grow_budget(&mut no_budget, 1500), None);
        assert!(no_budget.get("max_tokens").is_none());
    }

    /// Verifies `willingness_score` extracts and clamps the `puan` field from a willingness reply, including a fenced-code-block form.
    #[test]
    fn willingness_score_extracted() {
        assert_eq!(
            willingness_score(r#"{"puan": 7, "sebep": "bana soruldu"}"#),
            Some(7)
        );
        assert_eq!(willingness_score("```json\n{\"puan\": 3}\n```"), Some(3));
        assert_eq!(willingness_score(r#"{"puan": 25}"#), Some(10)); // clamp
        assert_eq!(willingness_score(r#"{"puan": -4}"#), Some(0));
        assert_eq!(willingness_score("puan veremem"), None);
    }

    /// Verifies `extract_target` reads the `hedef` field from JSON, matches plain text against known names, and passes an unknown name through as-is.
    #[test]
    fn target_extracted() {
        let known = vec!["Emin".to_string(), "Zeynep".to_string()];
        assert_eq!(
            extract_target(r#"{"hedef": "Zeynep"}"#, &known),
            Some("Zeynep".into())
        );
        // plain text works too, matched against a known name
        assert_eq!(extract_target("emin", &known), Some("Emin".into()));
        // an unknown name is returned as-is
        assert_eq!(
            extract_target(r#"{"hedef": "Misafir"}"#, &known),
            Some("Misafir".into())
        );
        assert_eq!(extract_target("", &known), None);
    }

    /// Verifies `Bot::apply_budget_floor` raises `max_tokens` only when it's below the floor, and leaves an unbudgeted call alone.
    #[test]
    fn budget_raised_when_below_floor() {
        let mut body = serde_json::json!({"max_tokens": 20});
        assert!(Bot::apply_budget_floor(&mut body, 500));
        assert_eq!(body["max_tokens"], 500);
        // left alone when already above the floor
        let mut body = serde_json::json!({"max_tokens": 800});
        assert!(!Bot::apply_budget_floor(&mut body, 500));
        assert_eq!(body["max_tokens"], 800);
        // left alone when max_tokens isn't set at all (an unbudgeted call)
        let mut body = serde_json::json!({});
        assert!(!Bot::apply_budget_floor(&mut body, 500));
        assert!(body.get("max_tokens").is_none());
    }

    /// Verifies `reasoning_mandatory_error` recognizes the specific "reasoning is mandatory" API error and rejects unrelated ones.
    #[test]
    fn reasoning_mandatory_error_recognized() {
        assert!(reasoning_mandatory_error(
            r#"{"error":{"message":"Reasoning is mandatory for this endpoint and cannot be disabled.","code":400}}"#
        ));
        assert!(!reasoning_mandatory_error(
            r#"{"error":{"message":"model not found","code":404}}"#
        ));
        assert!(!reasoning_mandatory_error("rate limit exceeded"));
    }

    /// Verifies `extract_mood` reads `durum`/`yogunluk`, clamps intensity, and treats low intensity as neutral (`None`).
    #[test]
    fn mood_extracted() {
        assert_eq!(
            extract_mood(r#"{"durum": "kafa karışıklığı", "yogunluk": 6}"#),
            Some("kafa karışıklığı (6)".into())
        );
        // clamp: 15 -> 10
        assert_eq!(
            extract_mood(r#"{"durum": "öfke", "yogunluk": 15}"#),
            Some("öfke (10)".into())
        );
        // low intensity: counts as neutral, never reflected
        assert_eq!(
            extract_mood(r#"{"durum": "huzur", "yogunluk": 2}"#),
            None
        );
        assert_eq!(extract_mood(r#"{"durum": "", "yogunluk": 8}"#), None);
        assert_eq!(extract_mood("bozuk cevap"), None);
    }

    /// Verifies `supports_cache` is true only for an openrouter.ai URL, regardless of model.
    #[test]
    fn cache_support_depends_on_openrouter_url() {
        // every request going to openrouter: model doesn't matter (claude/gemini/gpt/glm/grok),
        // openrouter decides on its own side which one it actually works for
        assert!(supports_cache(
            "https://openrouter.ai/api/v1/chat/completions"
        ));
        // mistral's native api and a custom router (API_URL) offer no such guarantee
        assert!(!supports_cache(
            "https://api.mistral.ai/v1/chat/completions"
        ));
        assert!(!supports_cache(
            "http://localhost:8080/v1/chat/completions"
        ));
    }

    /// Verifies `system_json` attaches `cache_control` only when the target URL is openrouter.ai, and skips the block entirely when the variable part is empty.
    #[test]
    fn system_json_cache_only_on_openrouter() {
        let or_url = "https://openrouter.ai/api/v1/chat/completions";
        let claude = system_json("sabit", "degisken", or_url);
        assert!(claude["content"][0]["cache_control"].is_object());
        // gpt/glm/grok through openrouter are marked too, the decision is left to openrouter
        let glm = system_json("sabit", "degisken", or_url);
        assert!(glm["content"][0]["cache_control"].is_object());
        let mistral = system_json(
            "sabit",
            "degisken",
            "https://api.mistral.ai/v1/chat/completions",
        );
        assert!(mistral["content"][0]["cache_control"].is_null());
        // empty variable: plain text regardless of url (no block at all)
        let plain = system_json("sabit", "", or_url);
        assert!(plain["content"].is_string());
    }

    /// Verifies `thought_counter`'s word count in its placeholder text.
    #[test]
    fn thought_counter_increments() {
        assert_eq!(
            thought_counter("tek"),
            "Düşünüyorum... Şu ana kadar 1 kelime düşündüm."
        );
        assert_eq!(
            thought_counter("a b\nc  d"),
            "Düşünüyorum... Şu ana kadar 4 kelime düşündüm."
        );
    }

    /// Verifies `thought_display` wraps the thought in a code block and truncates an overlong one with a note.
    #[test]
    fn thought_display_code_block() {
        let result = thought_display("düşünce metni");
        assert!(result.starts_with("```\n") && result.ends_with("\n```"));
        assert!(result.contains("düşünce metni"));
        // a long thought gets truncated with a note appended
        let long_text = "a".repeat(5000);
        let result = thought_display(&long_text);
        assert!(result.chars().count() <= MESSAGE_LIMIT);
        assert!(result.contains("kısaltıldı"));
    }

    /// Verifies `single_line` collapses newlines/extra whitespace into single spaces.
    #[test]
    fn collapses_to_single_line() {
        assert_eq!(single_line("a\nb\n\nc"), "a b c");
        assert_eq!(single_line("  boşluk   ve\nsatır  "), "boşluk ve satır");
    }

    // the reply budget varies by build profile: both are Option<u32>; which value shows
    // up depends on the profile, this only checks the type and internal consistency
    /// Verifies `reply_budget!()` returns a budget under `REPLY_CAP` in every build profile.
    #[test]
    fn reply_budget_consistent() {
        let budget: Option<u32> = reply_budget!();
        // both profiles have a cap now; debug's should be less than or equal to release's
        assert!(budget.is_some_and(|t| t <= REPLY_CAP));
    }

    // reads a real reqwest stream from a fake SSE server: even if a utf-8 chunk is split
    // mid-character, or reasoning and content arrive interleaved, they accumulate correctly
