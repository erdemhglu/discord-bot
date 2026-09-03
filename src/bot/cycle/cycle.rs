// Growth + memory scanning + background cycles and the actions they trigger, split into
// five files.
include!("cycle_growth.rs");
include!("cycle_memory.rs");
include!("cycle_news.rs");
include!("cycle_background.rs");
include!("cycle_sleep.rs");
