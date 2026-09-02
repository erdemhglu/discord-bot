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
    // reasoning zorunlu modeller (glm-5.3-flash gibi) cevabı bazen düşünce alanına gömer
    #[serde(default)]
    reasoning: Option<String>,
    #[serde(default)]
    reasoning_content: Option<String>,
}

// JSON bekleyen çağrı tipleri: content boşsa düşünce alanındaki JSON bloğu içerik sayılır
const JSON_KATEGORILER: [&str; 5] = ["gunlukcu", "isteklilik", "hedef_sec", "ruh_hali", "uyanis"];

// model yanıtından içerik: content doluysa o. Boşsa ve çağıran JSON bekliyorsa reasoning'de
// { … } bloğu aranır (reasoning zorunlu modeller cevabı düşünceye gömebiliyor); düzyazı
// çağrısında reasoning ASLA içerik sayılmaz (hoca düşünce dökümünü huy sanmasın).
fn yanit_icerigi(icerik: &Icerik, kategori: &str) -> Option<String> {
    if let Some(c) = icerik
        .content
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return Some(c.to_string());
    }
    if !JSON_KATEGORILER.contains(&kategori) {
        return None;
    }
    let dusunce = icerik
        .reasoning_content
        .as_deref()
        .or(icerik.reasoning.as_deref())?;
    let aday = json_ayikla(dusunce);
    (aday.starts_with('{') && serde_json::from_str::<serde_json::Value>(aday).is_ok())
        .then(|| aday.to_string())
}

// yanıttaki düşünce uzunluğu (hata mesajı için: bütçeyi düşünce mi yedi)
fn dusunce_uzunlugu(icerik: &Icerik) -> usize {
    icerik
        .reasoning_content
        .as_deref()
        .or(icerik.reasoning.as_deref())
        .map_or(0, |r| r.chars().count())
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
