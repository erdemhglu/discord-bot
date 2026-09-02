fn cevap_parcala(metin: &str) -> Cevap {
    let mut c = Cevap::default();
    let mut satirlar: Vec<String> = Vec::new();
    let mut atlanan = 0usize;
    // numara öneki ancak GERÇEK liste ise elenir: iki ya da daha çok numaralı satır varsa
    // model madde yazmış demektir. Tek satırdaki "3. sınıftayım" sıra sayısıdır, yenmez.
    let liste = metin
        .split('\n')
        .filter(|s| numara_oneki(s.trim()).is_some())
        .count()
        >= 2;
    for ham in metin.split('\n') {
        let mut satir = ham.trim();
        if satir.is_empty() {
            continue;
        }
        if sus_isareti(satir) {
            c.sus = true;
            continue;
        }
        if let Some(govde) = tepki_govdesi(satir) {
            // ilk tepki kazanır; emoji çözülemezse satır yine de mesaj olarak gitmez
            if c.tepki.is_none() {
                c.tepki = emoji_ayikla(govde);
            }
            continue;
        }
        // önceki mesajın kırıntısı ("'cım" gibi) gitmesin
        if satir.starts_with('\'') {
            continue;
        }
        if liste {
            if let Some(k) = numara_oneki(satir) {
                satir = k;
            }
        }
        let temiz = slop_temizle(satir);
        if temiz.is_empty() {
            continue;
        }
        // aynı turda birebir aynı laf iki mesaj olarak gitmesin ("he\nhe")
        if satirlar.contains(&temiz) {
            continue;
        }
        if satirlar.len() >= PATLAMA_SINIRI {
            atlanan += 1;
            continue;
        }
        satirlar.push(temiz);
    }
    if atlanan > 0 {
        log::debug!("cevap: patlama sınırı aşıldı, {atlanan} satır düştü");
    }
    // 1900'ü aşan satır tek mesaja sığmaz: bölünür ve düzleştirilir
    c.satirlar = satirlar.iter().flat_map(|s| bol(s, MESAJ_SINIRI)).collect();
    c
}

// son 4 bot satırından (tepki satırları sayılmaz) ikisi soruyla bittiyse üst üste
// soru sormuş demektir; cevapla bunu talimata çevirir
fn soru_fazla_mi(d: &Durum, kanal: ChannelId) -> bool {
    let onek = format!("{}: ", d.bot_adi);
    d.kanal_gecmisi
        .get(&kanal)
        .map(|g| {
            g.iter()
                .rev()
                .filter_map(|l| l.strip_prefix(&onek))
                .filter(|l| tepki_govdesi(l).is_none())
                .take(4)
                .filter(|l| l.trim_end().ends_with('?'))
                .count()
        })
        .unwrap_or(0)
        >= 2
}

// geçici sayılan durum kodları: geri çekilip yeniden denemeye değer
// (429 hız sınırı, 5xx sunucu tarafı); 401/404 gibi kalıcı hatalar denemeye girmez
fn durum_denenebilir(d: reqwest::StatusCode) -> bool {
    matches!(d.as_u16(), 429 | 500 | 502 | 503 | 504)
}

// hata gövdesini tek satıra indirir
fn kirp_hata(metin: &str) -> String {
    metin
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(300)
        .collect()
}

// sağlayıcı "reasoning kapatılamaz" diye 400 dönmüş mü (bazı GLM reasoning varyantları gibi)
fn reasoning_zorunlu_hatasi(govde_metni: &str) -> bool {
    let m = govde_metni.to_lowercase();
    m.contains("reasoning") && (m.contains("mandatory") || m.contains("cannot be disabled"))
}

// ```json ... ``` gibi süslerin içinden json'u çıkarır
fn json_ayikla(metin: &str) -> &str {
    match (metin.find('{'), metin.rfind('}')) {
        (Some(b), Some(s)) if s > b => &metin[b..=s],
        _ => metin,
    }
}

// isteklilik cevabından 0-10 puanı ve sebebi çözer; bozuksa None (sebep debug satırı için)
fn isteklilik_coz(cevap: &str) -> Option<(u8, String)> {
    #[derive(Deserialize)]
    struct Deger {
        #[serde(default)]
        puan: i32,
        #[serde(default)]
        sebep: String,
    }
    let d: Deger = serde_json::from_str(json_ayikla(cevap)).ok()?;
    Some((d.puan.clamp(0, 10) as u8, d.sebep.trim().to_string()))
}

#[cfg(test)]
fn isteklilik_puan(cevap: &str) -> Option<u8> {
    isteklilik_coz(cevap).map(|(p, _)| p)
}

// hedef seçiminden kişi adını çözer: önce JSON, olmazsa ilk satır/kelime
fn hedef_ayikla(cevap: &str, bilinenler: &[String]) -> Option<String> {
    #[derive(Deserialize)]
    struct Hedef {
        #[serde(default)]
        hedef: String,
    }
    let aday = serde_json::from_str::<Hedef>(json_ayikla(cevap))
        .map(|h| h.hedef.trim().to_string())
        .unwrap_or_else(|_| {
            cevap
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches('"')
                .to_string()
        });
    if aday.is_empty() {
        return None;
    }
    // bilinen adlardan birine benziyorsa onu kullan (model süslemiş olabilir)
    bilinenler
        .iter()
        .find(|b| b.eq_ignore_ascii_case(&aday))
        .cloned()
        .or(Some(aday))
}

// ruh hali cevabından durum+yoğunluğu çözer; yoğunluk düşükse (nötr) hiç yansıtmaya değmez
fn ruh_hali_ayikla(cevap: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct RuhHali {
        #[serde(default)]
        durum: String,
        #[serde(default)]
        yogunluk: i32,
    }
    let r: RuhHali = serde_json::from_str(json_ayikla(cevap)).ok()?;
    let durum = r.durum.trim();
    if durum.is_empty() {
        return None;
    }
    let yogunluk = r.yogunluk.clamp(1, 10);
    if yogunluk < 3 {
        return None; // nötr/belirsiz: talimata eklemeye değmez, düz kişilik konuşsun
    }
    Some(format!("{durum} ({yogunluk})"))
}

