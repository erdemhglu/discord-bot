fn secenekler_sifirla() -> Vec<CreateCommandOption> {
    vec![CreateCommandOption::new(
        CommandOptionType::Boolean,
        "hepsi",
        "tüm kanalları sıfırla (yoksa yalnız bu kanal)",
    )]
}

fn secenekler_zihin() -> Vec<CreateCommandOption> {
    vec![CreateCommandOption::new(
        CommandOptionType::Boolean,
        "test",
        "son 30 satırı hemen günlükçüye ver (zihin zinciri teşhisi)",
    )]
}

fn secenekler_uyu() -> Vec<CreateCommandOption> {
    vec![CreateCommandOption::new(
        CommandOptionType::Integer,
        "saat",
        "kaç saat uyusun (varsayılan 8)",
    )
    .min_int_value(1)]
}

fn secenekler_dusunme() -> Vec<CreateCommandOption> {
    vec![CreateCommandOption::new(
        CommandOptionType::String,
        "kip",
        "boşsa mevcut kip gösterilir",
    )
    .add_string_choice("göster", "goster")
    .add_string_choice("gizle", "gizle")
    .add_string_choice("sessiz", "sessiz")
    .add_string_choice("kapat", "kapat")]
}

fn secenekler_model() -> Vec<CreateCommandOption> {
    vec![CreateCommandOption::new(
        CommandOptionType::String,
        "id",
        "boşsa mevcut model gösterilir",
    )]
}

fn secenekler_debug() -> Vec<CreateCommandOption> {
    vec![
        CreateCommandOption::new(CommandOptionType::String, "durum", "boşsa tersine çevirir")
            .add_string_choice("aç", "ac")
            .add_string_choice("kapat", "kapat"),
    ]
}

// ---------- seçenek okuma yardımcıları ----------

fn secenek_metin<'a>(c: &'a CommandInteraction, ad: &str) -> Option<&'a str> {
    c.data
        .options()
        .into_iter()
        .find(|o| o.name == ad)
        .and_then(|o| match o.value {
            ResolvedValue::String(s) => Some(s),
            _ => None,
        })
}

fn secenek_tam(c: &CommandInteraction, ad: &str) -> Option<i64> {
    c.data
        .options()
        .into_iter()
        .find(|o| o.name == ad)
        .and_then(|o| match o.value {
            ResolvedValue::Integer(n) => Some(n),
            _ => None,
        })
}

fn secenek_bool(c: &CommandInteraction, ad: &str) -> Option<bool> {
    c.data
        .options()
        .into_iter()
        .find(|o| o.name == ad)
        .and_then(|o| match o.value {
            ResolvedValue::Boolean(b) => Some(b),
            _ => None,
        })
}

// ---------- yanıt yardımcıları ----------

// bilinen `CreateInteractionResponseMessage` (durum/yardım/zihin/ayarlar) doğrudan gider
async fn yanit_gonder(
    ctx: &Context,
    c: &CommandInteraction,
    yanit: CreateInteractionResponseMessage,
) {
    if let Err(e) = c
        .create_response(&ctx.http, CreateInteractionResponse::Message(yanit))
        .await
    {
        log::warn!("komut yanıtı gönderilemedi [{}]: {e}", c.data.name);
    }
}

// kısa embed'lik bilgi/onay metni; komutların çoğu bunu kullanır
async fn yanit_bilgi(ctx: &Context, c: &CommandInteraction, baslik: &str, aciklama: &str) {
    yanit_gonder(
        ctx,
        c,
        CreateInteractionResponseMessage::new()
            .ephemeral(true)
            .embed(modal::bilgi_embed(baslik, aciklama)),
    )
    .await;
}

// ağ/model çağrısı yapacak komutlar önce bunu çağırır (3 sn sınırını aşmasın)
async fn ertele(ctx: &Context, c: &CommandInteraction) {
    let mesaj = CreateInteractionResponseMessage::new().ephemeral(true);
    if let Err(e) = c
        .create_response(&ctx.http, CreateInteractionResponse::Defer(mesaj))
        .await
    {
        log::warn!("erteleme gönderilemedi [{}]: {e}", c.data.name);
    }
}

// ertele'den sonra asıl sonucu yazar
async fn sonucu_bildir(ctx: &Context, c: &CommandInteraction, baslik: &str, aciklama: &str) {
    let govde = EditInteractionResponse::new().embed(modal::bilgi_embed(baslik, aciklama));
    if let Err(e) = c.edit_response(&ctx.http, govde).await {
        log::warn!("sonuç güncellenemedi [{}]: {e}", c.data.name);
    }
}

// ---------- komutlar ----------

