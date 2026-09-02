// Hafif loglama: `log` makroları + elle yazılmış sink. Seviye LOG_SEVIYE
// ortam değişkeninden (varsayılan info); renk terminalde otomatik, LOG_RENK=on|off
// ile dayatılır. Yalnız discord_bot kayıtları seviyeye göre geçer; yabancı
// crate'lerin (serenity, reqwest, ...) iç olayları konsolu sel basmasın diye
// yalnız warn/error seviyesinde gösterilir.

use log::{Level, LevelFilter, Log, Metadata, Record};
use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

fn numara_filtre(s: LevelFilter) -> u8 {
    match s {
        LevelFilter::Off => 0,
        LevelFilter::Error => 1,
        LevelFilter::Warn => 2,
        LevelFilter::Info => 3,
        LevelFilter::Debug => 4,
        LevelFilter::Trace => 5,
    }
}

fn numara_seviye(l: Level) -> u8 {
    match l {
        Level::Error => 1,
        Level::Warn => 2,
        Level::Info => 3,
        Level::Debug => 4,
        Level::Trace => 5,
    }
}

// 0=off 1=error 2=warn 3=info 4=debug 5=trace
static SEVIYE: AtomicU8 = AtomicU8::new(3);
static RENK: AtomicBool = AtomicBool::new(false);

const SIFIRLA: &str = "\x1b[0m";
const SOLUK: &str = "\x1b[2m";

fn seviye_oku() -> LevelFilter {
    match std::env::var("LOG_SEVIYE")
        .unwrap_or_default()
        .to_lowercase()
        .as_str()
    {
        "error" => LevelFilter::Error,
        "warn" => LevelFilter::Warn,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        "kapali" | "off" => LevelFilter::Off,
        _ => LevelFilter::Info,
    }
}

fn seviye_renk(l: Level) -> &'static str {
    match l {
        Level::Error => "\x1b[1;31m", // kırmızı, kalın
        Level::Warn => "\x1b[33m",    // sarı
        Level::Info => "\x1b[32m",    // yeşil
        Level::Debug | Level::Trace => SOLUK,
    }
}

fn bizim_hedef(target: &str) -> bool {
    target.starts_with("discord_bot")
}

struct Kutuk;

impl Log for Kutuk {
    fn enabled(&self, metadata: &Metadata) -> bool {
        let gecir = numara_seviye(metadata.level()) <= SEVIYE.load(Ordering::Relaxed);
        if bizim_hedef(metadata.target()) {
            gecir
        } else {
            // serenity/reqwest/... iç olayları: yalnız uyarı ve hatalar görünsün
            gecir && metadata.level() <= Level::Warn
        }
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let tarih = crate::hafiza::tarih();
        let saat = crate::uyku::saat();
        if RENK.load(Ordering::Relaxed) {
            let kod = seviye_renk(record.level());
            println!(
                "{SOLUK}{tarih} {saat}{SIFIRLA} {kod}{:<5}{SIFIRLA} {kod}{}{SIFIRLA}",
                record.level(),
                record.args()
            );
        } else {
            println!("{} {} {:<5} {}", tarih, saat, record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

static KUTUK: Kutuk = Kutuk;

pub fn kur() {
    let seviye = seviye_oku();
    SEVIYE.store(numara_filtre(seviye), Ordering::Relaxed);
    log::set_max_level(seviye);
    let renk = match std::env::var("LOG_RENK")
        .unwrap_or_default()
        .to_lowercase()
        .as_str()
    {
        "on" | "acik" => true,
        "off" | "kapali" => false,
        _ => std::io::stdout().is_terminal(),
    };
    RENK.store(renk, Ordering::Relaxed);
    let _ = log::set_logger(&KUTUK);
}
