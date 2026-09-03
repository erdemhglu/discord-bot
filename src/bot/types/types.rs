// Constants + core types (Message/Chat/ThinkingMode/State/Bot), split into four files to
// respect the 200-line-per-file guideline. `include!` (not `mod`): same reasoning as the
// `include!` pattern in main.rs.
include!("types_settings.rs");
include!("types_message.rs");
include!("types_chat_state.rs");
include!("types_bot.rs");
