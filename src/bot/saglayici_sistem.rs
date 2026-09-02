fn onbellek_destekler(api_adres: &str) -> bool {
    api_adres.contains("openrouter.ai")
}

// sistem mesajını OpenAI uyumlu bloğa çevirir: değişken boşsa düz metin, değilse sabit+değişken
// iki metin bloğu; sabit blok yalnız openrouter'a giderken cache_control ile işaretlenir
fn sistem_json(sabit: &str, degisken: &str, api_adres: &str) -> serde_json::Value {
    if degisken.is_empty() {
        return serde_json::json!({ "role": "system", "content": sabit });
    }
    let mut ilk = serde_json::json!({ "type": "text", "text": sabit });
    if onbellek_destekler(api_adres) {
        ilk["cache_control"] = serde_json::json!({ "type": "ephemeral" });
    }
    serde_json::json!({ "role": "system", "content": [
        ilk,
        { "type": "text", "text": degisken }
    ]})
}

// her cevabın sistem mesajı iki parça: SABİT (kişilik, huy, profil, dizin...; ajan çalışınca
// değişir, prompt cache buradan kazanır) ve DEĞİŞKEN (getirilenler, saat, görev)
fn sistem_metni(d: &Durum, talimat: &str, getirilen: &str) -> (String, String) {
    let favori_satiri = match &d.favori_adi {
        Some(f) => FAVORI_SATIRI.replace("{favori}", f),
        None => String::new(),
    };
    let bolum = |s: &mut String, baslik: &str, icerik: &str| {
        if !icerik.trim().is_empty() {
            if !s.is_empty() {
                s.push_str("\n\n");
            }
            s.push_str(baslik);
            s.push('\n');
            s.push_str(icerik.trim());
        }
    };

    let mut sabit = KISILIK
        .replace("{ad}", &d.bot_adi)
        .replace("{favori_satiri}", &favori_satiri);
    bolum(
        &mut sabit,
        "GELİŞİM EVREN",
        &gelisim::evre_metni(&d.gelisim),
    );
    bolum(
        &mut sabit,
        "HUYUN (hocanın son notu, buna göre davran)",
        &d.huy,
    );
    bolum(&mut sabit, "BU GRUP HAKKINDA BİLDİKLERİN", &d.profil);
    bolum(
        &mut sabit,
        "HAFIZA DİZİNİ (kimi ve neyi biliyorsun; ayrıntı gerekince getiriliyor)",
        &d.dizin,
    );
    bolum(
        &mut sabit,
        "GÜNDEM (internette gezerken okudukların ve düşündüklerin)",
        &d.gundem,
    );
    bolum(&mut sabit, "SENİN SON HALİN", &d.kendim);
    bolum(
        &mut sabit,
        "KENDİNE NOTLAR (eleştirmenin son sohbetten çıkardığı dersler)",
        &d.duzeltmeler,
    );

    let mut degisken = String::new();
    bolum(
        &mut degisken,
        "BU SOHBET İÇİN HAFIZADAN GETİRİLENLER",
        getirilen,
    );
    bolum(
        &mut degisken,
        "ŞU AN",
        &format!("{} {}", uyku::durum_metni(d), seyahat::durum_metni()),
    );
    bolum(&mut degisken, "ŞU ANKİ GÖREVİN", talimat);
    (sabit, degisken)
}

// ---------- sohbet mekanizması ----------

fn sohbet_baslat(d: &mut Durum, kanal: ChannelId, acilis: Option<String>) -> &mut Sohbet {
    let mut s = Sohbet::default();
    // kanalın son satırlarıyla başla ki daha önce ne konuşulduğunu bilsin
    let onek = format!("{}: ", d.bot_adi);
    if let Some(g) = d.kanal_gecmisi.get(&kanal) {
        let atla = g.len().saturating_sub(SOHBET_TOHUM);
        for satir in g.iter().skip(atla) {
            match satir.strip_prefix(&onek) {
                Some(m) => s.gecmis.push(asistan(m)),
                None => s.gecmis.push(kullanici(satir.clone())),
            }
        }
    }
    if let Some(a) = acilis {
        // Açılış zaten gönderildi ve kanal geçmişine SATIR SATIR düştü (her satır ayrı
        // mesaj); tohumun sonundaki bot bloğunda o satırlar temizlenir, yoksa model kendi
        // açılışını hem tek tek hem de birleşik hâlde iki kez görür. Araya haber linki
        // gibi başka bir bot mesajı girmiş olabilir, o yüzden blok boyunca taranır.
        let parcalar: Vec<&str> = a.split('\n').map(str::trim).collect();
        let mut i = s.gecmis.len();
        while i > 0 && s.gecmis[i - 1].role == "assistant" {
            i -= 1;
            if parcalar.contains(&s.gecmis[i].content.trim()) {
                s.gecmis.remove(i);
            }
        }
        s.gecmis.push(asistan(a));
        s.sayac = 1;
    }
    d.son_aktivite.insert(kanal, Instant::now());
    d.sohbetler.entry(kanal).or_insert(s)
}

fn sohbet_bitir(d: &mut Durum, kanal: ChannelId) -> Option<Sohbet> {
    d.haber_bekleyen.remove(&kanal);
    d.sohbetler.remove(&kanal)
}

fn yeni_mesaj_var(d: &Durum, kanal: ChannelId, uretilen_gelen: u32) -> bool {
    d.sohbetler
        .get(&kanal)
        .is_some_and(|s| s.gelen > uretilen_gelen)
}

