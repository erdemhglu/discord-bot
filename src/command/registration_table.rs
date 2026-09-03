// Slash command manager: a registration table (`definitions`) holding each command's
// name+description+options+runner in one place. `modal::register_commands` builds the
// Discord registration list from this table, and `interaction_create` (main.rs) looks up
// an incoming `Interaction::Command` in it by name and runs it. There are no `!`/text
// commands — the bot is only managed via slash (plain messages only ever feed the
// chat/memory pipeline).
//
// Discord expects a first response within 3s: local/fast commands return an embed
// directly, commands that make a network/model call first acknowledge with `defer` and
// then edit the result in with `report_result` (news/problem/prank/agents/wander/wake/sleep
// already post their own messages to the channel via `Bot::send`; the embed here is just a short "ok" note).

use super::*;
use std::future::Future;
use std::pin::Pin;
use std::sync::OnceLock;

pub const HELP: &str = "\
komutlar (hepsi slash):
`/sifirla [hepsi]` kanal yasağını ve açık sohbeti sıfırlar
`/haber` şimdi haber atar · `/sorun` kod derdi sorar · `/gez` gündem gezintisi yapar
`/saka` / `/hack` görsel şakası / hacklenmiş taklidi
`/ajanlar` profilci ve hocayı şimdi çalıştırır
`/uyan` uykuyu keser · `/uyu [saat]` test için uyutur
`/durum` evre, sayaçlar, model, düşünme, uyku, seyahat
`/zihin [test]` kişi/konu/olay kartı + menü/butonlarla detay modalları; `test` son 30 satırı hemen günlükçüye verir (zihin zinciri teşhisi)
`/dusunme [kip]` düşünme kipi (göster: cevapla spoiler'da · gizle: düşünürken \"Düşünüyorum...\", cevap sonra · sessiz: arka planda düşünür, hiç iz göstermez · kapat: istekler reasoning'siz)
`/model [id]` modeli gösterir/değiştirir (yalnız favori)
`/debug [durum]` karar izleri kanala düşer: isteklilik puanı/sebebi, hedef, ruh hali, sus/tepki, sohbet kapanışı
`/ayarlar` butonlu ayar paneli: düşünme kipi, debug, uyku";

// ---------- command registration table ----------

/// A boxed-future function pointer type for a command's runner: takes the shared `&Bot`,
/// the event `&Context`, and the `&CommandInteraction`, returns a pinned `()`-future.
type CommandFn = for<'a> fn(
    &'a Bot,
    &'a Context,
    &'a CommandInteraction,
) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

/// One slash command's full registration: `name`/`description` (Discord-facing, Turkish),
/// `options` (a fn building its `CreateCommandOption`s), `run` (its handler, see
/// `CommandFn`). Built by `define_command!` below; the full table lives in `definitions`
/// below.
pub struct CommandDefinition {
    pub name: &'static str,
    pub description: &'static str,
    pub options: fn() -> Vec<CreateCommandOption>,
    pub run: CommandFn,
}

/// Builds one `CommandDefinition`, wrapping `$f` (an `async fn(&Bot, &Context,
/// &CommandInteraction)`) into the boxed-future shape `CommandFn` requires.
/// Input: `$name`/`$description: &'static str`; `$options: fn() -> Vec<CreateCommandOption>`;
/// `$f` — the async handler function. Output: `CommandDefinition`. Used by: `definitions`
/// below, once per command.
macro_rules! define_command {
    ($name:expr, $description:expr, $options:expr, $f:expr) => {
        CommandDefinition {
            name: $name,
            description: $description,
            options: $options,
            run: |bot, ctx, interaction| Box::pin($f(bot, ctx, interaction)),
        }
    };
}

/// The single source of truth for every slash command.
/// Input: none. Output: `&'static [CommandDefinition]`, built once (`OnceLock`) and reused.
/// Uses: `define_command!`, each `options_*` fn (`registration_helpers.rs`), each `cmd_*`
/// fn (`cards.rs`/`actions.rs`/`settings.rs`). Used by: `modal::register_commands`
/// (registration), `Handler::interaction_create` (`handler_event.rs`, dispatch),
/// `command.rs`'s own tests.
pub fn definitions() -> &'static [CommandDefinition] {
    static TABLE: OnceLock<Vec<CommandDefinition>> = OnceLock::new();
    TABLE.get_or_init(|| {
        vec![
            define_command!(
                "durum",
                "Botun şu anki halini kart olarak gösterir",
                Vec::new,
                cmd_status
            ),
            define_command!(
                "yardim",
                "Komut listesini kart olarak gösterir",
                Vec::new,
                cmd_help
            ),
            define_command!(
                "zihin",
                "Botun bildiklerini interaktif kart + menü/butonlarla gösterir",
                options_mind,
                cmd_mind
            ),
            define_command!(
                "ayarlar",
                "Butonlu ayar paneli: düşünme kipi, debug, uyku",
                Vec::new,
                cmd_settings
            ),
            define_command!(
                "sifirla",
                "Kanal yasağını ve açık sohbeti sıfırlar",
                options_reset,
                cmd_reset
            ),
            define_command!("haber", "Şimdi haber atar", Vec::new, cmd_news),
            define_command!("sorun", "Kod derdi sorar", Vec::new, cmd_problem),
            define_command!("gez", "Gündem gezintisi yapar", Vec::new, cmd_wander),
            define_command!("saka", "Görsel şakası yapar", Vec::new, cmd_prank),
            define_command!("hack", "Hacklenmiş taklidi yapar", Vec::new, cmd_hack),
            define_command!(
                "ajanlar",
                "Profilci ve hocayı şimdi çalıştırır",
                Vec::new,
                cmd_agents
            ),
            define_command!("uyan", "Uykuyu keser", Vec::new, cmd_wake),
            define_command!("uyu", "Test için uyutur", options_sleep, cmd_sleep),
            define_command!(
                "dusunme",
                "Düşünme kipini gösterir/değiştirir",
                options_thinking,
                cmd_thinking
            ),
            define_command!(
                "model",
                "Modeli gösterir/değiştirir (yalnız favori)",
                options_model,
                cmd_model
            ),
            define_command!(
                "debug",
                "Karar izlerini kanala düşürür/kapatır",
                options_debug,
                cmd_debug
            ),
        ]
    })
}

// ---------- option definitions ----------

