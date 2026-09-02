impl Bot {
    // düşünme kapalıysa modelin reasoning üretmesini istekte kapatır (token harcamasın).
    // her sağlayıcının dili farklı: openrouter "reasoning", qwen tarzı router'lar
    // "enable_thinking" anlar, mistral'de düğme yok (ikisini birden yollamak bazı
    // sağlayıcıları bozuyordu, adres bazlı seçilir). Alanları gerçekten eklediyse true
    // döner (bazı modeller yine de reasoning'i kapatmaya izin vermiyor, çağıran taraf
    // bunu geri alıp bir kez daha dener).
    // herhalukarda=true: kullanıcının kipine bakmadan kapatır — sor_ham (stream olmayan)
    // reasoning_content alanını hiç okumaz/göstermez, o yüzden arka plan ajanları (profilci,
    // hoca, günlükçü, gezgin, isteklilik, ruh_hali) kip "gizle" iken bile boşuna düşünüp küçük
    // max_tokens bütçesini tüketmesin, content: null dönüp "modelden boş yanıt geldi" hatasına
    // düşmesin. sor_ham_akis (stream, sohbet) kullanıcı kipine bakmaya devam eder: "gizle"
    // düşünce sayacı, "göster" tam metin gösterir, o yüzden false geçer.
    fn reasoning_kapat(&self, govde: &mut serde_json::Value, herhalukarda: bool) -> bool {
        if !herhalukarda && self.durum().dusunme != DusunmeKip::Kapali {
            return false;
        }
        let Some(o) = govde.as_object_mut() else {
            return false;
        };
        let adres = self.api_adres.to_lowercase();
        if adres.contains("openrouter") {
            o.insert("reasoning".into(), serde_json::json!({ "enabled": false }));
        } else if !adres.contains("mistral") {
            o.insert("enable_thinking".into(), serde_json::json!(false));
        } else {
            return false; // mistral: hiçbir alan eklenmedi, geri alacak bir şey yok
        }
        true
    }

    // reasoning_kapat'ın eklediği alanları geri alır (mandatory-reasoning hatasında yeniden deneme için)
    fn reasoning_alanlarini_kaldir(govde: &mut serde_json::Value) {
        if let Some(o) = govde.as_object_mut() {
            o.remove("reasoning");
            o.remove("enable_thinking");
        }
    }

    // max_tokens taban altındaysa yükseltir; reasoning zorunlu modelde bütçesiz kalmasın diye.
    // max_tokens hiç yoksa (bütçesiz çağrı) dokunmaz. Değiştirdiyse true döner (log için).
    fn butce_tabanini_uygula(govde: &mut serde_json::Value, taban: u32) -> bool {
        match govde.get("max_tokens").and_then(serde_json::Value::as_u64) {
            Some(mevcut) if (mevcut as u32) < taban => {
                govde["max_tokens"] = serde_json::json!(taban);
                true
            }
            _ => false,
        }
    }

    // reasoning kapatılamayan modelde bütçeyi büyütür: düşünce bütçenin çoğunu yiyebiliyor,
    // 500 tabanı 1200'lük günlükçü çağrısına hiç dokunmuyordu. max_tokens yoksa dokunmaz;
    // büyüttüyse yeni değeri döner
    fn butce_buyut(govde: &mut serde_json::Value, taban: u32) -> Option<u32> {
        let mevcut = govde
            .get("max_tokens")
            .and_then(serde_json::Value::as_u64)? as u32;
        let yeni = mevcut.saturating_mul(2).max(taban);
        if yeni <= mevcut {
            return None;
        }
        govde["max_tokens"] = serde_json::json!(yeni);
        Some(yeni)
    }

    // openrouter: reasoning kapatılamıyorsa en azından kısa düşünsün (birleşik parametre,
    // desteklemeyen model yok sayar). Başka adrese gitmez: bilinmeyen alan isteği bozabilir
    fn reasoning_dusuk_efor(&self, govde: &mut serde_json::Value) {
        if !self.api_adres.to_lowercase().contains("openrouter") {
            return;
        }
        if let Some(o) = govde.as_object_mut() {
            o.insert("reasoning".into(), serde_json::json!({ "effort": "low" }));
        }
    }

    // açıklayıcı hata ile ham istek; her şey buradan geçer. Ağ hatası / 429 / 5xx'te geri
    // çekilip yeniden dener; bazı modeller (ör. bazı GLM reasoning varyantları) reasoning'i
    // kapatmaya izin vermiyor ("mandatory"/"cannot be disabled" 400) — o durumda alanları
    // kaldırıp açık haliyle yeniden dener, model her turda başarısız olup sohbeti tıkamasın.
    // kategori yalnız token metriğinde kırılım için (!durum), isteğe hiçbir etkisi yok.
}
