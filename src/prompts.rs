// Prompts live as markdown under promptlar/ and get embedded at compile time (that
// directory and its filenames stay Turkish on purpose — see AGENTS.md). To change the
// text, edit the .md file and rebuild.

pub const PERSONALITY: &str = include_str!("../promptlar/kisilik.md");
pub const FAVORITE_LINE: &str = include_str!("../promptlar/favori-satiri.md");
pub const WELCOME: &str = include_str!("../promptlar/hos-geldin.md");
pub const OUT_OF_THE_BLUE: &str = include_str!("../promptlar/durup-dururken.md");
pub const NEWS_INTRO: &str = include_str!("../promptlar/haber-tanit.md");
pub const ANALYST: &str = include_str!("../promptlar/analist.md");
pub const WILLINGNESS: &str = include_str!("../promptlar/isteklilik.md");
pub const TARGET_PICK: &str = include_str!("../promptlar/hedef-sec.md");
pub const WAKING: &str = include_str!("../promptlar/uyanis.md");
pub const WAKING_REPLY: &str = include_str!("../promptlar/uyanis-cevap.md");
pub const PROFILE_EXTRACT: &str = include_str!("../promptlar/profil-cikar.md");
pub const NEWS_PICK: &str = include_str!("../promptlar/haber-sec.md");
pub const COACH: &str = include_str!("../promptlar/hoca.md");
pub const CRITIC: &str = include_str!("../promptlar/elestirmen.md");
pub const IMAGE_POST: &str = include_str!("../promptlar/resim-at.md");
pub const HACK_ENTER: &str = include_str!("../promptlar/hack-giris.md");
pub const HACK_CONTINUE: &str = include_str!("../promptlar/hack-devam.md");
pub const HACK_EXIT: &str = include_str!("../promptlar/hack-cikis.md");
pub const DIARIST: &str = include_str!("../promptlar/gunlukcu.md");
pub const SUMMARIZER_PERSON: &str = include_str!("../promptlar/ozetleyici-kisi.md");
pub const SUMMARIZER_TOPIC: &str = include_str!("../promptlar/ozetleyici-konu.md");
pub const SUMMARIZER_EVENTS: &str = include_str!("../promptlar/ozetleyici-olaylar.md");
pub const WANDERER_PICK: &str = include_str!("../promptlar/gezgin-sec.md");
pub const WANDERER_NOTE: &str = include_str!("../promptlar/gezgin-not.md");
pub const WOKE_UP: &str = include_str!("../promptlar/uyandim.md");
pub const ON_THE_WAY: &str = include_str!("../promptlar/yolda.md");
pub const LEAVING: &str = include_str!("../promptlar/gidiyorum.md");
pub const NAME_PICK: &str = include_str!("../promptlar/isim-sec.md");
pub const NAME_ANNOUNCE: &str = include_str!("../promptlar/isim-duyuru.md");
pub const PROBLEM: &str = include_str!("../promptlar/sorun.md");
pub const MOOD: &str = include_str!("../promptlar/ruh-hali.md");
