/// Input: none. Output: `Vec<CreateCommandOption>` — `/sifirla`'s `hepsi` (boolean) option.
/// Used by: `registration_table.rs`'s `definitions`, as `/sifirla`'s `options`.
fn options_reset() -> Vec<CreateCommandOption> {
    vec![CreateCommandOption::new(
        CommandOptionType::Boolean,
        "hepsi",
        "tüm kanalları sıfırla (yoksa yalnız bu kanal)",
    )]
}

/// Input: none. Output: `Vec<CreateCommandOption>` — `/zihin`'s `test` (boolean) option.
/// Used by: `registration_table.rs`'s `definitions`, as `/zihin`'s `options`.
fn options_mind() -> Vec<CreateCommandOption> {
    vec![CreateCommandOption::new(
        CommandOptionType::Boolean,
        "test",
        "son 30 satırı hemen günlükçüye ver (zihin zinciri teşhisi)",
    )]
}

/// Input: none. Output: `Vec<CreateCommandOption>` — `/uyu`'s `saat` (integer, min 1)
/// option. Used by: `registration_table.rs`'s `definitions`, as `/uyu`'s `options`.
fn options_sleep() -> Vec<CreateCommandOption> {
    vec![CreateCommandOption::new(
        CommandOptionType::Integer,
        "saat",
        "kaç saat uyusun (varsayılan 8)",
    )
    .min_int_value(1)]
}

/// Input: none. Output: `Vec<CreateCommandOption>` — `/dusunme`'s `kip` (string choice)
/// option; the choice values must match `ThinkingMode::from_arg` exactly (see
/// `command.rs`'s `thinking_mode_options_match_from_arg` test). Used by:
/// `registration_table.rs`'s `definitions`, as `/dusunme`'s `options`.
fn options_thinking() -> Vec<CreateCommandOption> {
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

/// Input: none. Output: `Vec<CreateCommandOption>` — `/model`'s `id` (string) option. Used
/// by: `registration_table.rs`'s `definitions`, as `/model`'s `options`.
fn options_model() -> Vec<CreateCommandOption> {
    vec![CreateCommandOption::new(
        CommandOptionType::String,
        "id",
        "boşsa mevcut model gösterilir",
    )]
}

/// Input: none. Output: `Vec<CreateCommandOption>` — `/debug`'s `durum` (string choice)
/// option. Used by: `registration_table.rs`'s `definitions`, as `/debug`'s `options`.
fn options_debug() -> Vec<CreateCommandOption> {
    vec![
        CreateCommandOption::new(CommandOptionType::String, "durum", "boşsa tersine çevirir")
            .add_string_choice("aç", "ac")
            .add_string_choice("kapat", "kapat"),
    ]
}

// ---------- option-reading helpers ----------

/// Input: `cmd: &'a CommandInteraction`; `name: &str` — the option's name. Output:
/// `Option<&'a str>` — its string value, if present and of that type. Used by:
/// `settings.rs`'s `cmd_thinking`/`cmd_model`/`cmd_debug`.
fn option_text<'a>(cmd: &'a CommandInteraction, name: &str) -> Option<&'a str> {
    cmd.data
        .options()
        .into_iter()
        .find(|o| o.name == name)
        .and_then(|o| match o.value {
            ResolvedValue::String(s) => Some(s),
            _ => None,
        })
}

/// Input: `cmd: &CommandInteraction`; `name: &str`. Output: `Option<i64>` — its integer
/// value, if present. Used by: `actions.rs`'s `cmd_sleep`.
fn option_int(cmd: &CommandInteraction, name: &str) -> Option<i64> {
    cmd.data
        .options()
        .into_iter()
        .find(|o| o.name == name)
        .and_then(|o| match o.value {
            ResolvedValue::Integer(n) => Some(n),
            _ => None,
        })
}

/// Input: `cmd: &CommandInteraction`; `name: &str`. Output: `Option<bool>` — its boolean
/// value, if present. Used by: `actions.rs`'s `cmd_reset`, `cards.rs`'s `cmd_mind`.
fn option_bool(cmd: &CommandInteraction, name: &str) -> Option<bool> {
    cmd.data
        .options()
        .into_iter()
        .find(|o| o.name == name)
        .and_then(|o| match o.value {
            ResolvedValue::Boolean(b) => Some(b),
            _ => None,
        })
}

// ---------- response helpers ----------

// a ready-made `CreateInteractionResponseMessage` (durum/yardim/zihin/ayarlar) goes out as-is
/// Input: `ctx: &Context`; `cmd: &CommandInteraction`; `response:
/// CreateInteractionResponseMessage`. Output: none (logs on failure). Used by: `reply_info`
/// below, `cards.rs`'s `cmd_status`/`cmd_help`/`cmd_settings`/`cmd_mind` (fast-path commands).
async fn send_response(
    ctx: &Context,
    cmd: &CommandInteraction,
    response: CreateInteractionResponseMessage,
) {
    if let Err(e) = cmd
        .create_response(&ctx.http, CreateInteractionResponse::Message(response))
        .await
    {
        log::warn!("couldn't send command response [{}]: {e}", cmd.data.name);
    }
}

// a short info/acknowledgment embed; most commands use this
/// Input: `ctx: &Context`; `cmd: &CommandInteraction`; `title`/`description: &str`. Output:
/// none. Uses: `send_response`, `modal::info_embed`. Used by: `actions.rs`'s `cmd_reset`,
/// `settings.rs`'s `cmd_thinking`/`cmd_model`/`cmd_debug`.
async fn reply_info(ctx: &Context, cmd: &CommandInteraction, title: &str, description: &str) {
    send_response(
        ctx,
        cmd,
        CreateInteractionResponseMessage::new()
            .ephemeral(true)
            .embed(modal::info_embed(title, description)),
    )
    .await;
}

// commands that will make a network/model call call this first (so they don't miss the 3s limit)
/// Input: `ctx: &Context`; `cmd: &CommandInteraction`. Output: none (logs on failure). Used
/// by: every network/model-calling command in `actions.rs` (`cmd_news`/`cmd_problem`/
/// `cmd_wander`/`prank_shared`/`cmd_agents`/`cmd_wake`/`cmd_sleep`), `settings.rs`'s
/// `cmd_model`.
async fn defer(ctx: &Context, cmd: &CommandInteraction) {
    let msg = CreateInteractionResponseMessage::new().ephemeral(true);
    if let Err(e) = cmd
        .create_response(&ctx.http, CreateInteractionResponse::Defer(msg))
        .await
    {
        log::warn!("couldn't send defer [{}]: {e}", cmd.data.name);
    }
}

// writes the actual result after defer()
/// Input: `ctx: &Context`; `cmd: &CommandInteraction`; `title`/`description: &str`. Output:
/// none. Uses: `modal::info_embed`. Used by: the same commands as `defer` above, once the
/// deferred work finishes.
async fn report_result(ctx: &Context, cmd: &CommandInteraction, title: &str, description: &str) {
    let body = EditInteractionResponse::new().embed(modal::info_embed(title, description));
    if let Err(e) = cmd.edit_response(&ctx.http, body).await {
        log::warn!("couldn't update result [{}]: {e}", cmd.data.name);
    }
}

// ---------- commands ----------

