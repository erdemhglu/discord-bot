// Discord event handler. `impl EventHandler for Handler` MUST be a single trait impl
// (Rust doesn't allow a second impl of the same trait for the same type, E0119) — that's
// why handler_event.rs stays over the 200-line guideline (struct + ready + guild_create +
// guild_member_addition + message + interaction_create all in one block). `impl Handler`
// (thinking/settings buttons) is an inherent impl, so it was free to live in its own file.
include!("handler_event.rs");
include!("handler_buttons.rs");
