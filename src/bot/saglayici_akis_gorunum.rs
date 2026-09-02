// akış sürerken cevabın görünen kısmı: tamamlanmış satırlar (ardında \n olan) + son yarım
// satır ancak YARIM_SATIR_ESIGI karakteri geçtiyse. Böylece "tep" yarım hâlde mesaj olup
// bir sonraki düzenlemede silinmez, kısa satır için boşuna edit atılmaz.
fn akis_kesiti(cevap: &str, bitti: bool) -> &str {
    if bitti {
        return cevap;
    }
    let (tam, yarim) = match cevap.rfind('\n') {
        Some(i) => (&cevap[..i], &cevap[i + 1..]),
        None => ("", cevap),
    };
    if yarim.trim().chars().count() >= YARIM_SATIR_ESIGI {
        cevap
    } else {
        tam
    }
}

// thinking fazı: cevap henüz başlamadıysa ve model düşünüyorsa placeholder gider;
// gizlede canlı kelime sayacı, göstergede düz "Düşünüyorum...". Sessiz ve kapalıda hiçbir
// şey gitmez (sessizde reasoning arka planda çalışır ama iz bırakmaz, kapalıda zaten yok).
// Cevap başlayınca aynı mesaj düzenlenerek stream edilir. bitti=false ise akış sürüyor:
// yalnız tamamlanmış satırlar gösterilir.
fn akis_gorunum(kip: DusunmeKip, dusunce: &str, cevap: &str, bitti: bool) -> Vec<String> {
    let kesit = akis_kesiti(cevap, bitti);
    if kesit.trim().is_empty() && !dusunce.trim().is_empty() {
        return match kip {
            DusunmeKip::Gizle => vec![dusunce_sayaci(dusunce)],
            DusunmeKip::Goster => vec!["Düşünüyorum...".to_string()],
            DusunmeKip::Sessiz | DusunmeKip::Kapali => Vec::new(),
        };
    }
    akis_yerlesim(kip, dusunce, &cevap_parcala(kesit).satirlar)
}

// düşünce blokları + satır mesajları. Satırlar dışarıdan gelir: son yerleşimde
// gonder_akis tekrar elemesinden geçmiş hâllerini verir
fn akis_yerlesim(kip: DusunmeKip, dusunce: &str, satirlar: &[String]) -> Vec<String> {
    let dusunce = tek_satir(dusunce);
    let mut v: Vec<String> = Vec::new();
    if kip == DusunmeKip::Goster && !dusunce.is_empty() {
        // göster: hem spoiler hem kod bloğu
        for p in bol(&dusunce, MESAJ_SINIRI - 4) {
            v.push(spoiler(&p));
        }
        v.extend(kod_bloklari(&dusunce));
    }
    // gizle/sessiz/kapalı kiplerde düşünce yerleşime girmez; gizlede cevap sonunda
    // "Düşünce Sürecini Göster" butonu gider (gonder_akis ekler), sessizde hiç buton yok
    v.extend(satirlar.iter().cloned());
    v
}

// gizlede düşünürken canlı sayaç: kaçıncı kelimede olduğu görünür
fn dusunce_sayaci(dusunce: &str) -> String {
    let n = dusunce.split_whitespace().count();
    format!("Düşünüyorum... Şu ana kadar {n} kelime düşündüm.")
}

// thinking'in kod bloğu biçimi; 1900'ü aşarsa birden çok blok
fn kod_bloklari(metin: &str) -> Vec<String> {
    bol(metin, MESAJ_SINIRI - 10)
        .into_iter()
        .map(|p| format!("```\n{p}\n```"))
        .collect()
}

// butonla açılan ephemeral düşünce: tek mesaja sığacak şekilde kod bloğu
fn dusunce_gosterim(metin: &str) -> String {
    let not = "\n_(düşünce uzun, kısaltıldı)_";
    let sinir = MESAJ_SINIRI - 12 - not.chars().count();
    let toplam = metin.chars().count();
    let govde: String = metin.chars().take(sinir).collect();
    let mut s = format!("```\n{govde}\n```");
    if toplam > sinir {
        s.push_str(not);
    }
    s
}

// thinking'de her düşünce için newline atılmasın; tek akıcı satıra indirgenir
fn tek_satir(metin: &str) -> String {
    metin.split_whitespace().collect::<Vec<_>>().join(" ")
}

// yerleşimi açık mesajlarla uzlaştırır: değişenler düzenlenir, eksikler açılır,
// metin kısalırsa (ad öneki soyulması gibi) fazla mesajlar silinir
async fn yaz_akis(
    ctx: &Context,
    kanal: ChannelId,
    gonderilenler: &mut Vec<Message>,
    yerlesim: &[String],
    yanit: Option<MessageId>,
) {
    // typing edit döngüsünde yinelenmez: her tur atmak discord hız sınırına takılır;
    // "yazıyor" göstergesini cevapla, model çağrısından önce bir kez yollar
    for (i, icerik) in yerlesim.iter().enumerate() {
        match gonderilenler.get_mut(i) {
            Some(m) if m.content != *icerik => {
                if let Err(e) = m
                    .edit(&ctx.http, EditMessage::new().content(icerik.clone()))
                    .await
                {
                    log::warn!("düzenlenemedi ({kanal}): {e}");
                }
            }
            Some(_) => {}
            None => {
                let mut izin = CreateAllowedMentions::new();
                let mut mesaj = CreateMessage::new().content(icerik);
                if i == 0 {
                    if let Some(id) = yanit {
                        izin = izin.replied_user(true);
                        mesaj = mesaj.reference_message((kanal, id));
                    }
                }
                match kanal
                    .send_message(&ctx.http, mesaj.allowed_mentions(izin))
                    .await
                {
                    Ok(m) => gonderilenler.push(m),
                    Err(e) => {
                        log::error!("gönderilemedi ({kanal}): {e}");
                        break;
                    }
                }
            }
        }
    }
    while gonderilenler.len() > yerlesim.len() {
        if let Some(m) = gonderilenler.pop() {
            let _ = m.delete(&ctx.http).await;
        }
    }
}

async fn sil_mesajlar(ctx: &Context, mesajlar: Vec<Message>) {
    for m in mesajlar {
        let _ = m.delete(&ctx.http).await;
    }
}

// cache_control anthropic'in uydurduğu bir alan. OpenRouter'a giden her istekte güvenle
// eklenebilir: openrouter kendi birleşik şemasının bir parçası olarak kabul eder, hangi modelde
// gerçekten önbellekleyeceğine (claude, gemini, ...) kendi tarafında karar verir, desteklemeyen
// modelde sessizce yok sayar — model adını burada tahmin etmeye gerek yok. Mistral'in native
// API'si ya da `API_ADRES` ile verilen özel bir router aynı garantiyi vermez: bilinmeyen alanla
// isteği tümden reddedebilir, o yüzden yalnız openrouter adresine gidiyorsa eklenir.
