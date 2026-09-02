fn sse_ayikla(satir: &str) -> Option<SseVeri> {
    let veri = satir.trim().strip_prefix("data:")?.trim();
    if veri == "[DONE]" {
        return Some(SseVeri {
            done: true,
            ..Default::default()
        });
    }
    let yanit: AkisYaniti = serde_json::from_str(veri).ok()?;
    let kullanim = yanit.usage;
    let parca = yanit.choices.into_iter().next().and_then(|s| {
        let metin = s.delta.content.unwrap_or_default();
        let dusunce = [s.delta.reasoning, s.delta.reasoning_content]
            .into_iter()
            .flatten()
            .find(|s| !s.is_empty())
            .unwrap_or_default();
        (!metin.is_empty() || !dusunce.is_empty()).then_some(Parca { metin, dusunce })
    });
    (parca.is_some() || kullanim.is_some()).then_some(SseVeri {
        parca,
        kullanim,
        done: false,
    })
}

// stream isteğinin okuyucusu; her çağrıda sıradaki parçayı verir, akış bitince None
struct AkisOkuyucu {
    cevap: reqwest::Response,
    tampon: Vec<u8>,        // henüz satıra bölünmemiş baytlar
    kuyruk: Vec<Parca>,     // çözülmüş, verilmeyi bekleyen parçalar
    kullanim: Kullanim,     // son chunk'tan toplanan token sayacı
    kategori: &'static str, // token metriği kırılımı için (!durum)
    done: bool,             // [DONE] görüldü mü (temiz kapanış işareti)
    bitti: bool,
}

impl AkisOkuyucu {
    async fn sonraki(&mut self) -> Result<Option<Parca>, Hata> {
        loop {
            if let Some(p) = self.kuyruk.pop() {
                return Ok(Some(p));
            }
            if self.bitti {
                if self.tampon.is_empty() {
                    return Ok(None);
                }
                // sonda satır sonu olmayan parça kalabilir
                let satir = String::from_utf8_lossy(&self.tampon).into_owned();
                self.tampon.clear();
                if let Some(v) = sse_ayikla(&satir) {
                    self.veri_uygula(&v);
                    if let Some(p) = v.parca {
                        return Ok(Some(p));
                    }
                }
                continue;
            }
            match self.cevap.chunk().await? {
                Some(p) => {
                    self.tampon.extend_from_slice(&p);
                    self.satirlari_isle();
                }
                None => self.bitti = true,
            }
        }
    }

    // done/usage yan etkilerini uygular; parça kuyruğa değil, döndürülür
    fn veri_uygula(&mut self, v: &SseVeri) {
        if v.done {
            self.done = true;
        }
        if let Some(k) = v.kullanim {
            self.kullanim.topla(k);
        }
    }

    // yalnız tam satırlar işlenir; eksik sondaki baytlar tamponda bekler
    // (utf-8 karakter chunk ortasında bölünse bile satır tamamlanınca çözülür)
    fn satirlari_isle(&mut self) {
        let mut sinir = 0;
        let mut veriler = Vec::new();
        for (i, b) in self.tampon.iter().enumerate() {
            if *b == b'\n' {
                let satir = String::from_utf8_lossy(&self.tampon[sinir..i]);
                if let Some(v) = sse_ayikla(&satir) {
                    veriler.push(v);
                }
                sinir = i + 1;
            }
        }
        self.tampon.drain(..sinir);
        let once = self.kuyruk.len();
        for v in veriler {
            self.veri_uygula(&v);
            if let Some(p) = v.parca {
                self.kuyruk.push(p);
            }
        }
        // yeni eklenenler geliş sırasında; pop ilk geleni versin diye ters çevrilir
        self.kuyruk[once..].reverse();
    }
}

// stream gönderiminin sonucu
enum AkisSonuc {
    Gonderildi(String), // son metin gönderildi
    Bos,                // akıştan kullanılır bir şey çıkmadı
    Sus,                // model susmayı seçti ("-"): hiçbir şey gitmez, geçmişe de girmez
}

// gonder_akis'in cevap bağlamı; argüman yığını yerine tek yapı
struct AkisBaglam<'a> {
    bot_adi: &'a str,
    yanit: Option<MessageId>,
    // emoji tepkisinin düşeceği mesaj; yanit koşullu olduğu için ayrı alan
    tepki_hedefi: Option<MessageId>,
    gecmis: &'a [Mesaj],
    talimat: &'a str,
    butce: Option<u32>,
}

