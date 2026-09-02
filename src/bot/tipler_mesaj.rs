#[derive(Serialize, Clone)]
struct Mesaj {
    role: &'static str,
    content: String,
    // ekli görselin adresi; istek gövdesi elle kurulduğu için serileştirmeye girmez.
    // Yalnız son kullanıcı mesajında dolu kalır: discord cdn linki ömürlü, eski
    // görseli her turda yeniden yollamak boşuna token yakar.
    #[serde(skip)]
    resim: Option<String>,
}

fn kullanici(metin: impl Into<String>) -> Mesaj {
    Mesaj {
        role: "user",
        content: metin.into(),
        resim: None,
    }
}

// görsel ekli kullanıcı mesajı: metinle birlikte resim de modele gider
fn kullanici_resimli(metin: impl Into<String>, url: impl Into<String>) -> Mesaj {
    Mesaj {
        role: "user",
        content: metin.into(),
        resim: Some(url.into()),
    }
}

fn asistan(metin: impl Into<String>) -> Mesaj {
    Mesaj {
        role: "assistant",
        content: metin.into(),
        resim: None,
    }
}

// mesajı openai uyumlu istek bloğuna çevirir. Resim varsa content düz metin değil
// çok parçalı dizi olur; biçim ajanlar.rs'deki resimci ile aynı, sağlayıcılar bunu anlıyor
fn mesaj_json(m: &Mesaj) -> serde_json::Value {
    match &m.resim {
        None => serde_json::json!({ "role": m.role, "content": m.content }),
        Some(url) => serde_json::json!({
            "role": m.role,
            "content": [
                { "type": "text", "text": m.content },
                { "type": "image_url", "image_url": { "url": url } }
            ]
        }),
    }
}

