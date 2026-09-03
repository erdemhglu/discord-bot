/// `/sifirla [hepsi]`. Input: `bot: &Bot`; `ctx: &Context`; `cmd: &CommandInteraction`.
/// Output: none. Uses: `option_bool`, `bot.state()`, `reply_info`. Registered as `run` for
/// `"sifirla"` in `registration_table.rs`.
async fn cmd_reset(bot: &Bot, ctx: &Context, cmd: &CommandInteraction) {
    let all = option_bool(cmd, "hepsi").unwrap_or(false);
    {
        let mut state = bot.state();
        if all {
            state.chats.clear();
            state.awaiting_comment.clear();
            state.busy.clear();
        } else {
            state.chats.remove(&cmd.channel_id);
            state.awaiting_comment.remove(&cmd.channel_id);
            state.busy.remove(&cmd.channel_id);
        }
    }
    reply_info(
        ctx,
        cmd,
        "Sıfırlandı",
        if all {
            "tüm kanallar sıfırlandı"
        } else {
            "bu kanal sıfırlandı"
        },
    )
    .await;
}

/// `/haber`. Input: `bot: &Bot`; `ctx: &Context`; `cmd: &CommandInteraction`. Output: none.
/// Uses: `defer`, `bot.state()`, `bot.post_news` (`cycle_news.rs`), `report_result`.
/// Registered as `run` for `"haber"` in `registration_table.rs`.
async fn cmd_news(bot: &Bot, ctx: &Context, cmd: &CommandInteraction) {
    defer(ctx, cmd).await;
    bot.state().chats.remove(&cmd.channel_id);
    let found = bot.post_news(ctx, cmd.channel_id).await;
    report_result(
        ctx,
        cmd,
        "Haber",
        if found {
            "gönderildi"
        } else {
            "haber bulamadım"
        },
    )
    .await;
}

/// `/sorun`. Input: `bot: &Bot`; `ctx: &Context`; `cmd: &CommandInteraction`. Output: none.
/// Uses: `defer`, `bot.state()`, `bot.post_problem` (`cycle_news.rs`), `report_result`.
/// Registered as `run` for `"sorun"` in `registration_table.rs`.
async fn cmd_problem(bot: &Bot, ctx: &Context, cmd: &CommandInteraction) {
    defer(ctx, cmd).await;
    bot.state().chats.remove(&cmd.channel_id);
    bot.post_problem(ctx, cmd.channel_id).await;
    report_result(ctx, cmd, "Sorun", "gönderildi").await;
}

/// `/gez`. Input: `bot: &Bot`; `ctx: &Context`; `cmd: &CommandInteraction`. Output: none.
/// Uses: `defer`, `bot.wander` (`agenda.rs`), `report_result`. Registered as `run` for
/// `"gez"` in `registration_table.rs`.
async fn cmd_wander(bot: &Bot, ctx: &Context, cmd: &CommandInteraction) {
    defer(ctx, cmd).await;
    bot.wander().await;
    report_result(ctx, cmd, "Gezinti", "tamamlandı").await;
}

/// `/saka`. Input: `bot: &Bot`; `ctx: &Context`; `cmd: &CommandInteraction`. Output: none.
/// Uses: `prank_shared` below (`hack=false`). Registered as `run` for `"saka"` in
/// `registration_table.rs`.
async fn cmd_prank(bot: &Bot, ctx: &Context, cmd: &CommandInteraction) {
    prank_shared(bot, ctx, cmd, false).await;
}

/// `/hack`. Input: `bot: &Bot`; `ctx: &Context`; `cmd: &CommandInteraction`. Output: none.
/// Uses: `prank_shared` below (`hack=true`). Registered as `run` for `"hack"` in
/// `registration_table.rs`.
async fn cmd_hack(bot: &Bot, ctx: &Context, cmd: &CommandInteraction) {
    prank_shared(bot, ctx, cmd, true).await;
}

/// Shared body for `cmd_prank`/`cmd_hack` above.
/// Input: `bot: &Bot`; `ctx: &Context`; `cmd: &CommandInteraction`; `hack: bool`. Output:
/// none. Uses: `defer`, `bot.state()`, `bot.run_prank` (`cycle_news.rs`), `report_result`.
async fn prank_shared(bot: &Bot, ctx: &Context, cmd: &CommandInteraction, hack: bool) {
    defer(ctx, cmd).await;
    bot.state().chats.remove(&cmd.channel_id);
    bot.run_prank(ctx, cmd.channel_id, hack).await;
    report_result(ctx, cmd, if hack { "Hack" } else { "Şaka" }, "gönderildi").await;
}

/// `/ajanlar`. Input: `bot: &Bot`; `ctx: &Context`; `cmd: &CommandInteraction`. Output:
/// none. Uses: `defer`, `bot.profiler`/`coach` (`agents.rs`), `memory::refresh_index`,
/// `report_result`. Registered as `run` for `"ajanlar"` in `registration_table.rs`.
async fn cmd_agents(bot: &Bot, ctx: &Context, cmd: &CommandInteraction) {
    defer(ctx, cmd).await;
    bot.profiler().await;
    bot.coach().await;
    bot.state().index = memory::refresh_index();
    report_result(ctx, cmd, "Ajanlar", "profilci ve hoca çalıştı").await;
}

/// `/uyan`. Input: `bot: &Bot`; `ctx: &Context`; `cmd: &CommandInteraction`. Output: none.
/// Uses: `defer`, `bot.wake`/`sleep_transition` (`command/remaining.rs`/`cycle_sleep.rs`),
/// `report_result`. Registered as `run` for `"uyan"` in `registration_table.rs`.
async fn cmd_wake(bot: &Bot, ctx: &Context, cmd: &CommandInteraction) {
    defer(ctx, cmd).await;
    bot.wake();
    bot.sleep_transition(ctx).await;
    report_result(ctx, cmd, "Uyandı", "uyku kesildi").await;
}

/// `/uyu [saat]`. Input: `bot: &Bot`; `ctx: &Context`; `cmd: &CommandInteraction`. Output:
/// none. Uses: `defer`, `option_int`, `bot.put_to_sleep`/`sleep_transition`
/// (`command/remaining.rs`/`cycle_sleep.rs`), `report_result`. Registered as `run` for
/// `"uyu"` in `registration_table.rs`.
async fn cmd_sleep(bot: &Bot, ctx: &Context, cmd: &CommandInteraction) {
    defer(ctx, cmd).await;
    let hours = option_int(cmd, "saat").unwrap_or(8);
    bot.put_to_sleep(hours);
    bot.sleep_transition(ctx).await;
    report_result(ctx, cmd, "Uyutuldu", &format!("{hours} saat")).await;
}
