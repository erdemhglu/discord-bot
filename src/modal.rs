// Command interface: slash commands open as embed cards (sectioned and readable, like a
// web page), details are spread across labeled modal fields — nothing gets dumped into a
// single text box.

use super::*;

const MODAL_LIMIT: usize = 4000; // discord TextInput value upper bound
const FIELD_LIMIT: usize = 1024; // discord embed field value upper bound
const LABEL_LIMIT: usize = 45; // modal title/label, select-menu option label
const DESCRIPTION_LIMIT: usize = 100; // select-menu option description
const PERSON_MENU_LIMIT: usize = 25; // discord select-menu option upper bound
const TRIM_NOTE: &str = "\n… (sığmadı, kırpıldı)";

// component ids
pub const MIND_PERSON_PICK: &str = "mind_person_pick";
pub const MIND_TOPICS: &str = "mind_topics";
pub const MIND_EVENTS: &str = "mind_events";
pub const MIND_SUMMARY: &str = "mind_summary";
// settings panel: the thinking-mode buttons' id is "setting_thinking:<mode file value>"
pub const SETTING_THINKING: &str = "setting_thinking:";
pub const SETTING_DEBUG: &str = "setting_debug";
pub const SETTING_WAKE: &str = "setting_wake";
pub const SETTING_SLEEP: &str = "setting_sleep";

// embed accent colors
const COLOR_MIND: u32 = 0x5865F2;
const COLOR_STATUS: u32 = 0x57F287;
const COLOR_HELP: u32 = 0xEB459E;
const COLOR_SETTINGS: u32 = 0xFEE75C;
const COLOR_INFO: u32 = 0x99AAB5;

/// One labeled field of a detail modal. Holds `label` (field title), `custom_id` (unique
/// within the modal, unused after submission — see `handler_event.rs`'s `interaction_create`
/// `Modal` arm), `content` (fitted to `MODAL_LIMIT` by `new` below). Built by `person_modal`/
/// `topics_modal`/`events_modal`/`summary_modal` below; consumed by `build_modal` below.
pub struct Section {
    pub label: String,
    pub custom_id: String,
    pub content: String,
}

impl Section {
    /// Input: `label`/`custom_id: impl Into<String>`; `content: String`. Output: `Self`,
    /// with `content` fitted to `MODAL_LIMIT`. Uses: `fit_to_limit`. Used by every modal
    /// builder below.
    fn new(label: impl Into<String>, custom_id: impl Into<String>, content: String) -> Self {
        Self {
            label: label.into(),
            custom_id: custom_id.into(),
            content: fit_to_limit(&content, MODAL_LIMIT),
        }
    }
}

// text over the limit is cut at the last line break/space + a note; the body always fits within the limit
/// Input: `text: &str`; `limit: usize`. Output: `String` — `text` unchanged if it fits,
/// else cut at the last line break/space within `limit` and a trailing `TRIM_NOTE`
/// appended (the total is always `<= limit`). Uses: `TRIM_NOTE`. Used by: `Section::new`
/// above, `info_embed`/`dash_if_empty`/`status_message` below.
fn fit_to_limit(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let mut s: String = text
        .chars()
        .take(limit - TRIM_NOTE.chars().count())
        .collect();
    if let Some(end) = s.rfind(['\n', ' ']) {
        s.truncate(end);
    }
    s.push_str(TRIM_NOTE);
    s
}

// "2026-09" -> "Eylül 2026"; returns the input unchanged if it can't be parsed
/// Input: `month: &str` — `"YYYY-MM"`. Output: `String` — a Turkish `"<Month> YYYY"` label,
/// or `month` unchanged if it doesn't parse. Used by: `events_modal` below.
fn month_name(month: &str) -> String {
    const MONTHS: [&str; 12] = [
        "Ocak", "Şubat", "Mart", "Nisan", "Mayıs", "Haziran", "Temmuz", "Ağustos", "Eylül", "Ekim",
        "Kasım", "Aralık",
    ];
    let mut parts = month.splitn(2, '-');
    match (parts.next(), parts.next()) {
        (Some(year), Some(m)) if (1..=12).contains(&m.parse::<usize>().unwrap_or(0)) => {
            format!("{} {}", MONTHS[m.parse::<usize>().unwrap_or(1) - 1], year)
        }
        _ => month.to_string(),
    }
}

// categories sorted by total tokens: "sohbet: 120 giriş + 80 çıkış · ..."
/// Input: `metrics: &Metrics`. Output: `String` — `metrics.categories` sorted by total
/// tokens descending, joined `" · "`. Used by: `summary_modal`/`status_message` below.
fn token_breakdown(metrics: &Metrics) -> String {
    use std::fmt::Write as _;
    let mut sorted: Vec<(&'static str, &Usage)> = metrics
        .categories
        .iter()
        .map(|(tag, k)| (*tag, k))
        .collect();
    sorted.sort_by_key(|(_, k)| std::cmp::Reverse(k.prompt_tokens + k.completion_tokens));
    let mut lines = String::new();
    for (i, (tag, k)) in sorted.iter().enumerate() {
        if i > 0 {
            lines.push_str(" · ");
        }
        let _ = write!(
            lines,
            "{tag}: {} giriş + {} çıkış",
            k.prompt_tokens, k.completion_tokens
        );
    }
    lines
}

// ---------- /zihin: embed card + menu/buttons ----------

// the mind card: three columns (people / topics / events) + counters in the footer
/// Input: `state: &State`. Output: `Vec<CreateEmbed>` — a single-element vec (one embed).
/// Uses: `memory::person_summaries`/`topic_summaries`/`event_months`/`trim`, `growth::stage`/
/// `days`, `dash_if_empty`. Used by: `mind_message` below.
pub fn mind_embeds(state: &State) -> Vec<CreateEmbed> {
    let people = memory::person_summaries();
    let topics = memory::topic_summaries();
    let events = memory::event_months(3);
    let event_count: usize = events.iter().map(|(_, s)| s.len()).sum();

    let mut people_lines = String::new();
    for p in people.iter().take(8) {
        people_lines += &format!("**{}** ({:+})", p.name, p.score);
        if !p.tags.is_empty() {
            people_lines += &format!(" · {}", p.tags.join(", "));
        }
        people_lines.push('\n');
    }

    let mut topic_lines = String::new();
    for (name, latest) in topics.iter().take(8) {
        topic_lines += &format!("**{name}**");
        if !latest.is_empty() {
            topic_lines += &format!(" · son: {latest}");
        }
        topic_lines.push('\n');
    }

    // most recent events: the end of each month working backward from the newest; older
    // months are put first so the display reads chronologically
    let mut chunks: Vec<Vec<&str>> = Vec::new();
    let mut total = 0usize;
    for (_, s) in events.iter() {
        if total >= 5 {
            break;
        }
        let take_n = (5 - total).min(s.len());
        total += take_n;
        let len = s.len();
        chunks.push(
            s.iter()
                .skip(len.saturating_sub(take_n))
                .map(|x| x.as_str())
                .collect(),
        );
    }
    chunks.reverse();
    let mut event_lines = String::new();
    for chunk in chunks {
        for item in chunk {
            // "- date time #channel: text" -> a readable single line
            event_lines += &memory::trim(item.trim_start_matches("- "), 90);
            event_lines.push('\n');
        }
    }

    let growth = &state.growth;
    vec![CreateEmbed::new()
        .title("Zihin")
        .color(COLOR_MIND)
        .description(format!(
            "{} · {}. gün · {} · {}",
            growth::stage(growth).name,
            growth::days(growth) + 1,
            state.model,
            state.thinking_mode.label(),
        ))
        .field(
            format!("Kişiler ({})", people.len()),
            dash_if_empty(&people_lines),
            true,
        )
        .field(
            format!("Konular ({})", topics.len()),
            dash_if_empty(&topic_lines),
            true,
        )
        .field(
            format!("Olaylar ({event_count})"),
            dash_if_empty(&event_lines),
            true,
        )
        .footer(CreateEmbedFooter::new(format!(
            "{} · {}",
            state.bot_name,
            memory::date()
        )))]
}

// the short acknowledgment/status reply for commands: this always goes out instead of
// plain text (e.g. "haber bulamadım", "tamam, <model>", "debug açık"); wrapped by
// reply_info/report_result in command.rs
/// Input: `title`/`description: &str`. Output: `CreateEmbed`. Uses: `fit_to_limit`. Used
/// by: `command/registration_helpers.rs`'s `reply_info`/`report_result`, `cycle_news.rs`'s
/// `run_prank`, `handler_event.rs`'s `guild_create`.
pub fn info_embed(title: &str, description: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title(title)
        .description(fit_to_limit(description, FIELD_LIMIT))
        .color(COLOR_INFO)
}

// an empty section shows as "—" on the card: faint, but the layout doesn't break
/// Input: `s: &str`. Output: `String` — `"—"` if `s` is blank, else `fit_to_limit(s,
/// FIELD_LIMIT)`. Used by: `mind_embeds` above.
fn dash_if_empty(s: &str) -> String {
    let t = s.trim();
    if t.is_empty() {
        "—".to_string()
    } else {
        fit_to_limit(t, FIELD_LIMIT)
    }
}

// person detail menu: most recently changed, label name + score, description holds tags/note
/// Input: none. Output: `Vec<CreateSelectMenuOption>` — up to `PERSON_MENU_LIMIT` options.
/// Uses: `memory::person_summaries`/`trim`. Used by: `mind_components` below.
fn person_options() -> Vec<CreateSelectMenuOption> {
    memory::person_summaries()
        .into_iter()
        .take(PERSON_MENU_LIMIT)
        .map(|p| {
            let mut description: Vec<String> = Vec::new();
            if !p.tags.is_empty() {
                description.push(p.tags.join(", "));
            }
            if !p.note.is_empty() {
                description.push(p.note.clone());
            }
            let option = CreateSelectMenuOption::new(
                memory::trim(&format!("{} ({:+})", p.name, p.score), LABEL_LIMIT),
                p.id.to_string(),
            );
            // discord's description field can't be empty once given (min length 1); if
            // there are no tags/note, the field is left off entirely — otherwise it dropped
            // the whole menu with "Invalid Form Body"
            let description = description.join(" · ");
            if description.trim().is_empty() {
                option
            } else {
                option.description(memory::trim(&description, DESCRIPTION_LIMIT))
            }
        })
        .collect()
}

/// Input: none. Output: `Vec<CreateActionRow>` — the person select-menu row (if any people
/// exist) plus the Topics/Events/Bot-summary button row. Uses: `person_options`,
/// `MIND_PERSON_PICK`/`MIND_TOPICS`/`MIND_EVENTS`/`MIND_SUMMARY`. Used by: `mind_message`
/// below.
pub fn mind_components() -> Vec<CreateActionRow> {
    let mut rows: Vec<CreateActionRow> = Vec::new();
    let options = person_options();
    if !options.is_empty() {
        rows.push(CreateActionRow::SelectMenu(
            CreateSelectMenu::new(MIND_PERSON_PICK, CreateSelectMenuKind::String { options })
                .placeholder("Kişi detayı seç…"),
        ));
    }
    rows.push(CreateActionRow::Buttons(vec![
        CreateButton::new(MIND_TOPICS)
            .label("Konular")
            .style(ButtonStyle::Secondary),
        CreateButton::new(MIND_EVENTS)
            .label("Olaylar")
            .style(ButtonStyle::Secondary),
        CreateButton::new(MIND_SUMMARY)
            .label("Bot özeti")
            .style(ButtonStyle::Secondary),
    ]));
    rows
}

/// Input: `state: &State`. Output: `CreateInteractionResponseMessage` — the ephemeral
/// `/zihin` reply. Uses: `mind_embeds`, `mind_components`. Used by: `Bot::cmd_mind`
/// (`command/cards.rs`), the only caller.
pub fn mind_message(state: &State) -> CreateInteractionResponseMessage {
    CreateInteractionResponseMessage::new()
        .ephemeral(true)
        .embeds(mind_embeds(state))
        .components(mind_components())
}

// ---------- detail modals: each topic in its own labeled field ----------

/// Input: `title`/`custom_id: &str`; `sections: Vec<Section>` (consumed). Output:
/// `CreateModal` — one `InputText` field per non-empty section, or a single "(henüz boş)"
/// placeholder field if all were empty. Uses: `memory::trim`. Used by:
/// `person_modal`/`topics_modal`/`events_modal`/`summary_modal` below.
fn build_modal(title: &str, custom_id: &str, sections: Vec<Section>) -> CreateModal {
    let filled: Vec<Section> = sections
        .into_iter()
        .filter(|b| !b.content.trim().is_empty())
        .collect();
    let rows = if filled.is_empty() {
        vec![CreateActionRow::InputText(
            CreateInputText::new(
                InputTextStyle::Paragraph,
                "Durum",
                format!("{custom_id}_bos"),
            )
            .value("(henüz boş)")
            .required(false),
        )]
    } else {
        filled
            .into_iter()
            .map(|b| {
                CreateActionRow::InputText(
                    CreateInputText::new(
                        InputTextStyle::Paragraph,
                        memory::trim(&b.label, LABEL_LIMIT),
                        b.custom_id,
                    )
                    .value(b.content)
                    .required(false),
                )
            })
            .collect()
    };
    CreateModal::new(
        custom_id,
        title.chars().take(LABEL_LIMIT).collect::<String>(),
    )
    .components(rows)
}

// person card: identity / impression / tags / facts / recent events in separate fields
/// Input: `id: u64` — a Discord user id. Output: `CreateModal`. Uses:
/// `memory::read_person`, `Section::new`, `build_modal`. Used by:
/// `Handler::interaction_create` (`handler_event.rs`), for the `MIND_PERSON_PICK` select.
pub fn person_modal(id: u64) -> CreateModal {
    let p = memory::read_person(id);
    let title = if p.name.is_empty() {
        "bilinmeyen".to_string()
    } else {
        p.name.clone()
    };
    let mut sections: Vec<Section> = Vec::new();

    let mut identity = format!("{}\nid: {}", p.name, p.id);
    if !p.username.is_empty() {
        identity += &format!("\nkullanıcı adı: {}", p.username);
    }
    if !p.previous_names.is_empty() {
        identity += &format!("\nönceki adları: {}", p.previous_names.join(", "));
    }
    sections.push(Section::new("Kimlik", "person_identity", identity));

    let mut impression = format!("puan: {:+}", p.score);
    if !p.note.is_empty() {
        impression += &format!("\n{}", p.note);
    }
    sections.push(Section::new("İzlenim", "person_impression", impression));

    if !p.tags.is_empty() {
        sections.push(Section::new("Etiketler", "person_tags", p.tags.join(" · ")));
    }
    if !p.facts.is_empty() {
        let n = p.facts.len();
        let list: Vec<&str> = p
            .facts
            .iter()
            .skip(n.saturating_sub(8))
            .map(|s| s.as_str())
            .collect();
        sections.push(Section::new("Bildikleri", "person_facts", list.join("\n")));
    }
    if !p.events.is_empty() {
        let n = p.events.len();
        let list: Vec<&str> = p
            .events
            .iter()
            .skip(n.saturating_sub(5))
            .map(|s| s.as_str())
            .collect();
        sections.push(Section::new(
            "Son olaylar",
            "person_events",
            list.join("\n"),
        ));
    }
    build_modal(&title, &format!("mind_person_{id}"), sections)
}

// topics: most recently changed with their notes, the rest as a plain name list
/// Input: none. Output: `CreateModal`. Uses: `memory::topic_summaries`, `Section::new`,
/// `build_modal`. Used by: `Handler::interaction_create` (`handler_event.rs`), for the
/// `MIND_TOPICS` button.
pub fn topics_modal() -> CreateModal {
    let topics = memory::topic_summaries();
    let mut sections: Vec<Section> = Vec::new();
    let recent: Vec<String> = topics
        .iter()
        .take(15)
        .map(|(name, note)| {
            if note.is_empty() {
                format!("- {name}")
            } else {
                format!("- {name} · son: {note}")
            }
        })
        .collect();
    sections.push(Section::new(
        "Son değişenler",
        "topics_recent",
        recent.join("\n"),
    ));
    if topics.len() > 15 {
        let other: Vec<&str> = topics[15..].iter().map(|(name, _)| name.as_str()).collect();
        sections.push(Section::new(
            "Diğer konular",
            "topics_other",
            other.join(" · "),
        ));
    }
    build_modal("Konular", "topics_modal", sections)
}

// events: one field per month, each month's most recent entries
/// Input: none. Output: `CreateModal`. Uses: `memory::event_months`, `month_name`,
/// `Section::new`, `build_modal`. Used by: `Handler::interaction_create`
/// (`handler_event.rs`), for the `MIND_EVENTS` button.
pub fn events_modal() -> CreateModal {
    let mut sections: Vec<Section> = Vec::new();
    for (month, lines) in memory::event_months(3) {
        if lines.is_empty() {
            continue;
        }
        let n = lines.len();
        let shown: Vec<&str> = lines
            .iter()
            .skip(n.saturating_sub(10))
            .map(|s| s.as_str())
            .collect();
        sections.push(Section::new(
            month_name(&month),
            format!("events_{month}"),
            shown.join("\n"),
        ));
    }
    build_modal("Olaylar", "events_modal", sections)
}

// bot summary: status / tokens / myself / agenda in separate fields
/// Input: `state: &State`. Output: `CreateModal`. Uses: `growth::stage`/`days`,
/// `sleep::is_awake`, `travel::now`, `token_breakdown`, `memory::trim`, `Section::new`,
/// `build_modal`. Used by: `Handler::interaction_create` (`handler_event.rs`), for the
/// `MIND_SUMMARY` button.
pub fn summary_modal(state: &State) -> CreateModal {
    let growth = &state.growth;
    let metrics = &state.metrics;
    let status = format!(
        "evre: {} ({}. gün)\nsohbet: {} · mesaj: {}\nmodel: {}\nuyku: {} · düşünme: {}\nseyahat: {}",
        growth::stage(growth).name,
        growth::days(growth) + 1,
        growth.chats,
        growth.messages,
        state.model,
        if sleep::is_awake(state) { "uyanık" } else { "uyuyor" },
        state.thinking_mode.label(),
        travel::now().map(|s| s.place).unwrap_or("yok"),
    );
    let mut token_text = format!(
        "{} çağrı · {} giriş ({} önbellek) + {} çıkış",
        metrics.calls, metrics.input_tokens, metrics.cache_tokens, metrics.output_tokens
    );
    if !metrics.categories.is_empty() {
        token_text += &format!("\nkırılım: {}", token_breakdown(metrics));
    }
    let mut sections = vec![
        Section::new("Durum", "summary_status", status),
        Section::new("Token", "summary_tokens", token_text),
    ];
    if !state.myself.trim().is_empty() {
        let recent: Vec<&str> = state.myself.lines().rev().take(4).collect();
        sections.push(Section::new(
            "Kendim",
            "summary_myself",
            recent.into_iter().rev().collect::<Vec<_>>().join("\n"),
        ));
    }
    if !state.agenda.trim().is_empty() {
        sections.push(Section::new(
            "Gündem",
            "summary_agenda",
            memory::trim(&state.agenda, 1000),
        ));
    }
    build_modal("Bot özeti", "summary_modal", sections)
}

// ---------- /durum and /yardim ----------

/// Input: `state: &State`. Output: `CreateInteractionResponseMessage` — the ephemeral
/// `/durum` reply. Uses: `growth::stage`/`days`, `sleep::is_awake`, `travel::now`,
/// `version_text`, `token_breakdown`, `fit_to_limit`. Used by: `Bot::cmd_status`
/// (`command/cards.rs`), the only caller.
pub fn status_message(state: &State) -> CreateInteractionResponseMessage {
    let metrics = &state.metrics;
    let growth = &state.growth;
    let mut embed = CreateEmbed::new()
        .title("Durum")
        .color(COLOR_STATUS)
        .field(
            "Genel",
            format!(
                "sürüm: {}\nevre: {} ({}. gün)\nsohbet: {} · mesaj: {}\nmodel: {}",
                version_text(),
                growth::stage(growth).name,
                growth::days(growth) + 1,
                growth.chats,
                growth.messages,
                state.model,
            ),
            true,
        )
        .field(
            "Hal",
            format!(
                "uyku: {}\ndüşünme: {}\ndebug: {}\nseyahat: {}",
                if sleep::is_awake(state) {
                    "uyanık"
                } else {
                    "uyuyor"
                },
                state.thinking_mode.label(),
                if state.debug { "açık" } else { "kapalı" },
                travel::now().map(|s| s.place).unwrap_or("yok"),
            ),
            true,
        )
        .field(
            "Token",
            format!(
                "{} çağrı · {} giriş ({} önbellek) + {} çıkış",
                metrics.calls, metrics.input_tokens, metrics.cache_tokens, metrics.output_tokens
            ),
            false,
        );
    if !metrics.categories.is_empty() {
        embed = embed.field(
            "Kırılım",
            fit_to_limit(&token_breakdown(metrics), FIELD_LIMIT),
            false,
        );
    }
    CreateInteractionResponseMessage::new()
        .ephemeral(true)
        .embed(embed)
}

/// Input: none. Output: `CreateInteractionResponseMessage` — the ephemeral `/yardim` reply.
/// Uses: `command::HELP` (`command/registration_table.rs`). Used by: `Bot::cmd_help`
/// (`command/cards.rs`), the only caller.
pub fn help_message() -> CreateInteractionResponseMessage {
    CreateInteractionResponseMessage::new()
        .ephemeral(true)
        .embed(
            CreateEmbed::new()
                .title("Yardım")
                .color(COLOR_HELP)
                .description(command::HELP)
                .field(
                    "Arayüz",
                    "bot yalnız slash (/) komutlarla yönetilir; /zihin'deki menü ve butonlar detay modallarına götürür.",
                    false,
                ),
        )
}

// ---------- settings panel (buttons) ----------

/// Input: `state: &State`. Output: `CreateEmbed` — the settings panel's description/footer.
/// Uses: `sleep::is_awake`, `version_text`, `travel::now`. Used by: `settings_message` below.
pub fn settings_embed(state: &State) -> CreateEmbed {
    let sleep_status = if sleep::is_awake(state) {
        if state.forced_awake_until > now_unix() {
            "uyanık (zorla, !uyan)"
        } else {
            "uyanık"
        }
    } else {
        "uyuyor"
    };
    CreateEmbed::new()
        .title("Ayarlar")
        .color(COLOR_SETTINGS)
        .description(format!(
            "sürüm: {}\nmodel: {} (`!model <id>`, yalnız favori)\ndüşünme: **{}**\ndebug: **{}**\nuyku: **{}**\nseyahat: {}",
            version_text(),
            state.model,
            state.thinking_mode.label(),
            if state.debug { "açık" } else { "kapalı" },
            sleep_status,
            travel::now().map(|s| s.place).unwrap_or("yok"),
        ))
        .footer(CreateEmbedFooter::new(
            "butona bas, panel yerinde yenilenir · göster: düşünce spoiler'da · gizle: düşünüyorum… · sessiz: iz yok · kapat: reasoning'siz",
        ))
}

/// Input: `state: &State`. Output: `Vec<CreateActionRow>` — the thinking-mode button row
/// (highlighting the active mode) plus the debug/wake/sleep button row. Uses:
/// `ThinkingMode::file_value`, `SETTING_THINKING`/`SETTING_DEBUG`/`SETTING_WAKE`/
/// `SETTING_SLEEP`, `sleep::is_awake`. Used by: `settings_message` below.
pub fn settings_components(state: &State) -> Vec<CreateActionRow> {
    let modes = [
        (ThinkingMode::Show, "göster"),
        (ThinkingMode::Hide, "gizle"),
        (ThinkingMode::Silent, "sessiz"),
        (ThinkingMode::Off, "kapat"),
    ];
    let thinking_buttons: Vec<CreateButton> = modes
        .iter()
        .map(|(mode, label)| {
            CreateButton::new(format!("{SETTING_THINKING}{}", mode.file_value()))
                .label(format!("düşünme: {label}"))
                .style(if *mode == state.thinking_mode {
                    ButtonStyle::Primary
                } else {
                    ButtonStyle::Secondary
                })
        })
        .collect();
    let awake = sleep::is_awake(state);
    vec![
        CreateActionRow::Buttons(thinking_buttons),
        CreateActionRow::Buttons(vec![
            CreateButton::new(SETTING_DEBUG)
                .label(if state.debug {
                    "debug: açık"
                } else {
                    "debug: kapalı"
                })
                .style(if state.debug {
                    ButtonStyle::Success
                } else {
                    ButtonStyle::Secondary
                }),
            CreateButton::new(SETTING_WAKE)
                .label("uyandır")
                .style(ButtonStyle::Secondary)
                .disabled(awake),
            CreateButton::new(SETTING_SLEEP)
                .label("uyut (8 saat)")
                .style(ButtonStyle::Secondary)
                .disabled(!awake),
        ]),
    ]
}

// /ayarlar (ephemeral) and the in-place refresh after a button press (UpdateMessage) share this body
/// Input: `state: &State`; `ephemeral: bool`. Output: `CreateInteractionResponseMessage`.
/// Uses: `settings_embed`, `settings_components`. Used by: `Bot::cmd_settings`
/// (`command/cards.rs`, `/ayarlar`), `Handler::setting_button` (`handler_buttons.rs`,
/// panel refresh).
pub fn settings_message(state: &State, ephemeral: bool) -> CreateInteractionResponseMessage {
    CreateInteractionResponseMessage::new()
        .ephemeral(ephemeral)
        .embed(settings_embed(state))
        .components(settings_components(state))
}

// ---------- slash registration ----------

// guild commands: called on every ready, overwrites what's on Discord (idempotent); a
// guild command shows up instantly, a global command is delayed. The list comes from
// command.rs's definitions() table — a single source, never kept in two places by hand.
/// Input: `http: &Http`; `guild: GuildId`. Output: `Result<(), BotError>`. Uses:
/// `command::definitions`. Used by: `Handler::ready` (`handler_event.rs`), once per guild
/// on every connect/reconnect.
pub async fn register_commands(http: &Http, guild: GuildId) -> Result<(), BotError> {
    let commands: Vec<CreateCommand> = command::definitions()
        .iter()
        .map(|k| {
            CreateCommand::new(k.name)
                .description(k.description)
                .set_options((k.options)())
        })
        .collect();
    guild.set_commands(http, commands).await?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    /// Verifies `fit_to_limit` truncates with a "kırpıldı" marker when over the limit, and passes short text through unchanged.
    #[test]
    fn fit_stays_within_limit() {
        let long_text = "kelime ".repeat(1000);
        let s = fit_to_limit(&long_text, 200);
        assert!(s.chars().count() <= 200);
        assert!(s.contains("kırpıldı"));
        // short text passes through unchanged
        assert_eq!(fit_to_limit("kısa", 200), "kısa");
    }

    /// Verifies `month_name` converts a `YYYY-MM` key to a Turkish month name, and passes through unparseable input as-is.
    #[test]
    fn month_name_converts() {
        assert_eq!(month_name("2026-09"), "Eylül 2026");
        assert_eq!(month_name("2026-01"), "Ocak 2026");
        assert_eq!(month_name("bozuk"), "bozuk");
        assert_eq!(month_name("2026-13"), "2026-13");
    }

    /// Verifies the empty-section filter `build_modal` relies on: only non-empty `Section`s survive.
    #[test]
    fn empty_sections_become_placeholder_field() {
        // build_modal skips empty sections; if all are empty, a single "(henüz boş)" field remains.
        // CreateModal isn't inspectable, so behavior is verified via the Section filter itself:
        let filled: Vec<Section> = vec![
            Section::new("A", "a", String::new()),
            Section::new("B", "b", "veri".into()),
        ]
        .into_iter()
        .filter(|b| !b.content.trim().is_empty())
        .collect();
        assert_eq!(filled.len(), 1);
        assert_eq!(filled[0].label, "B");
    }

    /// Verifies `token_breakdown` sorts categories by usage, descending.
    #[test]
    fn token_breakdown_sorted() {
        let mut metrics = Metrics::default();
        metrics.categories.insert(
            "few",
            Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                ..Default::default()
            },
        );
        metrics.categories.insert(
            "many",
            Usage {
                prompt_tokens: 100,
                completion_tokens: 50,
                ..Default::default()
            },
        );
        let result = token_breakdown(&metrics);
        // the larger category comes first
        assert!(result.find("many").unwrap() < result.find("few").unwrap());
    }
}
