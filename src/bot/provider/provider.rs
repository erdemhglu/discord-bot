// The AI call layer + sending. Split into six separate `impl Bot` blocks instead of one
// giant one (Rust lets you reopen an inherent impl as many times as you like); `include!`
// stays at the top level because each piece is a self-contained, reasonably sized file.
include!("provider_types.rs");
include!("provider_stream.rs");
include!("provider_reasoning.rs");
include!("provider_ask_raw.rs");
include!("provider_ask.rs");
include!("provider_generate.rs");
include!("provider_send_stream.rs");
include!("provider_send_line.rs");
include!("provider_stream_view.rs");
include!("provider_system.rs");
