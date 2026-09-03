// File-based memory, the "second brain" logic:
//   - INDEX.md   the list of what it knows; sent with every reply (a pointer, not the data itself)
//   - kisiler/   one file per person; only people talking in this chat get pulled in
//   - konular/   one file per topic; pulled in by keyword match against the chat
//   - olaylar/   one line per finished chat, filed by month
//   - arsiv/     raw chunks summarized out of files that hit their limit (nothing is ever deleted)
// Once a file hits its limit, the summarizer agent (agents.rs) shrinks it. The context
// window doesn't grow: every reply gets the index plus whatever was retrieved for that
// chat, so the budget stays constant.

use super::*;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

// writes go through a single lock and land atomically: two agents hitting the same file,
// or the process dying mid-write, can't leave a half-written/corrupt file (temp file +
// rename, atomic on the same filesystem)
static WRITE_LOCK: Mutex<()> = Mutex::new(());

pub const PERSON_LIMIT: usize = 1800; // a person file gets summarized past this size
pub const PERSON_TARGET: usize = 1000; // target size after summarizing
pub const TOPIC_LIMIT: usize = 1500;
pub const TOPIC_TARGET: usize = 800;
pub const EVENT_LIMIT: usize = 6000; // a month's event file gets its older half summarized past this size
pub const CONTEXT_BUDGET: usize = 6000; // total characters of memory retrieved for one reply
pub const INDEX_PEOPLE: usize = 40; // max people shown in the index
pub const FAVORITE_NOTE: &str = "canın ciğerin, ne yaparsa yapsın arkasındasın";

// ---------- file operations ----------

/// Input: `rel: &str` — a path relative to `STATE_DIR` (e.g. `"kisiler/1.md"`). Output:
/// `PathBuf` — `STATE_DIR/rel`. Used throughout this module and by `over_limit` below.
pub fn path(rel: &str) -> PathBuf {
    Path::new(STATE_DIR).join(rel)
}

/// Input: `rel: &str`. Output: `String` — the file's contents, or `""` if it doesn't exist
/// or can't be read. Uses: `path`. Used throughout the crate (`State::load`, agents,
/// commands) wherever a `durum/` file is read.
pub fn read(rel: &str) -> String {
    fs::read_to_string(path(rel)).unwrap_or_default()
}

// writing to a temp file then renaming is atomic: even on a crash/kill, no reader ever
// sees half-written content (`fs::write` alone doesn't guarantee that). The temp name is
// fixed: WRITE_LOCK guarantees a single writer, so there's no need for a counter/pid to keep it unique
/// Input: `rel: &str`; `content: &str` — the full new file content. Output: none (failures
/// are logged, not propagated). Uses: `path`, `WRITE_LOCK`. Used throughout the crate
/// wherever a whole `durum/` file is rewritten.
pub fn write(rel: &str, content: &str) {
    let _lock = WRITE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let p = path(rel);
    if let Some(parent) = p.parent() {
        let _ = fs::create_dir_all(parent);
    }
    // write to a temp file, then rename: a half-written file is never visible
    let tmp = p.with_extension("tmp");
    let result = fs::write(&tmp, content).and_then(|_| fs::rename(&tmp, &p));
    if let Err(e) = result {
        let _ = fs::remove_file(&tmp);
        log::error!("couldn't write {}: {e}", p.display());
    }
}

// a real append instead of rewriting the whole file with a read+write
/// Input: `rel: &str`; `line: &str` — one line to add (a trailing `\n` is added). Output:
/// none (failures are logged). Uses: `path`, `WRITE_LOCK`. Used by: `archive`/`add_event`
/// below.
fn append(rel: &str, line: &str) {
    let _lock = WRITE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let p = path(rel);
    if let Some(parent) = p.parent() {
        let _ = fs::create_dir_all(parent);
    }
    // two write_all calls: no intermediate allocation just to format the line
    let result = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&p)
        .and_then(|mut f| {
            f.write_all(line.as_bytes())
                .and_then(|_| f.write_all(b"\n"))
        });
    if let Err(e) = result {
        log::error!("couldn't append to {}: {e}", p.display());
    }
}

// a raw chunk dropped by summarizing goes to the archive; nothing is ever deleted
/// Input: `rel: &str` — the original file's path, relative to `STATE_DIR`; `content: &str`
/// — the chunk being dropped. Output: none. Uses: `append`, `date_time`. Used by:
/// `Bot::summarizer` (`agents.rs`), whenever a file shrinks.
pub fn archive(rel: &str, content: &str) {
    append(
        &format!("arsiv/{rel}"),
        &format!("\n## {} öncesi\n{}", date_time(), content.trim_end()),
    );
}

/// Input: `name: &str`. Output: `String` — a lowercase, Turkish-letter-simplified,
/// hyphen-separated slug (e.g. `"Emin Şeyrek"` → `"emin-seyrek"`), or `"bilinmeyen"` if
/// nothing alphanumeric remains. Used by: `add_topic` below.
pub fn slug(name: &str) -> String {
    let mut s = String::new();
    for c in name.chars().flat_map(|c| c.to_lowercase()) {
        let c = match c {
            'ç' => 'c',
            'ğ' => 'g',
            'ı' => 'i',
            'ö' => 'o',
            'ş' => 's',
            'ü' => 'u',
            'â' => 'a',
            'î' => 'i',
            'û' => 'u',
            c => c,
        };
        if c.is_ascii_alphanumeric() {
            s.push(c);
        } else if !s.is_empty() && !s.ends_with('-') {
            s.push('-');
        }
    }
    let s = s.trim_end_matches('-').to_string();
    if s.is_empty() {
        "bilinmeyen".to_string()
    } else {
        s
    }
}

// YYYY-MM-DD, no external crate
/// Input: none. Output: `String` — today's date, `YYYY-MM-DD`. Uses: `date_from_unix`,
/// `now_unix`. Used by: `date_time` below, `month` below, `modal::mind_embeds`.
pub fn date() -> String {
    date_from_unix(now_unix())
}

// every record lands with a seconds-precision timestamp: YYYY-MM-DD HH:MM:SS
/// Input: none. Output: `String` — `"YYYY-MM-DD HH:MM:SS"`. Uses: `date`,
/// `sleep::time_of_day`. Used by: `archive` above, `add_topic`/`add_event` below,
/// `Bot::diarist` (`agents.rs`).
pub fn date_time() -> String {
    format!("{} {}", date(), crate::sleep::time_of_day())
}

/// Input: `unix: i64` — a Unix timestamp. Output: `String` — `"YYYY-MM-DD"` in UTC, via
/// Howard Hinnant's `civil_from_days` algorithm (no external crate). Used by: `date` above,
/// `travel::year_of`/`day_number`-adjacent tests, `modal::status_message`.
pub fn date_from_unix(unix: i64) -> String {
    let z = unix.div_euclid(86400) + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}")
}

/// Input: none. Output: `String` — `"YYYY-MM"` (the first 7 characters of `date()`). Used
/// by: `add_event`/`over_limit`/`retrieve` below, `Bot::send_news`/`news_cycle`
/// (`cycle_news.rs`, `awaiting_comment` window's event file name).
pub fn month() -> String {
    date()[..7].to_string()
}

// ---------- person file ----------

/// One `kisiler/<id>.md` file's parsed contents. Holds `id` (file key, Discord user id),
/// `name`/`username`/`previous_names` (identity), `score` (-10..10), `tags`, `note`,
/// `facts`/`events` (the two bulleted lists). Round-trips through `parse`/`text` below.
#[derive(Default, Clone)]
pub struct Person {
    pub id: u64,                     // file key; the discord user id
    pub name: String,                // display name (global_name or username)
    pub username: String,            // discord username
    pub previous_names: Vec<String>, // earlier display names
    pub score: i32,
    pub tags: Vec<String>,
    pub note: String,
    pub facts: Vec<String>,
    pub events: Vec<String>,
}

impl Person {
    /// Input: `id: u64`; `text: &str` — a `kisiler/<id>.md` file's contents. Output:
    /// `Person`. Unknown lines are ignored; the field prefixes it looks for
    /// (`kullanici_adi:`, `eski_adlar:`, `puan:`, `etiket:`, `not:`) are Turkish on purpose
    /// — they're the on-disk format (see `docs/durum-dosyalari.md`), not translated so
    /// existing data keeps parsing. Used by: `read_person`/`person_summaries` below,
    /// `Bot::diarist` (`agents.rs`).
    pub fn parse(id: u64, text: &str) -> Person {
        let mut p = Person {
            id,
            ..Default::default()
        };
        let mut section = "";
        for line in text.lines() {
            let s = line.trim();
            if let Some(title) = s.strip_prefix("# ") {
                p.name = title.trim().to_string();
            } else if let Some(title) = s.strip_prefix("## ") {
                section = if title.starts_with("Bildik") {
                    "bilgi"
                } else if title.starts_with("Son") {
                    "olay"
                } else {
                    ""
                };
            } else if let Some(v) = s.strip_prefix("id:") {
                p.id = v.trim().parse().unwrap_or(p.id);
            } else if let Some(v) = s.strip_prefix("kullanici_adi:") {
                p.username = v.trim().to_string();
            } else if let Some(v) = s.strip_prefix("eski_adlar:") {
                p.previous_names = v
                    .split(',')
                    .map(|e| e.trim().to_string())
                    .filter(|e| !e.is_empty())
                    .collect();
            } else if let Some(v) = s.strip_prefix("puan:") {
                p.score = v.trim().parse().unwrap_or(0);
            } else if let Some(v) = s.strip_prefix("etiket:") {
                p.tags = v
                    .split(',')
                    .map(|e| e.trim().to_string())
                    .filter(|e| !e.is_empty())
                    .collect();
            } else if let Some(v) = s.strip_prefix("not:") {
                p.note = v.trim().to_string();
            } else if let Some(item) = s.strip_prefix("- ") {
                match section {
                    "bilgi" => p.facts.push(item.to_string()),
                    "olay" => p.events.push(item.to_string()),
                    _ => {}
                }
            }
        }
        p
    }

    /// Input: `&self`. Output: `String` — the `kisiler/<id>.md` file format (inverse of
    /// `parse`). Used by: `write_person` below.
    pub fn text(&self) -> String {
        let mut s = format!(
            "# {}\nid: {}\nkullanici_adi: {}\neski_adlar: {}\npuan: {:+}\netiket: {}\nnot: {}\n\n## Bildiklerin\n",
            self.name,
            self.id,
            self.username,
            self.previous_names.join(", "),
            self.score,
            self.tags.join(", "),
            self.note
        );
        for f in &self.facts {
            s += &format!("- {f}\n");
        }
        s += "\n## Son olaylar\n";
        for e in &self.events {
            s += &format!("- {e}\n");
        }
        s
    }
}

/// Input: `id: u64`. Output: `Person` — parsed from `kisiler/<id>.md`, or an empty
/// `Person{id, ..}` if the file doesn't exist. Uses: `read`, `Person::parse`. Used by:
/// `Bot::diarist` (`agents.rs`), `modal::person_modal`, `retrieve` below (via `read`
/// directly, not this — see that function).
pub fn read_person(id: u64) -> Person {
    let m = read(&format!("kisiler/{id}.md"));
    if m.is_empty() {
        Person {
            id,
            ..Default::default()
        }
    } else {
        Person::parse(id, &m)
    }
}

/// Input: `p: &Person`. Output: none (writes `kisiler/<p.id>.md`). Uses: `write`,
/// `Person::text`. Used by: `Bot::diarist` (`agents.rs`), the only caller.
pub fn write_person(p: &Person) {
    write(&format!("kisiler/{}.md", p.id), &p.text());
}

// ---------- topics and events ----------

// the check + header + line all happen under a single lock: two concurrent calls can't
// duplicate the header or clobber each other's line (there used to be a gap between read->write->append)
/// Input: `name: &str` — the topic name (used verbatim as the `# ` header on a new file,
/// slugified for the filename); `note: &str` — the dated note to append. Output: none.
/// Uses: `path`, `WRITE_LOCK`, `slug`, `date_time`. Used by: `Bot::diarist` (`agents.rs`),
/// the only caller.
pub fn add_topic(name: &str, note: &str) {
    let _lock = WRITE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let p = path(&format!("konular/{}.md", slug(name)));
    if let Some(parent) = p.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let empty = fs::metadata(&p).map(|m| m.len() == 0).unwrap_or(true);
    let result = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&p)
        .and_then(|mut f| {
            let mut data = String::new();
            if empty {
                data.push_str(&format!("# {name}\netiket: \n\n"));
            }
            data.push_str(&format!("- {}: {}\n", date_time(), note.trim()));
            f.write_all(data.as_bytes())
        });
    if let Err(e) = result {
        log::error!("couldn't append to {}: {e}", p.display());
    }
}

/// Input: `channel: &str` — the channel name to record; `event: &str` — the one-line
/// summary. Output: none (appends to `olaylar/<current month>.md`). Uses: `append`,
/// `month`, `date_time`. Used by: `Bot::diarist` (`agents.rs`), the only caller.
pub fn add_event(channel: &str, event: &str) {
    append(
        &format!("olaylar/{}.md", month()),
        &format!("- {} #{}: {}", date_time(), channel, event.trim()),
    );
}

// summaries for the modal display (mtime order, most recently changed first)

/// Input: none. Output: `Vec<Person>` — every `kisiler/*.md` file with a valid numeric id
/// and non-empty name, most recently changed first. Uses: `files`, `Person::parse`. Used
/// by: `modal::mind_embeds`/`person_options`, the `/zihin` card.
pub fn person_summaries() -> Vec<Person> {
    let mut list = Vec::new();
    for path in files("kisiler") {
        // filename is id-based; old slug-named files (id can't be parsed) are skipped
        let Some(id) = path
            .file_stem()
            .and_then(|f| f.to_str())
            .and_then(|f| f.parse::<u64>().ok())
        else {
            continue;
        };
        let person = Person::parse(id, &fs::read_to_string(&path).unwrap_or_default());
        if person.name.is_empty() {
            continue;
        }
        list.push(person);
    }
    list
}

// (topic name, latest note)
/// Input: none. Output: `Vec<(String, String)>` — up to 30 topics as (name, latest note),
/// most recently changed first. Uses: `files`, `first_line`. Used by: `modal::mind_embeds`/
/// `topics_modal`.
pub fn topic_summaries() -> Vec<(String, String)> {
    files("konular")
        .into_iter()
        .take(30)
        .map(|path| {
            let content = fs::read_to_string(&path).unwrap_or_default();
            let latest = content
                .lines()
                .rev()
                .find(|l| l.starts_with("- "))
                .map(|l| l.trim_start_matches("- ").to_string())
                .unwrap_or_default();
            (first_line(&path), latest)
        })
        .collect()
}

// event records for the last `month_count` months: (month, "- " lines); newest month first.
// looking only at the current month gave an empty view at the start of a new month
/// Input: `month_count: usize`. Output: `Vec<(String, Vec<String>)>` — up to `month_count`
/// months as (`"YYYY-MM"`, its `"- "` lines), newest first. Uses: `files`. Used by:
/// `modal::mind_embeds`/`events_modal`.
pub fn event_months(month_count: usize) -> Vec<(String, Vec<String>)> {
    files("olaylar")
        .into_iter()
        .take(month_count)
        .filter_map(|path| {
            let month = path.file_stem()?.to_str()?.to_string();
            let lines: Vec<String> = fs::read_to_string(&path)
                .unwrap_or_default()
                .lines()
                .filter(|l| l.starts_with("- "))
                .map(|l| l.to_string())
                .collect();
            Some((month, lines))
        })
        .collect()
}

// ---------- channel history ----------

// reads durum/kanallar/<id>.md files: (channel id, recent lines)
/// Input: none. Output: `Vec<(u64, VecDeque<String>)>` — every channel history file as
/// (channel id, non-empty lines). Uses: `files`. Used by: `State::load`
/// (`types_chat_state.rs`), the only caller (once, at startup).
pub fn load_channel_history() -> Vec<(u64, VecDeque<String>)> {
    let mut list = Vec::new();
    for path in files("kanallar") {
        let Some(id) = path
            .file_stem()
            .and_then(|f| f.to_str())
            .and_then(|f| f.parse::<u64>().ok())
        else {
            continue;
        };
        let lines: VecDeque<String> = fs::read_to_string(&path)
            .unwrap_or_default()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.to_string())
            .collect();
        list.push((id, lines));
    }
    list
}

// ---------- index ----------

/// Input: `folder: &str` — a subdirectory of `STATE_DIR` (e.g. `"kisiler"`). Output:
/// `Vec<PathBuf>` — its `.md` files, most recently modified first. Used by:
/// `person_summaries`/`topic_summaries`/`event_months`/`load_channel_history`/
/// `refresh_index`/`retrieve`/`over_limit` — every listing function in this module.
fn files(folder: &str) -> Vec<PathBuf> {
    let mut list: Vec<(std::time::SystemTime, PathBuf)> = fs::read_dir(path(folder))
        .map(|dir| {
            dir.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|u| u == "md"))
                .filter_map(|p| Some((fs::metadata(&p).ok()?.modified().ok()?, p)))
                .collect()
        })
        .unwrap_or_default();
    list.sort_by_key(|e| std::cmp::Reverse(e.0)); // most recently changed first
    list.into_iter().map(|(_, p)| p).collect()
}

/// Input: `path: &Path`. Output: `String` — the file's first line, with a leading `"# "`
/// stripped. Used by: `topic_summaries` above.
fn first_line(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .next()
        .unwrap_or("")
        .trim_start_matches("# ")
        .to_string()
}

// the index display only needs the header fields; parsed without building the
// facts/events Vecs (cheaper than Person::parse's full parse)
/// A cheaper partial parse of a person file: just `name`/`score`/`tags`/`note`, no
/// `facts`/`events`. Produced by `person_header` below; used only by `refresh_index`.
struct PersonHeader {
    name: String,
    score: i32,
    tags: Vec<String>,
    note: String,
}

/// Input: `text: &str` — a `kisiler/<id>.md` file's contents. Output: `PersonHeader`.
/// Stops at the first `## ` heading (never descends into the `facts`/`events` lists).
/// Used by: `refresh_index` below, the only caller.
fn person_header(text: &str) -> PersonHeader {
    let mut header = PersonHeader {
        name: String::new(),
        score: 0,
        tags: Vec::new(),
        note: String::new(),
    };
    for line in text.lines() {
        let s = line.trim();
        if s.starts_with("## ") {
            break; // header is over, don't descend into the lists
        }
        if let Some(title) = s.strip_prefix("# ") {
            header.name = title.trim().to_string();
        } else if let Some(v) = s.strip_prefix("puan:") {
            header.score = v.trim().parse().unwrap_or(0);
        } else if let Some(v) = s.strip_prefix("etiket:") {
            header.tags = v
                .split(',')
                .map(|e| e.trim().to_string())
                .filter(|e| !e.is_empty())
                .collect();
        } else if let Some(v) = s.strip_prefix("not:") {
            header.note = v.trim().to_string();
        }
    }
    header
}

// regenerates and returns INDEX.md: the "what I know" list sent with every reply
/// Input: none. Output: `String` — the regenerated `INDEX.md` content (also written to
/// disk). Uses: `files`, `person_header`, `write`. Used by: `State::load`
/// (`types_chat_state.rs`), `Bot::diarist`/`summarizer` (`agents.rs`),
/// `Bot::cmd_agents` (`command/actions.rs`) — anywhere the person/topic/event listing
/// might have changed.
pub fn refresh_index() -> String {
    use std::fmt::Write as _;
    let mut output = String::from("## Kişiler\n");
    for path in files("kisiler").into_iter().take(INDEX_PEOPLE) {
        // filename is id-based; old slug-named files (id can't be parsed) are skipped
        let has_id = path
            .file_stem()
            .and_then(|f| f.to_str())
            .is_some_and(|f| f.parse::<u64>().is_ok());
        if !has_id {
            continue;
        }
        let header = person_header(&fs::read_to_string(&path).unwrap_or_default());
        if header.name.is_empty() {
            continue;
        }
        let _ = write!(output, "- {} ({:+})", header.name, header.score);
        if !header.tags.is_empty() {
            let _ = write!(output, " · {}", header.tags.join(", "));
        }
        if !header.note.is_empty() {
            let _ = write!(output, " · {}", header.note);
        }
        output.push('\n');
    }
    output.push_str("\n## Konular\n");
    for path in files("konular").into_iter().take(30) {
        // a single read: the name and latest note both come from the same content
        let content = fs::read_to_string(&path).unwrap_or_default();
        let name = content
            .lines()
            .next()
            .unwrap_or("")
            .trim_start_matches("# ");
        let latest = content
            .lines()
            .rev()
            .find(|l| l.starts_with("- "))
            .and_then(|l| l.get(2..12))
            .unwrap_or("");
        let _ = writeln!(output, "- {name} · son: {latest}");
    }
    output.push_str("\n## Olaylar\n");
    for path in files("olaylar").into_iter().take(3) {
        let n = fs::read_to_string(&path)
            .unwrap_or_default()
            .lines()
            .filter(|l| l.starts_with("- "))
            .count();
        let _ = writeln!(
            output,
            "- {} · {n} kayıt",
            path.file_stem().and_then(|f| f.to_str()).unwrap_or("")
        );
    }
    write("INDEX.md", &output);
    output
}

// ---------- retrieval ----------

const STOPWORDS: &[&str] = &[
    "için", "gibi", "değil", "bence", "yani", "falan", "filan", "diye", "olan", "daha", "böyle",
    "şöyle", "nasıl", "neden", "niye", "sonra", "önce", "şimdi", "bugün", "yarın", "zaten", "hala",
    "hâlâ", "bile", "kadar", "biraz", "bayağı", "aynen", "tamam", "evet", "hayır", "olur", "oldu",
    "olsun", "yapsın", "yaptı", "bunu", "şunu", "onun", "bunun", "bana", "sana", "beni", "seni",
    "bizi", "sizi", "kendi", "hangi", "nerede", "burada", "orada", "http", "https", "that", "this",
    "with", "have", "what", "just", "like", "abi", "lan", "amk", "aga", "reis",
];

// search keywords from chat text: 4+ letters, common words filtered out
/// Input: `texts: &[String]` — recent chat lines. Output: `Vec<String>` — up to 40 distinct
/// lowercase words of 4+ letters, `STOPWORDS` excluded. Uses: `STOPWORDS`. Used by:
/// `Bot::chat_system` (`provider_generate.rs`), feeding `retrieve` below.
pub fn keywords(texts: &[String]) -> Vec<String> {
    let mut list: Vec<String> = Vec::new();
    for m in texts {
        for word in m.split(|c: char| !c.is_alphanumeric()) {
            let word = word.to_lowercase();
            if word.chars().count() >= 4
                && !STOPWORDS.contains(&word.as_str())
                && !list.contains(&word)
            {
                list.push(word);
            }
        }
    }
    list.truncate(40);
    list
}

/// Input: `text: &str`; `keywords: &[String]`. Output: `usize` — how many `keywords` appear
/// (case-insensitively) in `text`. Used by: `retrieve` below, twice (topic files and raw
/// history lines).
fn score_matches(text: &str, keywords: &[String]) -> usize {
    let lower = text.to_lowercase();
    keywords
        .iter()
        .filter(|a| lower.contains(a.as_str()))
        .count()
}

// the result is ALWAYS at most `limit` characters (some Discord fields — like a select
// menu option's description — enforce this exact limit strictly); "…" is included in the
// count too, otherwise the trimmed result comes out to limit+1 characters (see
// docs/kararlar.md — this caused a live "Must be 100 or fewer in length" error)
/// Input: `text: &str`; `limit: usize`. Output: `String` — `text` trimmed, or hard-cut to
/// `limit - 1` characters plus a trailing `…` (so the total is always `<= limit`). Used
/// throughout the crate wherever text has to fit a Discord field/embed limit (`modal.rs`,
/// `retrieve` below, `Bot::firecrawl_search`/`wander` (`agenda.rs`), `Bot::research`
/// (`chat_lookup.rs`)).
pub fn trim(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        text.trim().to_string()
    } else {
        let take_n = limit.saturating_sub(1);
        format!("{}…", text.chars().take(take_n).collect::<String>().trim())
    }
}

// what to retrieve for this chat: the speakers' files, topic files touching the subject,
// recent events, and old raw-memory lines matching a keyword. The budget is fixed.
/// Input: `participants: &[String]` — names currently talking (up to 4 used);
/// `name_to_id: &HashMap<String, u64>` — to resolve names to person files;
/// `keywords: &[String]` — from `keywords` above; `history: &VecDeque<String>` — the raw
/// message buffer; `exclude_recent: usize` — how many of the most recent `history` lines
/// to skip (they're already in the chat, no need to retrieve them again).
/// Output: `String` — sections (person files, up to 2 matching topic files, the month's
/// last 8 events, up to 12 old keyword-matching lines) concatenated up to
/// `CONTEXT_BUDGET` characters. Uses: `read`, `trim`, `files`, `score_matches`, `month`.
/// Used by: `Bot::chat_system` (`provider_generate.rs`), the only caller.
pub fn retrieve(
    participants: &[String],
    name_to_id: &std::collections::HashMap<String, u64>,
    keywords: &[String],
    history: &VecDeque<String>,
    exclude_recent: usize,
) -> String {
    let mut sections: Vec<String> = Vec::new();

    for name in participants.iter().take(4) {
        let Some(id) = name_to_id.get(&name.to_lowercase()) else {
            continue;
        };
        let content = read(&format!("kisiler/{id}.md"));
        if !content.is_empty() {
            sections.push(trim(&content, 1200));
        }
    }

    // content travels bundled with its score: the top two files aren't read a second time
    let mut topics: Vec<(usize, String)> = files("konular")
        .into_iter()
        .take(30)
        .map(|p| fs::read_to_string(&p).unwrap_or_default())
        .map(|content| (score_matches(&content, keywords), content))
        .filter(|(score, _)| *score >= 1)
        .collect();
    topics.sort_by_key(|k| std::cmp::Reverse(k.0));
    for (_, content) in topics.into_iter().take(2) {
        sections.push(trim(&content, 800));
    }

    let events = read(&format!("olaylar/{}.md", month()));
    let recent_events: Vec<&str> = events.lines().filter(|l| l.starts_with("- ")).collect();
    if !recent_events.is_empty() {
        let skip = recent_events.len().saturating_sub(8);
        sections.push(format!(
            "Son olaylar:\n{}",
            recent_events[skip..].join("\n")
        ));
    }

    // raw context window: old lines that aren't in the current chat but touch the topic
    if !keywords.is_empty() {
        let available = history.len().saturating_sub(exclude_recent);
        let mut matches: Vec<(usize, usize, &String)> = history
            .iter()
            .take(available)
            .enumerate()
            .map(|(i, line)| (score_matches(line, keywords), i, line))
            .filter(|(score, _, _)| *score >= 2)
            .collect();
        matches.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
        matches.truncate(12);
        matches.sort_by_key(|e| e.1);
        if !matches.is_empty() {
            let lines: Vec<String> = matches.iter().map(|(_, _, line)| trim(line, 200)).collect();
            sections.push(format!(
                "Hafızadan, konuya değen eski mesajlar:\n{}",
                lines.join("\n")
            ));
        }
    }

    // budget: add sections in order, stop once full; size tracked with a counter so the
    // result isn't rescanned on every section (chars().count() would make this O(n²))
    let mut result = String::new();
    let mut size = 0usize;
    for section in sections {
        let section_len = section.chars().count();
        if size + section_len > CONTEXT_BUDGET {
            break;
        }
        if !result.is_empty() {
            result.push_str("\n\n");
            size += 2;
        }
        result.push_str(&section);
        size += section_len;
    }
    result
}

// files over their limit: (kind, file path)
/// Input: none. Output: `Vec<(&'static str, PathBuf)>` — `("kisi", path)`/`("konu", path)`/
/// `("olay", path)` for every file over `PERSON_LIMIT`/`TOPIC_LIMIT`/`EVENT_LIMIT`. Uses:
/// `files`, `path`, `month`. Used by: `Bot::summarizer` (`agents.rs`), the only caller.
pub fn over_limit() -> Vec<(&'static str, PathBuf)> {
    let mut list = Vec::new();
    for path in files("kisiler") {
        if fs::metadata(&path)
            .map(|m| m.len() as usize > PERSON_LIMIT)
            .unwrap_or(false)
        {
            list.push(("kisi", path));
        }
    }
    for path in files("konular") {
        if fs::metadata(&path)
            .map(|m| m.len() as usize > TOPIC_LIMIT)
            .unwrap_or(false)
        {
            list.push(("konu", path));
        }
    }
    let event_path = path(&format!("olaylar/{}.md", month()));
    if fs::metadata(&event_path)
        .map(|m| m.len() as usize > EVENT_LIMIT)
        .unwrap_or(false)
    {
        list.push(("olay", event_path));
    }
    list
}

#[cfg(test)]
mod test {
    use super::*;

    /// Verifies `date_from_unix` against known unix timestamps, including a day boundary.
    #[test]
    fn date_is_correct() {
        assert_eq!(date_from_unix(0), "1970-01-01");
        assert_eq!(date_from_unix(1788220800), "2026-09-01");
        assert_eq!(date_from_unix(1788220799), "2026-08-31");
    }

    /// Verifies `trim` caps at the char limit with an ellipsis, and leaves short text untouched.
    #[test]
    fn trim_stays_within_limit() {
        // on a hard Discord limit like a select-menu description, going over `limit`
        // including the "…" got the request rejected (live "Must be 100 or fewer" error)
        let long_text = "a".repeat(200);
        let result = trim(&long_text, 100);
        assert_eq!(result.chars().count(), 100);
        assert!(result.ends_with('…'));
        // text that already fits is left untouched
        assert_eq!(trim("kısa", 100), "kısa");
        assert_eq!(trim(&"a".repeat(100), 100), "a".repeat(100));
    }

    /// Verifies `slug` transliterates Turkish characters and falls back to "bilinmeyen" when nothing survives.
    #[test]
    fn slug_handles_turkish() {
        assert_eq!(slug("Emin Şeyrek"), "emin-seyrek");
        assert_eq!(slug("LNG deniz altı"), "lng-deniz-alti");
        assert_eq!(slug("!!!"), "bilinmeyen");
    }

    /// Verifies `Person::text` / `Person::parse` are inverses (a person survives a write-then-read cycle).
    #[test]
    fn person_round_trips() {
        let p = Person {
            id: 259669117248864257,
            name: "Emin".into(),
            username: "kaju".into(),
            previous_names: vec!["eski ad".into()],
            score: -3,
            tags: vec!["rust".into(), "oyun".into()],
            note: "laf soktu".into(),
            facts: vec!["yks'ye hazırlanıyor".into()],
            events: vec!["2026-09-01 14:03:22: tartıştık".into()],
        };
        let parsed = Person::parse(p.id, &p.text());
        assert_eq!(parsed.id, p.id);
        assert_eq!(parsed.name, "Emin");
        assert_eq!(parsed.username, "kaju");
        assert_eq!(parsed.previous_names, p.previous_names);
        assert_eq!(parsed.score, -3);
        assert_eq!(parsed.tags, p.tags);
        assert_eq!(parsed.note, p.note);
        assert_eq!(parsed.facts, p.facts);
        assert_eq!(parsed.events, p.events);
    }

    /// Verifies `keywords` drops stopwords/short tokens and keeps the meaningful ones.
    #[test]
    fn keywords_filtered() {
        let result = keywords(&["rust ile bot yazdım abi, bence güzel oldu".to_string()]);
        assert_eq!(result, vec!["rust", "yazdım", "güzel"]);
    }

    /// Verifies `date_time`'s output shape (`YYYY-MM-DD HH:MM:SS`, 19 characters).
    #[test]
    fn date_time_format() {
        // YYYY-MM-DD HH:MM:SS (19 characters, seconds precision)
        let s = date_time();
        assert_eq!(s.chars().count(), 19);
        assert_eq!(&s[4..5], "-");
        assert_eq!(&s[10..11], " ");
        assert_eq!(&s[13..14], ":");
        assert_eq!(&s[16..17], ":");
    }

    /// Verifies `person_header` reads only the header fields and stops before the fact/event lists,
    /// and that an empty file parses to an empty header rather than panicking.
    #[test]
    fn person_header_stops_before_lists() {
        let text = "# Ali\nid: 5\npuan: +3\netiket: rust, oyun\nnot: yakın arkadaş\n\n## Bildiklerin\n- bilgi1\n- bilgi2\n\n## Son olaylar\n- olay1\n";
        let header = person_header(text);
        assert_eq!(header.name, "Ali");
        assert_eq!(header.score, 3);
        assert_eq!(header.tags, vec!["rust", "oyun"]);
        assert_eq!(header.note, "yakın arkadaş");
        // empty file: everything stays empty, no panic
        let empty = person_header("");
        assert!(empty.name.is_empty());
    }

    /// Verifies `write`/`append`/`read` round-trip on a real temp file and that no `.tmp` file is left behind.
    #[test]
    fn write_append_round_trip_on_disk() {
        let name = "test-gecici.md";
        write(name, "ilk\n");
        append(name, "ikinci");
        let content = read(name);
        let tmp_left = path(name).with_extension("tmp").exists();
        let _ = fs::remove_file(path(name));
        assert_eq!(content, "ilk\nikinci\n");
        assert!(!tmp_left); // no temp file should remain after the rename
    }

    /// Verifies `retrieve` pulls in history lines matching the keywords/name and excludes unrelated ones.
    #[test]
    fn retrieve_pulls_from_memory() {
        let mut history = VecDeque::new();
        history.push_back("emin: rust derleme süresi çok uzun".to_string());
        history.push_back("lng: bugün hava güzel".to_string());
        history.push_back("emin: son mesaj, sohbette zaten var".to_string());
        let mut name_to_id = std::collections::HashMap::new();
        name_to_id.insert("emin".to_string(), 1u64);
        let result = retrieve(
            &[],
            &name_to_id,
            &["rust".into(), "derleme".into()],
            &history,
            1,
        );
        assert!(result.contains("rust derleme süresi"));
        assert!(!result.contains("hava güzel"));
        assert!(!result.contains("son mesaj"));
    }
}
