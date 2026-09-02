// Hafif loglama: `log` makroları + elle yazılmış sink. Seviye LOG_SEVIYE
// ortam değişkeninden (varsayılan info); zaman damgası hafiza/uyku'nun
// mevcut tarih-saat fonksiyonlarından, yeni bağımlılık yok.

use log::{Level, LevelFilter, Log, Metadata, Record};
use std::sync::atomic::{AtomicU8, Ordering};

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

struct Kutuk;

impl Log for Kutuk {
    fn enabled(&self, metadata: &Metadata) -> bool {
        numara_seviye(metadata.level()) <= SEVIYE.load(Ordering::Relaxed)
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!(
                "{} {} {:<5} {}",
                crate::hafiza::tarih(),
                crate::uyku::saat(),
                record.level(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

static KUTUK: Kutuk = Kutuk;

pub fn kur() {
    let seviye = seviye_oku();
    SEVIYE.store(numara_filtre(seviye), Ordering::Relaxed);
    log::set_max_level(seviye);
    let _ = log::set_logger(&KUTUK);
}
