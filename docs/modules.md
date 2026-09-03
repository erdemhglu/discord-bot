# Modules and functions

Each line: signature · what it does · who calls it · lock/await note. Line numbers are
approximate, find with `grep -n "fn name"`.

## src/main.rs (+ src/bot/<group>/*.rs, in the same module via `include!`)

### Types
- `ChatMessage { role: &'static str, content: String, image: Option<String> }` — message going to OpenRouter (named `ChatMessage` on purpose instead of `Mesaj`, so it doesn't get confused with serenity's own `Message` type). Constructors `user(..)`, `user_with_image(text, url)`, `assistant(..)`. `image` is `#[serde(skip)]`: since the request body is built by hand it never enters serialization, `message_json` reads it. Stays filled only on the very latest user message (`Handler::message` sets the older ones' to `None` when adding a new line) — a discord cdn link has a short lifetime, and resending an old image every turn burns tokens.
- `message_json(&ChatMessage) -> Value` — turns a message into an openai-compatible block: no image → `{role, content: "…"}`, with one → `content` = `[{type:text,text},{type:image_url,image_url:{url}}]` (same shape as agents.rs's `image_commenter` body). Both `ask_split` and `ask_raw_stream` use this, so images travel on both the streaming and non-streaming path.
- `Reply { lines: Vec<String>, reaction: Option<String>, silent: bool }` — the parsed form of the model's reply (output protocol, see flows.md). `Reply::is_empty()` — is there neither a line, a reaction, nor a silence decision; `Reply::protocol_text()` — the form that goes into history/channel notes (lines joined by `\n` + `tepki: 💀` if present), so the model sees its own format again next turn.
- `Chat { history: Vec<ChatMessage>, counter: u32, hacked: u32, last_message, last_was_tagged: bool, incoming: u32, recent_arrivals, mood: String }` —
  an open chat in one channel. `counter` is the number of messages the bot has written; `hacked` is
  the number of replies left in the hack joke; `last_was_tagged` is for the reply-to decision (see
  `reply`); `mood` is `determine_mood`'s most recent result, in "state (intensity)" form, neutral
  when empty.
- `State` — the single shared state (see architecture.md). `State::load()` reads profile/temperament/corrections/myself/agenda/scanned from disk, refreshes the index.
- `Bot { state: Mutex<State>, http: reqwest::Client, key, news_channel, firecrawl, guild_id: Option<GuildId>, allowed_channels: Option<HashSet<ChannelId>> }`.
- `Bot::state() -> MutexGuard<State>` — also unlocks a poisoned lock. **Never hold across an await.**
- `Handler { bot: Arc<Bot>, started: AtomicBool }` — serenity `EventHandler`.
- `BotError = Box<dyn Error + Send + Sync>`.

### Helpers
- `now_unix() -> i64` — current time, unix seconds.
- `display_name(&User) -> String` — display name (`global_name`), falling back to username. Memory and person files use this name.
- `channel_note(&mut State, channel, line)` — appends to the channel's history (60 in memory + `durum/kanallar/<id>.md`); user lines come from `message`, bot lines from `send`. `channel_notes(&mut State, channel, lines)` does the same for several lines with a SINGLE file write (`channel_note` is its one-element case); `send_stream` writes a multi-line reply with this — otherwise the whole history got rewritten from scratch for every single line.
- `remember(&mut State, name, text)` — appends "name: text" to raw memory, drops from the front once it exceeds 2000.
- `recent_messages(&State, n) -> String` — the last n lines of raw memory, joined by `\n`.
- `transcript(&[ChatMessage], bot_name) -> String` — turns a chat into "name: text" lines. Since a bot reply can span multiple lines (protocol text), **every** line gets a `bot_name:` prefix — including the reaction line — otherwise the critic/diarist/coach would count the extra lines as belonging to people in the group.
- `strip_name(&str, bot_name) -> &str` — model output: strips a leading `bot_name:` pattern and outer quotes. Returns a slice, doesn't clone.
- `clean(String, bot_name) -> String` — `strip_name` + cut at 1900 characters. Applied to the ENTIRE reply on `generate`'s output, i.e. the non-streaming path (before the protocol is split into lines): if a 4-line reply's total exceeds 1900, the last line(s) are silently cut. No cutting on the streaming path — there each line is split individually with `split`.
- `parse_reply(text) -> Reply` — **the single place that decodes the output protocol.** Operates on text that has already had `strip_name` applied (doesn't strip again): splits on `\n`, trims, drops blanks; a `silence_marker` line → `silent`; `reaction_body` + `extract_emoji` → `reaction` (first one wins, the line doesn't go out as a message); a stray line starting with `'` is dropped; `clean_slop` is applied; if it's a **real list** (a numeric prefix on ≥2 lines) `number_prefix` also strips `1. `/`2) ` prefixes — a single line like "3. sınıftayım" is an ordinal, untouched; a line that exactly repeats one already seen this turn isn't taken a second time; at most `BURST_LIMIT` (4) lines survive (extras drop with a debug log); every line is flattened with `split(line, MESSAGE_LIMIT)`. **A short line is never filtered out** ("he", "yok", "la" are natural reactions). Callers: `send_stream`, `send_lines`, `stream_view`, `run_prank`, `chat_cli`.
- `reaction_body(line) -> Option<&str>` — does the line start with `tepki:` (case and the "tepki :" spacing are tolerated); returns what comes after the colon. `too_many_questions` also uses this so it doesn't count reaction lines.
- `extract_emoji(text) -> Option<String>` — the first emoji run: starts with `emoji_start` (known emoji blocks: U+2600–27BF, U+2B00–2BFF, U+1F000–1FAFF, and singles like ©/®/™), continues with `emoji_continues` (the same plus VS15/VS16, ZWJ, keycap) for up to 8 chars. The definition is deliberately narrow: "anything non-letter is an emoji" also counted `—`, `…`, `→`, and typographic quotes as emoji, and Discord rejected those requests with a 400. `None` for a custom `:kekw:`-style emoji or when there's no emoji at all.
- `silence_marker(line) -> bool` — is the line by itself `-`, `"-"`, `'-'`, `[sus]`, or `(sus)`.
- `clean_slop(line) -> String` — strips "written by AI" tells: a leading `- `/`* `/`• ` bullet, `**` and `__` markdown marks. Backticks and their contents are both preserved (the line is split on `` ` `` and single-index pieces are left alone) — so `` `__init__` `` doesn't get mangled. The number prefix is not stripped here but in `parse_reply` (`number_prefix`), because "a real list vs. a Turkish ordinal" can only be told apart by looking at the whole reply.
- `number_prefix(line) -> Option<&str>` — what follows a `1. ` / `2) ` prefix. Never applied on its own; `parse_reply` only calls it when the reply has ≥2 numbered lines.
- `too_many_questions(&State, channel) -> bool` — do ≥2 of the channel history's last 4 bot lines (`tepki:` lines don't count) end in `?`. `reply` and `chat_cli` add "Bu sefer soru sorma; düz laf et ya da sus." to the instruction; nothing is cut. Code measures, the model enforces.
- `split(text, limit) -> Vec<String>` — splits text into pieces of at most `limit` characters: first at a sentence boundary, then whitespace, then a hard cut if neither exists; nothing is dropped. Used when a reply exceeds 1900 and for long thinking.
- `cut_point(text, limit) -> usize` — where `split` cuts; a sentence/space boundary before the first quarter of the limit doesn't count.
- `spoiler(text) -> String` — `||...||`; any `|` inside is escaped.
- `stream_view(mode, thought, answer, done) -> Vec<String>` — the on-screen view for a given mode: while thinking is in progress (answer empty, thought present) show mode displays "Düşünüyorum...", hide mode shows `thought_counter` (a live word count), silent/off modes show nothing; once the answer starts, show mode displays `single_line(thought)` as both a spoiler and `code_blocks`, plus the answer lines, while hide/silent/off modes show only the lines. The answer is now split into messages with `parse_reply(...).lines` rather than `split`: a new message opens when the model moves to a new line, the previous message doesn't change. `single_line(text)` collapses the thinking into one flowing line (no newline is emitted per thought).
- `stream_slice(answer, done) -> &str` — the part of the answer that's showable while streaming: finished lines (followed by `\n`) plus the last partial line, but only once it passes `HALF_LINE_THRESHOLD` (12) characters. Rationale: a half-formed "tep" could turn into `tepki: 💀` or `-`, and it shouldn't be opened as a message only to get deleted on the next edit; and there's no point editing for a tiny sliver of text. If `done=true`, the whole text.
- `stream_layout(mode, thought, lines) -> Vec<String>` — thinking blocks (show mode only) + line messages. Lines come from outside: `send_stream` passes in already-re-filtered final-layout versions.
- `thought_counter(thought)` — "Düşünüyorum... Şu ana kadar N kelime düşündüm." `code_blocks(text)` — thinking's code blocks (split at 1900). `thought_display(text)` — the button's ephemeral reply: a code block that fits one message, with a truncation note if it's long.
- `State::link_thought(message, thought)` — in hide mode, links the thought to the latest message id so the button can find it (`thought_store` holds 50 entries, dropped oldest-first via `thought_order`). `Handler::interaction_create` — when `THOUGHT_BUTTON` is clicked, fetches it from storage and sends an ephemeral code block visible only to the clicker.
- `ThinkingMode { Show, Hide, Silent, Off }` (bot/types/types_chat_state.rs) — thinking mode; `from_arg` parses the command argument, `read`/`file_value` read `durum/dusunme.md`, `label` is the display name. In Silent mode reasoning is still requested normally (not turned off), only `send_stream` never collects/shows the thought (no placeholder/counter/button) — it leaves in the background what show/hide would put on screen. Off mode has `Bot::disable_reasoning` add `reasoning.enabled=false` + `enable_thinking=false` to the request (only the streaming/chat path looks at the mode; `ask_raw`'s non-streaming path — background agents — always turns it off regardless of mode, since that path never reads reasoning anyway).
- `reply_budget!()` — macro; the chat reply's token budget by build: release `Some(REPLY_CAP=4096)` (an ordinary reply stays under it, it only cuts off runaway cases like repetition/loops), debug `Some(2000)` (a cost guard).
- `extract_json(&str) -> &str` — everything between the first `{` and last `}` (strips code-block decoration).

### OpenRouter (impl Bot)
- `ask_raw(Value, category) -> Result<String>` — POST `/chat/completions`, `choices[0].message.content`; error if empty. The single HTTP entry point. Timeouts: 15s to connect, 120s between data chunks, no overall cap (long thinking isn't cut off). On a network error / 429 / 5xx (`status_retryable`) backs off 2+4s and retries `AI_RETRIES` times; if a reasoning-mandatory model returns 400 (`reasoning_mandatory_error`) it strips the fields and retries — in that case `apply_budget_floor` raises `max_tokens` (if set) to `REASONING_MANDATORY_BASE` (500), since otherwise small-budget mini-calls (20-80 tokens) let reasoning eat the whole budget and leave `content: null`. For the same reason, a 200 with empty content doesn't error out immediately either (reasoning may have eaten the budget): the budget is raised to the floor and retried once more, and only after `AI_RETRIES` at the floor does it give up. A successful reply's `usage` goes to `add_metric` tagged with `category` (the `!durum` breakdown). `ask_raw_stream` follows the same logic (only before the stream opens; the empty-content-then-retry step exists only in `ask_raw`, the streaming side handles it separately in `send_stream`).
- `disable_reasoning(body, force) -> bool` — if mode is Off or `force=true`, flips the provider-specific switch: openrouter's `reasoning.enabled`, nothing for mistral, others (qwen-style routers) `enable_thinking:false`. `ask_raw` (non-streaming) always passes `true` — that path never reads `reasoning_content` anyway, so it turns it off no matter the user's mode; `ask_raw_stream` passes `false`, turning it off only when the mode is Off. Returns `true` if it actually added the fields (for the mandatory-reasoning retry).
- `BusyGuard` — RAII that releases the channel's busy flag when `reply` returns: Drop runs on normal/early return and on panic; for a new turn, `drop(_busy_guard)` then re-insert above.
- `strip_name(text, bot_name)` — strips the name prefix + quotes; char-safe (no byte-slicing), compares by `casefold` dropping the İ→i̇ combining dot.
- `ask(system, history, max_tokens, category)` — `system` + history → `ask_split` (budget `Some`).
- `generate(history, instruction, budget: Option<u32>, category)` — **the single way to speak with personality.** Builds the system message with `chat_system` → `ask_split` → `clean`. If budget is `None`, no max_tokens is sent (only for a few one-off calls; a chat reply always uses `reply_budget!()`'s `Some`). Callers: the streaming fallback/retry, poke, prank, news teaser, welcome, woke up, wanderer note, image_commenter fallback, name.
- `chat_system(history, instruction) -> (fixed, variable, bot_name)` — pulls participant names (the `"name: "` prefix) and texts out of the `user` messages in history → `memory::keywords` → under lock, `memory::retrieve` + `system_text`. Shared by `generate` and `generate_stream`.
- `ask_raw_stream(fixed, variable, history, budget, category) -> Result<StreamReader>` — a `stream:true` POST; error handling is the same as `ask_raw`. `StreamReader::next()`, returning `Chunk{text,thought}`, decodes SSE lines (`extract_sse`; reasoning from the `reasoning` or `reasoning_content` field), buffering even when a utf-8 chunk is split mid-character.
- `memory_cycle(bot)` — every 10 minutes, unaffected by the sleep check: if asleep, queues a night observation every 2 hours; then processes `memory_queue` (`diarist` for each finished chat + `critic`).
- `generate_stream(history, instruction, budget, category) -> Result<(StreamReader, bot_name)>` — opens the chat reply as a stream. Caller: `reply` only.
- `send_stream(ctx, channel, reader, StreamContext) -> Result<StreamResult>` — accumulates chunks (reasoning isn't accumulated in Off mode), edits every `STREAM_EDIT_INTERVAL` with `stream_view(..., done=false)` + `write_stream`; once done, `parse_reply(strip_name(...))`:
  **silent** (no line AND no reaction) → the temporary messages opened so far are removed with `delete_messages`, `StreamResult::Silent` (if `-` and `tepki: 💀` arrive together it's not silence: the emoji still lands)
  **empty** (no line, no reaction, no silence) → same cleanup, `StreamResult::Empty`;
  **repeats**, now checked line-by-line: lines identical to one of the last 5 bot lines are dropped; if no line is left and there's no reaction either, regenerate once with `generate`, and if that repeats too, delete + Empty;
  final `stream_layout` + `write_stream`; if there's a reaction, `ctx.http.create_reaction(..., ReactionType::Unicode(emoji))` on `context.reaction_target`'s message (an error is only a warn log, the stream doesn't stop — a reaction alone is still a valid reply); logging: every visible line goes into `own_messages` + `channel_note` one at a time, the reaction as a `"{bot}: tepki: 💀"` line (for seed consistency), thinking never enters. The `Sent(String)` it returns holds `Reply::protocol_text()`.
  `StreamResult::{Sent(String), Empty, Silent}`; `StreamContext{bot_name, reply_to, reaction_target, history, instruction, budget}` is a single struct instead of a pile of arguments. `reaction_target` is a separate field from `reply_to` because `reply_to` is conditional (set only when tagged or when several people are talking), whereas a reaction always needs some message to land on. `reaction_target` is **always** the chat's `last_message`; `reply_to` instead shifts to whichever person's message `pick_target` picked — so the two can diverge: the reply can be linked as a discord reply to erdem while the emoji lands on whoever wrote last. This is deliberate: picking a target changes who the reply is addressed to, but a reaction still lands on "the message just before."
- `send_lines(ctx, channel, raw, reply_to, reaction_target, ping) -> Option<String>` — **the shared sender for the non-streaming paths.** Runs `strip_name` + `parse_reply` and hands off to `send_reply`. `send_reply(ctx, channel, Reply, reply_to, reaction_target, ping)` is the body: paths that already hold a resolved/de-duplicated `Reply` (the reply's fallback branch) call this directly without going back through text. Lines go out in order as separate messages (`send`); between them, `LINE_DELAY_BASE + LINE_DELAY_PER_CHAR × character-count` (capped at `LINE_DELAY_CAP`) delay plus `broadcast_typing` — none of the stream's own pacing applies here, so lines don't all land at once. `reply_to` only attaches to the first line; `ping` too, but **only after the protocol is decoded**, prepended as `<@id> ` to the first line's front at send time (pasting it in before decoding made "`<@id> -`" no longer read as a silence marker, and "`<@id> tepki: 💀`" no longer read as a reaction line). Without a `reaction_target`, a reaction is dropped (so a reaction that wouldn't be visible in the channel isn't counted as "sent"). Nothing is sent for `silent`, or for a reply with nothing left to send; `None` is returned. The `protocol_text` it returns becomes the chat's opening text in the caller. Callers: `reply`'s fallback `generate` branch, `post_problem`, `send_news` (teaser), `poke_cycle` (OUT_OF_THE_BLUE/ON_THE_WAY/LEAVING), `guild_member_addition` (welcome, with a ping), `sleep_transition` (WOKE_UP), `evaluate_waking`, `pick_name` (announcement). If it returns `None` that opening is skipped and no chat starts (debug log).
- `write_stream(ctx, channel, &mut Vec<Message>, layout, reply_to)` — a free function. Reconciles the layout with the open messages: edits whichever changed via `EditMessage`, opens whatever's missing (only the first message carries the reply/mention), deletes whatever's extra; typing isn't sent here (it would repeat on every edit and hit Discord's rate limit — `reply` sends it once before the model call instead). `delete_messages(ctx, Vec<Message>)` undoes the opened messages.
- `analyze(text, instruction, max_tokens, category)` — **the single personality-free path.** System = `ANALYST`; user message = `text + "---" + instruction`. Callers: profiler, diarist, coach, critic, summarizer, news_agent selection, wanderer selection, waking evaluation.
- `willingness() -> Option<(u8, String)>` — the mini "should I join this conversation?" evaluation: profile+index in the fixed block (cache_control), last 12 messages variable → `ask_split(..., 80, "isteklilik")` → `parse_willingness` reads 0-10 from JSON. Caller: `Handler::message` (at most once every 2 minutes per channel, `last_evaluation`, only when someone different wrote or there's no chat yet). `None` on error/malformed → falls back to a dice roll.
- `pick_target(pending) -> Option<String>` — picks who to reply to once 2+ different people have written: last 12 messages + pending names → fixed block TARGET_PICK{name}, variable = pending → `ask_split(..., 40)` → `extract_target` (JSON or plain text, matched against known names). Caller: `reply`; the chosen person's message becomes `reply_to`, and it's noted in the instruction.
- `determine_mood(history) -> Option<String>` — determines this chat's mood: ANALYST fixed, MOOD{name} variable, the chat's own history goes along as the message list (images are set to `None` in this copy: no point sending an image payload for a 40-token analysis, so a vision-less route doesn't error) → `ask_split(..., 40, "ruh_hali")` → `extract_mood` (intensity <3 counts as None, neutral). Caller: `reply`, only when a chat opens (`counter==0`) and every 4 turns after; the result is written to `Chat.mood` and added to the instruction as a "ŞU ANKİ RUH HALİN" line.
- `send(ctx, channel, text, ping, file, reply_to: Option<MessageId>)` — with `reply_to` set, it becomes a discord reply (`reference_message`) and pings whoever's being replied to (`replied_user`). Mentions are off (`CreateAllowedMentions::new()`, only `ping` turns it on), with an optional attached file; on success it's added to `own_messages` (50). The lock is taken AFTER sending.
- `Bot::ask_split(fixed, variable, history, budget: Option<u32>)` — sends the system message as two text blocks via `system_json`, the first with `cache_control: ephemeral`; no max_tokens if budget is `None`.
- `system_json(fixed, variable) -> Value` — plain system if variable is empty, two blocks otherwise. Free function.
- `Bot::is_repeat(channel, reply)` — is it the same as one of the channel history's last 5 bot lines. `Bot::research(text) -> Option<String>` — a page, RSS, or Firecrawl search result triggered by a link/news/research cue.
- `system_text(&State, instruction, retrieved) -> (String, String)` — (fixed, variable); appends the sections in order (the list in architecture.md). Free function, called under lock.

### Chat engine
- `Bot::post_problem(ctx, channel)` — a made-up code problem via `generate(PROBLEM, 160)`, sends it, opens a chat. Called by the poke cycle (25%) and `/sorun`.
- `start_chat(&mut State, channel, opening: Option<String>) -> &mut Chat` — seeds with the channel history's last 10 lines (bot lines as assistant). The opening was already sent and has landed in history LINE BY LINE: the trailing bot block in the seed is scanned and lines matching the opening's lines are dropped (something else, like a news link, might have landed in between), so the model doesn't see the opening twice; returns the existing chat if there is one (`entry().or_insert`), otherwise a new one; with an opening, adds an `assistant` message + `counter=1`.
- `end_chat(&mut State, channel) -> Option<Chat>` — clears the news wait, removes and returns the chat; no channel ban, closing comes from `close_timed_out`.
- `Bot::close_timed_out(ctx)` — on the minute tick: closes chats quiet for `CHAT_TIMEOUT` (30 min) that aren't busy, hands the transcript to `diarist`+`critic`, `growth.chats++`; also clears out-of-time news chats (the comment window has passed and there's no activity in it, it closes silently, the `awaiting_comment` map doesn't bloat).
- Commands → inside `src/command.rs` (below, the slash command handler).
- `Bot::post_news(ctx, channel) -> bool` — news_agent → link → teaser → send → chat + a 2-hour comment window. Called by `news_cycle` and `/haber`.
- `Bot::run_prank(ctx, channel, hack)` — picks an image, `HACK_ENTER` if hacking, otherwise `image_commenter`; the text goes through `parse_reply` and only the **first line** is kept (the image travels in a single message, line-bursting makes no sense here); the joke is skipped if the model goes silent; sends; opens a chat (`hacked=3`). Called by `prank_cycle` and `/saka`/`/hack`.
- `Bot::reply(ctx, channel)` — the loop: (1) lock: exit if busy; exit if there's no chat; pick an instruction and mark busy. (2) 0.15-0.35s message-batching grace period; fresh history, target message, `incoming`; mood (every 4 turns), any `research` finding and target-person note get added to the task; if `too_many_questions`, add a "don't ask this time" instruction; `broadcast_typing`. (3) open a stream with `generate_stream` (budget `reply_budget!()`). (4) `send_stream` (`reaction_target = last_message`): each line its own message, thinking per mode, repeated lines dropped. (5) `Silent` → **nothing** is written to history/counters/`last_activity`, the fallback `generate` isn't called; loop again if a new message arrived, otherwise exit. (6) `Empty` → loop again if a new message arrived, otherwise a non-streaming fallback via `generate` + `send_lines` (exit if that goes silent too). (7) clear busy, add the assistant line (`protocol_text`), advance counters, refresh `last_activity`. Loop back to the top if a new message arrived. No closing here; a quiet chat gets closed by the timeout.

### Memory (discord side)
- `read_history(bot, ctx, guild)` — fetches the bot's membership, walks permitted (`VIEW_CHANNEL|READ_MESSAGE_HISTORY`) text channels in position order, reads 14 days back with `GetMessages` in pages of 100, skips bot/empty messages, turns mentions into names with `content_safe`, sorts by time; writes `favorite_name` on seeing the favorite's id, fills `name_to_id` only when empty (a live mapping takes priority). History is prepended to the FRONT of memory (live messages arriving while the scan runs stay at the back, chronology isn't broken, the backfill never overwrites the live ones).
- `default_channel(bot, ctx) -> Option<ChannelId>` — `news_channel` → the guild's system channel → the topmost text channel. From cache, no await.
- `idle_channel(bot) -> Option<(ChannelId, String)>` — the last channel talked in; no open chat, not banned, has a profile → (channel, last 40 lines). Used by poke and prank.

### Cycles (`run_cycle`, started once in `ready`)
`run_cycle(name, setup)` — starts every cycle under a watchdog: logs + restarts after 5s on
panic, restarts on a clean return too (the cycles are infinite). `SHUTTING_DOWN` (AtomicBool) is
the shutdown signal: cycles check it at the start of each tick and return without the watchdog
restarting them; `main`'s shutdown task sets it.
- `news_cycle(bot, ctx)` — every 6 hours: **if asleep**, picks news but doesn't post it, stashes it in `stashed_news` (once); if traveling, profiler+coach, skip; while awake: profiler → observation queued → coach → skip if a chat is already open in the default channel → post the stash with `send_news(stash)` if there is one, otherwise `post_news`.
- `poke_cycle(bot, ctx)` — hourly: skip if not awake; if traveling, once a day at 25% send `ON_THE_WAY`; if travel starts tomorrow, `LEAVING` once; otherwise 30% `OUT_OF_THE_BLUE`; `idle_channel` → `generate(last 40 lines)` → send → start a chat.
- `prank_cycle(bot, ctx)` — every 3 hours: skip if not awake/traveling; 10%; `idle_channel`; skip if `random_image` is empty; 30% hack: `generate(HACK_ENTER)`, otherwise `image_commenter(image)`; send with the image; start a chat, `hacked = 3` if hacking.
- `wanderer_cycle(bot)` — 10 min after startup, then every 4 hours, `wander` if awake.
- `Bot::sleep_transition(ctx)` — logs the falling-asleep/waking transition; while asleep, marks `sleep_start`+`sleep_start_memory_len`. On waking: if there's a pending mention, replies with `WOKE_UP` (the list is put back on error); otherwise `evaluate_waking` evaluates the night's messages. Called by the cycle and `/uyan`/`/uyu`.
- `Bot::evaluate_waking(ctx, night)` — `analyze(WAKING{name}, 100)` → `{"ilgi","konu"}`; if interest ≥5, `generate(WAKING_REPLY{name,topic}, 250)` sends a morning line + starts a chat in the last channel talked in.
- `sleep_cycle(bot, ctx)` — every minute: `sleep::update`, logs the wake/sleep transition; on waking, if `pending_mentions` isn't empty, replies to the last mention's channel with `generate(WOKE_UP)`, starts a chat.

### Discord events (Handler)
- `ready` — logs the bot's name; registers slash commands on every connect (`modal::register_commands`, idempotent); starts the five cycles the first time (`started`).
- `interaction_create` — `Command` → looked up by name in the `command::definitions()` table and run (each command produces its own embed reply); `Modal` → a brief ephemeral confirmation (no input is collected); `Component` → the thought button (`thought_button`) or the mind detail layer (the `MIND_TOPICS/EVENTS/SUMMARY` buttons open a section modal, the `MIND_PERSON_PICK` menu opens a person modal).
- `guild_create` — the first time a guild enters `scanned`, runs `read_history → profiler → coach (if temperament is empty)` in the background.
- `guild_member_addition` — channel: guild system channel → default; if it's the favorite, saves their name; skips if a chat is open/banned there; `generate(WELCOME)` → sent with a mention (ping on) → starts a chat.
- `reaction_add` — only cares about a reaction on the **bot's own message**, from a human, in a guild, passing the `GUILD_ID`/`CHANNELS` filter (everything else returns immediately). Since the `Reaction` event carries neither the reactor nor the message text, both are fetched over HTTP via `add_reaction.user`/`.message`; a reaction on a message with empty text (embed-only) is skipped. `reaction_label` makes the emoji readable (unicode as-is, a custom emoji as `:name:`, not Discord's raw `<:name:id>` mention form). The result lands as a `"(tepki 💀) \"...\" mesajına tepki verdi"` line in `remember`+`channel_note`, and in `chat.history` too if a chat is open in that channel — **it never triggers a reply on its own**, it only becomes context for the next natural reply. Logged as a trace if `debug` is on.
- `message` — returns for bot/webhook/DM; returns if outside `GUILD_ID`/`CHANNELS`; `content_safe`; **image attachment:** the URL of the first attachment whose `content_type` starts with `image/` is taken from `attachments`, and the early-return condition becomes "neither text nor an attachment", not "text is empty" (a message that's only an image still gets processed); the text is marked as `[resim] <text>` or `[resim attı]` and this marked text is exactly what goes into memory, the channel note, and the chat line — the URL only goes into the chat history's `ChatMessage.image`, and adding a new user line sets earlier entries' `image` to `None` (only the latest image goes to the model). **Phase 1 (locked):** was it tagged (mention list, replied-to message is the bot's, bot name appears in the text) → `remember`, `name_to_id`/`usernames`, `last_channel`, favorite's name; if the news wait has expired, close that chat; **if asleep**: queue in `pending_mentions` (20) if tagged, return; `ongoing_dialog` — a chat is open AND the sender of that chat's last user message is the same person who sent this one (genuinely talking to them) → answered directly, willingness evaluation skipped. A tag is likewise answered directly. If neither (someone else wrote in the channel, or there's no chat) a willingness evaluation is needed (at most once every 2 minutes per channel). **Phase 2 (unlocked):** `willingness()` if needed; joins if the score ≥ the threshold (stage ±1, travel +2); a fallback dice roll (`CHANCE`) if the call fails. **Phase 3 (locked):** `start_chat` if joining, add the user line to history (kept at 20), `channel_note`. Outside the lock: `reply`. Note: this removes the old design's behavior of auto-replying to EVERYONE in an already-open chat's channel — only to its actual counterpart.

### Startup
- `setting(name)` — a non-empty env var or a hard error.
- `wait_for_shutdown()` — ctrl-c or SIGTERM.
- `Bot::setup() -> Result<Arc<Bot>, BotError>` — provider selection (`PROVIDER`/keys/`MODEL`/`API_URL`), `NEWS_CHANNEL`/`GUILD_ID`/`CHANNELS`, the `durum/arsiv/` + `photos/` folders, `memory::init(durum/hafiza.redb)`, `State::load` + `sleep::update` + `durum/model.md`, a reqwest client. **Doesn't connect to Discord, doesn't need DISCORD_TOKEN**: both `main`'s bot path and `cargo run -- chat` go through this (pulled out into one function so both see the same setup).
- `main` — `.env`, logging, panic hook; if the first argument is `chat`, runs `Bot::setup()` + `Bot::chat_cli()` (a one-line message + exit code 1 on a setup error) and returns. Otherwise `DISCORD_TOKEN` + `Bot::setup()`, intents `GUILDS|GUILD_MESSAGES|GUILD_MEMBERS|MESSAGE_CONTENT`, `shard_manager.shutdown_all` on shutdown.

- `version_text()` — `v{CARGO_PKG_VERSION} ({VERSION_COMMIT}, {VERSION_DATE})`; the two env vars are filled from git at build time by `build.rs` (`?` if git/date aren't available). Used by `modal::status_message` and the `guild_create` version announcement.

## src/command.rs (+ src/command/*.rs) — the slash command handler
The bot is managed only through slash (`/`) commands; there's no `!`/text command,
`Handler::message` no longer parses text as a command (every message goes straight into the
chat/memory flow). A single registration table, `definitions()`: each `CommandDefinition`
carries a name + description + Discord options (`CreateCommandOption`) + handler (registered via
the `define_command!` macro as `fn(&Bot,&Context,&CommandInteraction) -> Pin<Box<dyn
Future<...>+Send>>`). `modal::register_commands` derives the registration list for Discord from
this table, and `interaction_create` (main.rs) looks up `Interaction::Command` by name in the
table and runs it — the command name is never hand-maintained in two places. (Slash command
NAMES — `durum`, `zihin`, etc. — stay Turkish; the Discord-facing surface, see AGENTS.md item 8.)
- Commands: durum · yardim · zihin(`test`) · ayarlar · sifirla(`hepsi`) · haber · sorun · gez ·
  saka · hack · ajanlar · uyan · uyu(`saat`) · dusunme(`kip`) · model(`id`) · debug(`durum`).
- Replies: every command returns an embed, never plain text. Local/fast commands
  (durum/yardim/ayarlar/zihin default view/sifirla/dusunme/model-query/debug) reply directly with
  `CreateInteractionResponse::Message` (`send_response`/`reply_info`, embed via
  `modal::info_embed`). Commands that make a network/model call
  (haber/sorun/gez/saka/hack/ajanlar/uyan/uyu/zihin `test`/model id change) could exceed
  Discord's 3-second initial-reply limit, so they acknowledge instantly with `defer` (`Defer`)
  first and write a short result embed with `report_result` (`edit_response`) once the work is
  done — the actual content (news/joke/etc.) already went to the channel via its own
  `Bot::send` call, this is just an "OK" note.
- `zihin` with `test:true` does the old `!zihin test` diagnostic: hands the channel's last 30
  lines straight to the diarist (so the mind chain can be seen without the 40-minute wait); with
  no option, it's `modal::mind_message` (person menu + section buttons → detail modals).
- The `dusunme` option values (`goster/gizle/sessiz/kapat`) are kept exactly in sync with the
  strings `ThinkingMode::from_arg` recognizes (test: `thinking_mode_options_match_from_arg`); an
  argument-less call reports the current mode.
- `model id`: can only change FAVORITE; `model_exists(id)` looks it up in OpenRouter's
  `/models` list, and doesn't block if the list can't be fetched.
- `Bot::wake/put_to_sleep/set_debug` (the state-writing helpers) stay inside `impl Bot`,
  commands call them.

## src/chat_cli.rs (impl Bot)
A terminal chat rig for trying out the output protocol (line = message, `tepki:`, `-`) without
connecting to discord. Opened with `cargo run -- chat` (from `main`).
- `CLI_CHANNEL = 1` — a fake channel id; not a real discord channel, just the key for the chat state.
- `append_history(&mut State, channel, line)` — appends to the channel history **in memory
  only** (bounded by `CHANNEL_HISTORY`). Not the same as `channel_note`, which also writes to
  disk — this rig must never touch the real `durum/kanallar/*.md` files. (`remember` is already
  memory-only, so that one is called as-is.)
- `parse_line(line) -> (name, text)` — `"name: text"`; if there's no colon, or either side is
  empty, the sender is `misafir` and the whole line is the text.
- `Bot::chat_cli(&self)` — if `bot_name` is empty (`ready` never fires here), falls back to
  `growth.name`, then to `"bot"`; `start_chat` seeds from the real `durum/` files (so the
  personality stays realistic). stdin is read blockingly, line by line (no background cycle runs
  in this mode, so a separate reader isn't worth it); `!quit` or EOF exits. Each turn: `remember`
  + memory history + a `user` line added to chat history → the `too_many_questions` instruction
  → `generate` (**no streaming**: it's the output protocol being tried out here, not the
  streaming pace) → `strip_name` → `parse_reply` → each line as `"bot_name: line"`, a reaction as
  `[reaction 💀]`, silence as `(silent)`, nothing as `(empty)`, a model error as `(error: …)`
  (the loop continues) → `protocol_text` appended to history, `counter++`.
- Tests: `line_parses`, `history_limited_in_memory`.

## src/modal.rs
The command surface: slash commands return ephemeral **embed cards** (sectioned, like a web
page), with details spread across **labeled modal fields** — nothing gets dumped into a single
text box. Discord limits: embed field value ≤1024, modal ≤5 components × value ≤4000,
title/label ≤45, select menu ≤25 options (label ≤45, description ≤100).
- `info_embed(title, description)` — commands' short confirmation/status reply (e.g. "couldn't
  find news", "ok, {model}"); `reply_info`/`report_result` in command.rs wrap this, plain text
  never goes out. `token_breakdown(m)` — categories sorted by total tokens; used by
  `status_message` and `summary_modal`.
- `mind_embeds(state)` — a single card: description holds stage/day/model/mode; three inline
  fields: People (first 8: name+score+tag), Topics (first 8: name+latest note), Events (last 5,
  newest month first, chronological); footer is bot name + date. An empty field shows "—".
- `mind_components()` — row 1: a person select menu (`MIND_PERSON_PICK`, ≤25 people, value=id,
  description=tag+note); row 2: Topics/Events/Bot-summary buttons.
- `mind_message(state)` / `status_message(state)` / `help_message()` — ephemeral
  `CreateInteractionResponseMessage` (embed + components).
- Detail modals (`person_modal(id)` / `topics_modal()` / `events_modal()` /
  `summary_modal(state)`) — each topic in its own labeled field: person = Identity/Impression/
  Tags/What-they-know(last 8)/Recent-events(last 5); topics = Recently-changed(15)+Other; events
  = one field per month (last 3 months, last 10 records of each month, titled "Eylül 2026");
  summary = Status/Tokens/Myself/Agenda. Empty sections are skipped, and if all are empty there's
  a single "(henüz boş)" field.
- `fit_to_limit(text, limit)` — a limit overrun is cut at a line/space boundary + a note.
  `month_name("2026-09")` → "Eylül 2026".
- `register_commands(http, guild)` — derives the guild's command list from the
  `command::definitions()` table (name/description/options in one source); idempotent on every
  ready.
- `memory.rs` helpers: `person_summaries` (a `Person` list in mtime order), `topic_summaries`
  (name + latest note), `event_months(n)` (the last n months' "- " lines, newest month first).

## src/agents.rs (impl Bot)
- `profiler()` — last 600 lines → `analyze(PROFILE_EXTRACT, 1200)` → `profil.md` + `State.profile`.
- `diarist(transcript, source, channel)` — `analyze(DIARIST{name,source,favorite}, 1200)` → JSON
  `Record{olay, kisiler[{isim,puan_degisimi,not,bilgiler,etiketler}], konular[{ad,not}], kendim}`
  (field names kept Turkish so they match the model's output, see prompts/gunlukcu.md); if the
  JSON fails to parse, the raw output is salvaged to `arsiv/gunlukcu-<source>.md` (the work isn't
  lost) → an event line (`memory::add_event`, with seconds); each person: the name is converted
  to an id via `State.name_to_id` (unresolved ones are skipped, logged), `kisiler/<id>.md` is
  read, a changed name goes to `previous_names`, score += clamp(-3..3) then clamp(-10..10),
  note/facts/tags, +10 and a fixed note for the favorite, `memory::write_person`; topics via
  `memory::add_topic`; myself → `kendim.md`; the index is refreshed; then `summarizer`.
- `summarizer()` — for whatever `memory::over_limit()` returns: a person →
  `analyze(SUMMARIZER_PERSON{limit=1000}, 700)`, a topic → `SUMMARIZER_TOPIC{800}`, events → the
  oldest 60% of lines go through `SUMMARIZER_EVENTS` down to 3-5 lines, the newest 40% stay as
  is. If the result isn't empty and is shorter than before: for a person/topic the old file goes
  to the archive and the new one is written; for events the lines that moved go to the archive.
  Untouched if it didn't shrink. The index is refreshed.
- `send_news(ctx, channel, item) -> bool` — shares the chosen news item (either this round's pick
  or the stash from sleep); teaser via `generate(NEWS_INTRO)`, opens a chat, waits 2 hours for
  comments.
- `coach()` — profile + index + agenda + myself + current temperament + last 200 lines + the
  bot's own recent messages → `analyze(COACH{name}, 800)` → `huy.md`.
- `critic(transcript)` — `analyze(CRITIC{name,current}, 400)` → `duzeltmeler.md`.
- `news_agent() -> Result<News>` — HN's first 12 (not yet posted) + Sözcü RSS's first 12 (not yet
  posted, id = link hash) → a list of "n. [hn|gündem] title" → `analyze(NEWS_PICK{profile}, 10)`
  → a number → `News{id,title,url,score,source}`.
- `image_commenter(&PathBuf) -> Result<String>` — sends the image as base64 `image_url` with
  system=`system_text(IMAGE_POST)` via `ask_raw`; falls back to `generate` blind on error. `clean`.
- `random_image() -> Option<PathBuf>` — a png/jpg/jpeg/gif/webp from `photos/`.
- `News` — serde; `source` is `#[serde(skip)]`, `score` is 0 for non-HN.

## src/memory.rs
- Constants: `PERSON_LIMIT 1800 / PERSON_TARGET 1000 / TOPIC_LIMIT 1500 / TOPIC_TARGET 800 /
  EVENT_LIMIT 6000 / CONTEXT_BUDGET 6000 / INDEX_PEOPLE 40 / FAVORITE_NOTE`.
- `init(path)` — opens/creates `durum/hafiza.redb`, fills the process-lifetime
  `static DB: OnceLock<Database>` (called once, from `Bot::setup`). `read(key)`, `write(key,
  content)` — the key matches the old relative file path (e.g. `"kisiler/1.md"`); `write` writes
  both the content and `now_unix()` in a single redb write transaction (atomic, `WRITE_LOCK` is
  gone — redb serializes its own writers). `append(key, line)` — get+insert in a SINGLE
  transaction (no race, thanks to redb's own serialization). `archive(key, content)` is the one
  exception: it still writes to a real file (appends to `arsiv/key` with a dated header, guarded
  by `ARCHIVE_LOCK`), because `arsiv/` was never moved into redb (it's for humans only, see
  `docs/state-files.md`).
- `person_summaries` / `topic_summaries` / `event_months` — mtime-sorted dumps for the modal
  display.
- `slug(name)` — lowercase, simplifies Turkish letters, non-alphanumerics become `-`, "bilinmeyen"
  if empty.
- `date()`, `date_from_unix(unix)` (Hinnant civil-from-days), `month()` "YYYY-AA".
- `Person { id, name, username, previous_names, score, tags, note, facts, events }` —
  `parse(id, text)` from a file, `text()` to a file; the file is `kisiler/<id>.md`. Format (file
  field names stay Turkish, see AGENTS.md item 8): `# İsim` / `id:` / `kullanici_adi:` /
  `eski_adlar:` / `puan: +3` / `etiket: a, b` / `not: ...` / `## Bildiklerin` `- ...` / `## Son
  olaylar` `- tarih saat: ...`.
- `read_person(id)`, `write_person(&Person)` — `kisiler/<id>.md`.
- `add_topic(name, note)` — `konular/<slug>.md`, a title+tag line if it doesn't exist yet, then
  `- date: note`.
- `add_event(channel, event)` — appends `- date #channel: event` to `olaylar/YYYY-AA.md`.
- `files(folder)` — `.md` files, most recently changed first. `first_line(p)`.
- `refresh_index() -> String` — `## Kişiler` (≤40: `- name (+score) · tags · note`), `## Konular`
  (≤30: `- name · latest: date`), `## Olaylar` (≤3 months: `- YYYY-AA · n record(s)`); written to
  `INDEX.md`.
- `STOPWORDS` — commonly filtered words. `keywords(&[String])` — 4+ letters, not a stopword, no
  duplicates, ≤40.
- `score_matches(text, keyword)` — how many keywords appear. `trim(text, limit)` — a character
  limit + `…`.
- `retrieve(participants, name_to_id, keywords, memory, exclude_recent) -> String` — in order:
  participants' person files (names resolved to ids via `name_to_id`, ≤4, each ≤1200), the 2
  best-matching topic files (≤800), the current month's last 8 events, up to 12 lines from raw
  memory (excluding the last `exclude_recent`) matching ≥2 keywords (sorted by score then
  recency, then chronologically). Budget 6000 characters; a section that doesn't fit, and
  everything after it, is skipped.

## src/migrate.rs
- `run(args) -> Result<(), BotError>` — `cargo run -- migrate-durum [--from <dir>] [--to
  <redb-path>] [--dry-run] [--force]`. Collects every `.md` file (except `arsiv/`) in an old
  `durum/` tree via `collect` (key + content + the actual OS mtime), prints counts, then writes
  to the target redb with `memory::init` + `memory::write_with_mtime`. Refuses if the target is
  already populated, unless `--force` is given. Never touches the original files.
- `over_limit() -> Vec<(kind, path)>` — person/topic files over their size limit and this
  month's event file.

## src/agenda.rs
- Constants: `RSS_URL`, `AGENDA_ENTRIES 12`, `PAGE_LIMIT 3500`.
- `clean_html(raw)` — drops CDATA, script/style blocks, tags; basic entities; whitespace collapsed.
- `tag_content(chunk, tag)` — the content inside `<tag>`/`<tag ` … `</tag>`, cleaned.
- `rss(http) -> Result<Vec<RssNews{title,link,summary}>>` — split on `<item`; a title and an
  http link are required.
- `link_id(link) -> u64` — DefaultHasher; used to track posted news.
- `entries(text) -> Vec<String>` — splits `gundem.md` into `## `-headed entries.
  `latest_agenda(text)` returns the last 3.
- `Bot::read_page(url)` — with a firecrawl key, `POST api.firecrawl.dev/v1/scrape {url,
  formats:[markdown], onlyMainContent}` → `data.markdown`; otherwise a plain `GET` +
  `clean_html`. 3500 characters.
- `Bot::firecrawl_search(query) -> Result<String>` — `POST /v1/search` limit 5; title,
  description, and address lines.
- `Bot::wander()` — first 20 from rss → `analyze(WANDERER_PICK{name,temperament,profile}, 20)` →
  ≤3 numbers → each fetched with `read_page` (a summary on error) → `generate(WANDERER_NOTE,
  350)` (with personality, its own journal entry) → a `## date time` entry in `gundem.md`; the
  oldest entry over 12 goes to the archive; `State.agenda` = the last 3.

## src/sleep.rs
- Constants: `TIMEZONE_OFFSET +3h`, `INSOMNIA_CHANCE 0.07`, `INSOMNIA_TENSE 0.20`.
- `Plan { day, insomnia_start: Option<i64>, start, end }` — one night's plan (unix seconds).
- `local_time(unix) -> (day no., seconds into the day)`, `time()` "SS:DD", `time_text()`
  "YYYY-AA-GG dayname SS:DD".
- `jitter()` ±45 min. `is_tense(&State)` — does `myself`+`temperament` contain
  hurt/angry/tense/obsessive/sleep/head/burnt-out-ish wording.
- `build_plan(day, tense)` — normal: 01:00±45 → 09:00±45; sleepless: up at 01:00, 06:00±45 →
  13:00±45.
- `update(&mut State)` — builds a plan for yesterday and today if missing, drops a finished one.
  `is_awake`, `status_text` (the "ŞU AN" line).

## src/travel.rs
- `Trip { place, reason, start, end }` (local day number). `Event` table `EVENTS` (yearly +
  year-specific holidays for 2026-2027).
- `day_number(y,m,d)` — Hinnant days-from-civil. `year_of(day)`.
- `on_day(day) -> Option<Trip>` — scans the table for this year and last year (for the
  new-year overhang); place = `(y + month*31 + day) % places.len()`, kept stable.
- `today()`, `now()`, `tomorrow()` (starting tomorrow, not today). `status_text()` — "Şu an
  X'desin (...); n gündür, m gün sonra dönüyorsun" / "Yarın X'ye gidiyorsun" / empty.

## src/growth.rs
- `Stage { name, min_days, min_chats, confidence, poke, description }`, `STAGES` (4 stages),
  `NAME_STAGE = 2`.
- `Growth { birth, chats, messages, stage, name }` — `load()` from `durum/gelisim.md` (birth =
  now if missing), `save(&Growth)` writes `key: value` lines.
- `days(&Growth)` days since birth. `earned_stage(&Growth)` the highest stage whose day and chat
  thresholds are passed. `stage(&Growth)` the current stage. `stage_text` the "GELİŞİM EVREN"
  section.
- `clean_name(&str) -> Option<String>` — the first word, alphanumeric, 2..20 characters.
- `Bot::check_growth(ctx)` (main.rs) — on every finished chat and every 6-hour round: advances
  and saves the stage if the earned one is higher than the current one; picks a name once the
  stage ≥ `NAME_STAGE` and there's no name yet.
- `Bot::pick_name(ctx)` (main.rs) — `generate(NAME_PICK, 12)` → `clean_name` → `edit_nickname`
  in every guild → `growth.name`, `bot_name` → `generate(NAME_ANNOUNCE{name})` to the default
  channel + a chat. Tag detection recognizes both the chosen name and the username.

## src/prompts.rs
Nothing but `pub const X: &str = include_str!("../prompts/x.md");` lines. See docs/prompts.md.

## Added today (2026-09-02, version/debug/settings/reasoning)
- `response_content(&Content, category)`, `thought_length`, `JSON_CATEGORIES`,
  `Bot::grow_budget`, `Bot::reasoning_low_effort` — agent-call resilience for reasoning-mandatory
  models (`ask_raw`).
- `agents::DiaristSummary`; `diarist` now returns a result; `/zihin test:true` (command.rs).
- `State.debug`, `Bot.debug_channel`, `Bot::debug_note`, `Bot::debug_trace`, `Bot::set_debug`,
  `parse_willingness` (score+reason; `willingness` is now `Option<(u8, String)>`).
- `modal::settings_embed/settings_components/settings_message`, `SETTING_*` ids,
  `Handler::setting_button`, `Bot::wake/put_to_sleep` (sleep paths shared with the commands).
- **Reverted the same day**: the panel image (`zihin_gorsel.rs`, `resvg`, embedded fonts,
  `Bot::gonder_ekli`, `cargo run -- zihin`) was removed entirely, `/zihin` embed+button+modal
  became the only path; the `!`/text commands (`Bot::komut`) were dropped in favor of the
  `command::definitions()` slash registration table (see `docs/decisions.md`).

## 2026-09-03: code translated to English
Identifiers, comments, file/directory names were translated to English (see AGENTS.md item 8
and docs/progress.md). Every code reference in this file was updated to the new names. What
stayed Turkish is unchanged: `prompts/*.md` (directory+file name+content), `durum/` file formats
(field names, file names — the JSON/file fields of types like `Person`/`Record` were kept
Turkish so they match the model prompts), everything Discord-facing (slash command names, embed
text, button/menu labels).
