mod ajanlar;
mod gelisim;
mod gundem;
mod hafiza;
mod komut;
mod loglama;
mod modal;
mod promptlar;
mod seyahat;
mod sohbet_cli;
mod uyku;

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ajanlar::rastgele_resim;
use promptlar::*;
use serde::{Deserialize, Serialize};
use serenity::all::*;
use serenity::async_trait;
use tokio::time::sleep;

// Bu dosyanın eski içeriği (sabitler, Durum/Bot/Handler, sohbet motoru, döngüler,
// discord olayları) konu bazlı dosyalara bölündü: src/bot/. `include!` kullanılır
// (gerçek `mod` değil) ki görünürlük/`use super::*` hiçbir yerde değişmesin — bu
// dosyalar main.rs'in aynı modülünde yazılmış gibi derlenir, ajanlar.rs/komut.rs/
// modal.rs/gundem.rs/sohbet_cli.rs/uyku.rs'nin mevcut erişimi aynen sürer.
include!("bot/tipler.rs");
include!("bot/metin.rs");
include!("bot/saglayici.rs");
include!("bot/sohbet.rs");
include!("bot/dongu.rs");
include!("bot/handler.rs");
include!("bot/kurulum.rs");

#[tokio::main]
async fn main() -> Result<(), Hata> {
    dotenvy::dotenv().ok();
    loglama::kur();
    // panikler log'a backtrace ile düşsün; spawn'lu döngülerde sessiz ölüm kalmasın
    std::panic::set_hook(Box::new(|bilgi| {
        let iz = std::backtrace::Backtrace::force_capture();
        log::error!("PANİK: {bilgi}\n{iz}");
    }));
    // `cargo run -- sohbet`: discord'a hiç bağlanmadan terminalden konuşma tezgâhı.
    // Token istemez, yalnız model anahtarı gerekir; anahtar yoksa tek satır hata + çıkış 1
    if std::env::args().nth(1).as_deref() == Some("sohbet") {
        let bot = match Bot::kur() {
            Ok(b) => b,
            Err(e) => {
                eprintln!("sohbet modu açılamadı: {e}");
                std::process::exit(1);
            }
        };
        bot.sohbet_cli().await;
        return Ok(());
    }
    let token = ayar("DISCORD_TOKEN")?;
    let bot = Bot::kur()?;

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler {
            bot,
            baslatildi: AtomicBool::new(false),
            duyuruldu: AtomicBool::new(false),
        })
        .await?;

    // ctrl+c veya sigterm gelince düzgün kapan
    let yonetici = client.shard_manager.clone();
    tokio::spawn(async move {
        kapanis_bekle().await;
        log::info!("kapanıyor");
        // döngüler yeni tur açmasın, bekçi yeniden başlatmasın
        KAPANIYOR.store(true, Ordering::SeqCst);
        yonetici.shutdown_all().await;
    });

    client.start().await?;
    Ok(())
}
include!("bot/testler.rs");
