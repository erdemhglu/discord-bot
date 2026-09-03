/// `/dusunme [kip]`. Input: `bot: &Bot`; `ctx: &Context`; `cmd: &CommandInteraction`.
/// Output: none. Uses: `option_text`, `ThinkingMode::from_arg`/`file_value`/`label`,
/// `bot.state()`, `memory::write`, `reply_info`. Registered as `run` for `"dusunme"` in
/// `registration_table.rs`.
async fn cmd_thinking(bot: &Bot, ctx: &Context, cmd: &CommandInteraction) {
    match option_text(cmd, "kip").and_then(ThinkingMode::from_arg) {
        Some(new_mode) => {
            bot.state().thinking_mode = new_mode;
            memory::write("dusunme.md", new_mode.file_value());
            reply_info(ctx, cmd, "Düşünme", &format!("düşünme artık {}", new_mode.label())).await;
        }
        None => {
            let mode = bot.state().thinking_mode;
            reply_info(
                ctx,
                cmd,
                "Düşünme",
                &format!(
                    "düşünme şu an {} · seçenekler: göster/gizle/sessiz/kapat",
                    mode.label()
                ),
            )
            .await;
        }
    }
}

/// `/model [id]`. Input: `bot: &Bot`; `ctx: &Context`; `cmd: &CommandInteraction`. Output:
/// none. Uses: `option_text`, `bot.state()`, `FAVORITE`, `defer`, `bot.model_exists`
/// (`command/remaining.rs`), `memory::write`, `reply_info`/`report_result`. Registered as
/// `run` for `"model"` in `registration_table.rs`.
async fn cmd_model(bot: &Bot, ctx: &Context, cmd: &CommandInteraction) {
    match option_text(cmd, "id") {
        None => {
            let m = bot.state().model.clone();
            reply_info(ctx, cmd, "Model", &format!("şu an {m}")).await;
        }
        Some(id) if cmd.user.id.get() != FAVORITE => {
            let _ = id;
            reply_info(ctx, cmd, "Model", "onu sen değiştiremezsin").await;
        }
        Some(id) => {
            let id = id.to_string();
            defer(ctx, cmd).await;
            // model_exists only queries openrouter's catalog; a custom router (API_URL)
            // or mistral's native api has no equivalent lookup, so the id is trusted as-is
            if bot.api_url.contains("openrouter") && !bot.model_exists(&id).await {
                report_result(ctx, cmd, "Model", "yok öyle model").await;
            } else {
                bot.state().model = id.clone();
                memory::write("model.md", &id);
                report_result(ctx, cmd, "Model", &format!("tamam, {id}")).await;
            }
        }
    }
}

/// `/debug [durum]`. Input: `bot: &Bot`; `ctx: &Context`; `cmd: &CommandInteraction`.
/// Output: none. Uses: `option_text`, `bot.set_debug` (`command/remaining.rs`),
/// `reply_info`. Registered as `run` for `"debug"` in `registration_table.rs`.
async fn cmd_debug(bot: &Bot, ctx: &Context, cmd: &CommandInteraction) {
    let arg = option_text(cmd, "durum").unwrap_or("");
    let enabled = bot.set_debug(arg);
    reply_info(
        ctx,
        cmd,
        "Debug",
        if enabled {
            "debug açık: kararlar bu kanala düşecek (DEBUG_CHANNEL ayarlıysa oraya)"
        } else {
            "debug kapalı"
        },
    )
    .await;
}
