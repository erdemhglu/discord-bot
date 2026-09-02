// ---------- yardımcılar ----------

fn simdi_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn ad(u: &User) -> String {
    u.global_name.clone().unwrap_or_else(|| u.name.clone())
}

fn hatirla(d: &mut Durum, isim: &str, metin: &str) {
    d.hafiza.push_back(format!("{isim}: {metin}"));
    if d.hafiza.len() > HAFIZA_BOYU {
        d.hafiza.pop_front();
    }
}

// uzun metni en çok sinir karakterlik parçalara böler: önce cümle sınırı,
// sonra boşluk, o da yoksa tam sınırdan sert keser; hiçbir şey atılmaz.
// dilim üzerinde yürür: tur başına ara tahsis yok, yalnız çıkan parça String olur
fn bol(metin: &str, sinir: usize) -> Vec<String> {
    let mut parcalar: Vec<String> = Vec::new();
    let mut kalan = metin.trim();
    while kalan.char_indices().nth(sinir).is_some() {
        let kes = kesim_noktasi(kalan, sinir); // bayt ofseti
        let bas = kalan[..kes].trim();
        if !bas.is_empty() {
            parcalar.push(bas.to_string());
        }
        kalan = kalan[kes..].trim_start();
    }
    if !kalan.is_empty() {
        parcalar.push(kalan.to_string());
    }
    parcalar
}

// ilk sinir karakter içindeki en iyi kesim yerinin BAYT ofseti; çok ufak parça
// çıkmasın diye cümle/boşluk kesimi sınırın dörtte birinden sonra değilse sert kese düşer
fn kesim_noktasi(metin: &str, sinir: usize) -> usize {
    let mut cumle = (0usize, 0usize); // (karakter sırası, bayt ofseti)
    let mut bosluk = (0usize, 0usize);
    let mut bitis = metin.len();
    for (i, (off, c)) in metin.char_indices().enumerate() {
        if i >= sinir {
            bitis = off;
            break;
        }
        if matches!(c, '.' | '!' | '?' | '\n') {
            cumle = (i + 1, off + c.len_utf8());
        } else if c == ' ' {
            bosluk = (i, off);
        }
    }
    let asgari = sinir / 4;
    if cumle.0 > asgari {
        cumle.1
    } else if bosluk.0 > asgari {
        bosluk.1
    } else {
        bitis
    }
}

// discord spoiler'ı; içindeki dik çizgiler kaçırılır ki spoiler bozulmasın
fn spoiler(metin: &str) -> String {
    format!("||{}||", metin.replace('|', "\\|"))
}

// kanalın geçmişine satır ekler ve dosyaya yazar; sohbet bitse, bot yeniden başlasa da kalır
fn kanal_not(d: &mut Durum, kanal: ChannelId, satir: String) {
    kanal_not_coklu(d, kanal, [satir]);
}

// birden çok satırı TEK dosya yazımıyla ekler. Cevap artık satır satır gittiği için
// her satıra ayrı yazım bütün kanal geçmişini tur başına 4-5 kez baştan yazıyordu.
fn kanal_not_coklu(d: &mut Durum, kanal: ChannelId, satirlar: impl IntoIterator<Item = String>) {
    let mut satirlar = satirlar.into_iter().peekable();
    if satirlar.peek().is_none() {
        return;
    }
    let g = d.kanal_gecmisi.entry(kanal).or_default();
    g.extend(satirlar);
    while g.len() > KANAL_GECMIS {
        g.pop_front();
    }
    // ara Vec yok: doğrudan tek String'e birleştirilir
    let mut icerik = String::new();
    for (i, satir) in g.iter().enumerate() {
        if i > 0 {
            icerik.push('\n');
        }
        icerik.push_str(satir);
    }
    hafiza::yaz(&format!("kanallar/{}.md", kanal.get()), &icerik);
}

fn son_mesajlar(d: &Durum, n: usize) -> String {
    let atla = d.hafiza.len().saturating_sub(n);
    let mut s = String::new();
    for (i, satir) in d.hafiza.iter().skip(atla).enumerate() {
        if i > 0 {
            s.push('\n');
        }
        s.push_str(satir);
    }
    s
}

// sohbeti eleştirmen/günlükçü/hoca'nın okuduğu düz döküme çevirir. Bot cevabı çok satırlı
// olabilir (her satır ayrı mesaj gitti, tepki satırı da içeride): önek HER satıra konur,
// yoksa ikinci ve sonraki satırlar gruptaki insanlara aitmiş gibi okunur
fn dokum(gecmis: &[Mesaj], bot_adi: &str) -> String {
    let mut s = String::new();
    let mut ilk = true;
    for m in gecmis {
        for satir in m.content.split('\n') {
            if !ilk {
                s.push('\n');
            }
            ilk = false;
            if m.role == "assistant" {
                s.push_str(bot_adi);
                s.push_str(": ");
            }
            s.push_str(satir);
        }
    }
    s
}

// karşılaştırma için küçük harfe indirir; İ→i̇ dönüşümünün eklediği birleşik
// noktayı atar ki "ŞAHİN"/"şahin" gibi adlar eşleşsin
fn kucult(s: &str) -> String {
    s.to_lowercase().replace('\u{0307}', "")
}

// modelin başa ekleyebildiği ad öneki ve tırnakları soyar; dilim döndürür.
// sıcak yolda (stream'de her edit) metnin tamamı klonlanıp lowercase edilmez:
// önek karşılaştırması yalnız ilk karakterlere, kucult'taki gibi birleşik nokta atılarak
fn soy<'a>(metin: &'a str, bot_adi: &str) -> &'a str {
    let mut m = metin.trim();
    // "isim: metin" kalıbını taklit edip başına kendi adını koyabiliyor
    let onek = format!("{bot_adi}:");
    let karakter = onek.chars().count();
    let bas: String = m
        .chars()
        .take(karakter)
        .flat_map(|c| c.to_lowercase())
        .filter(|&c| c != '\u{0307}')
        .collect();
    if bas.starts_with(&kucult(&onek)) {
        m = match m.char_indices().nth(karakter) {
            Some((off, _)) => m[off..].trim(),
            None => "",
        };
    }
    if m.chars().count() > 1 && m.starts_with('"') && m.ends_with('"') {
        m = &m[1..m.len() - 1];
    }
    m
}

// stream'siz yollar tek mesajla sınırlı: soy + 1900 kapak.
// stream yolu soy'dan sonra bol() ile bölerek gönderir, kırpma yok.
fn temizle(metin: String, bot_adi: &str) -> String {
    let m = soy(&metin, bot_adi);
    // sınırda bayt ofsetini bul, yerinde kes: ara collect yok
    match m.char_indices().nth(MESAJ_SINIRI) {
        Some((off, _)) => m[..off].to_string(),
        None => m.to_string(),
    }
}

// ---------- çıktı protokolü ----------

// Modelin cevabı satır bazlı bir protokoldür: her satır ayrı bir mesaj olarak gider,
// "tepki: 💀" satırı yazı yerine emoji tepkisi olur, tek başına "-" susma işaretidir.
// soy() uygulanmış metin üzerinde çalışılır, burada yeniden soyulmaz.
