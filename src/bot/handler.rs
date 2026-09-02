// Discord event handler. `impl EventHandler for Handler` tek trait impl olmak
// ZORUNDA (Rust aynı trait+tip için ikinci impl'e izin vermiyor, E0119) — bu yüzden
// handler_event.rs 200 satır sınırının üstünde kalıyor (struct+ready+guild_create+
// guild_member_addition+message+interaction_create tek blok). `impl Handler`
// (düşünce/ayar butonları) inherent impl olduğu için ayrı dosyada kalabildi.
include!("handler_event.rs");
include!("handler_dugmeler.rs");
