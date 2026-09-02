// AI çağrı katmanı + gönderim. Tek dev `impl Bot` bloğu yerine altı ayrı `impl Bot`
// bloğuna bölündü (Rust inherent impl'i istediğin kadar tekrar açmana izin verir);
// `include!` her biri kendi başına dengeli bir dosya olduğu için üst seviyede kalır.
include!("saglayici_tipler.rs");
include!("saglayici_akis.rs");
include!("saglayici_reasoning.rs");
include!("saglayici_sor_ham.rs");
include!("saglayici_sor.rs");
include!("saglayici_uret.rs");
include!("saglayici_gonder_akis.rs");
include!("saglayici_gonder_satir.rs");
include!("saglayici_akis_gorunum.rs");
include!("saglayici_sistem.rs");
