# discord-bot

> Entry point for developers and AI agents: [AGENTS.md](AGENTS.md) · details in [docs/](docs/)

A Discord bot that hangs around in a server, gets to know people, and develops a personality
over time. Written in Rust, gets replies through OpenRouter (or Mistral); through OpenRouter
it can use any model — GLM, Grok, Gemini, Claude. `z-ai/glm-5.3-flash` (reasoning
mandatory) has been observed to perform well.

## what it does

- on first joining a server, reads the last 2 weeks of messages and gets to know the group (once; doesn't re-scan on later restarts)
- greets a newcomer, joins the chat
- drops into conversation now and then; a chat that's been quiet for 30 min closes itself without a goodbye
- once an hour, 30% odds it says something unprompted, referencing an old topic
- every 6 hours, posts a news item fitting the group from hacker news + the Sözcü news agenda, waits 2 hours for comments
- every 4 hours, browses Turkey's news agenda (Sözcü RSS, reads the page via firecrawl if available), writes its own take into its journal; the personality is fed from this too
- sleeps at night (01:00-09:00), rarely has a sleepless night (more often when its mood is off); doesn't write while asleep but still listens: messages are processed into memory, news is stashed; on waking it evaluates what was written overnight, and replies with a morning line if something caught its interest
- always replies when mentioned, named, or replied to; while a chat is open it also automatically continues with whoever it JUST talked to, without jumping to someone else's message in the channel before weighing willingness (like a real person would)
- a Discord reply-to is only used when mentioned or when more than one message came in between; in an ordinary one-on-one chat it just sends a plain message
- each chat has its own moment-to-moment mood (from categories like cognitive, fear, positive, low, anger, social reasoning); it's woven into the tone, never announced
- chat replies stream live: the message appears and grows as it's written; if the model produces a thought (reasoning) it's shown unclipped in a spoiler, a reply over 1900 characters isn't truncated, it's split into a new message
- posts short messages back-to-back instead of one long one: **every line the model writes goes out as its own message** (at most 4 lines per turn — usually 4 messages; a line over 1900 characters is further split, thought messages are also added in thinking "show" mode). Most replies are a single line; a neutral/informational remark isn't split, an excited one is
- **stays silent** when it has nothing to say: if the conversation isn't directed at it, or joining in would stick out, it sends nothing at all (even while a chat is open)
- can leave an **emoji reaction** instead of text; a reaction can also arrive alongside a line of text
- notices when someone reacts to one of **its own** messages: the reaction (and what it was on) is noted the same way a message is, and enters an open chat's context — it doesn't trigger a reply by itself, but the bot may bring it up next time it naturally speaks
- **sees** an image if you post one: the attached image goes to the model (only the most recently posted one), it doesn't describe it as "I see X in the image" — it comments or reacts like a person would
- doesn't ask questions back-to-back: if its recent replies have piled up questions, it's told not to ask one that turn
- can be locked to a single server/channel list with `GUILD_ID`/`CHANNELS` (.env, optional); runs everywhere it can reach if unset
- acts like it's traveling during holidays, long weekends, summer, festivals: writes less, posts on-the-road messages, gives notice before leaving
- posts an image from the `resimler/` folder now and then; sometimes with the "hacked" bit (runs for 3 messages,
  then snaps out of it; posts no links, asks nothing of anyone)
- growth stages: new → warming up → established → old hand (based on days and chat count); the stage changes its tone and confidence
- picks its own name on reaching the established stage, changes its nickname, tells the group
- `/zihin` shows its mind as a three-column **embed card** (People/Topics/Events); a person-picker menu on top, Topics/Events/Bot summary buttons below — the menu or a button opens the matching detail **modal** (visible only to the caller)
- the person with the `FAVORITE` id is an exception: it likes them no matter what

## who manages the personality

The talking side doesn't decide everything alone; separate agents run in the background
(`src/agents.rs`). All of them are personality-free, do plain analysis, and write the result to
the `durum/` folder; the talking side reads these on every reply.

| agent | when | produces |
|---|---|---|
| profiler | on startup and every 6 hours | group profile: how people talk, inside jokes, topics (`profil.md`) |
| diarist | after every finished chat and from a 6-hour observation | a person's score (-10/+10) and note, a topic note, an event line, the bot's own current state (`kisiler/`, `konular/`, `olaylar/`, `kendim.md`) |
| coach | on startup and every 6 hours | what kind of personality the bot should have: humor, language, topics it gets excited about, attitude, naturalness (`huy.md`) |
| critic | after every finished chat | concrete correction notes on the bot's own messages (`duzeltmeler.md`) |
| summarizer | when a person/topic/event file goes over its limit | shrinks the file, the overflow goes to `arsiv/` |
| news_agent | every 6 hours | picks a news item fitting the group from hacker news + Turkey's agenda |
| wanderer | every 4 hours | browses the news and writes its take into its journal (`gundem.md`) |
| image_commenter | at prank time | a one-line personality comment on the attached image (the model sees the image) |
| mood | when a chat opens and every 4 turns | that chat's moment-to-moment mood (not persistent, drifts with the chat; added to the instruction as "ŞU ANKİ RUH HALİN") |

Check the `durum/` folder to see what it has learned.

## memory architecture

A "second brain" so the context window doesn't grow: an index is carried around, data is
retrieved on demand, a record gets summarized once it hits its limit, nothing is ever deleted
(`src/memory.rs`). Everything below lives in one embedded database, `durum/hafiza.redb`
([redb](https://github.com/cberner/redb) — pure Rust, single file, ACID transactions), keyed
by the same names a plain-file layout would have used — `arsiv/` is the one exception, kept as
real markdown files since it's write-only and meant for a human to read, never the bot again.
Migrating an older plain-markdown `durum/` tree: `cargo run -- migrate-durum` (non-destructive,
see `src/migrate.rs`).

```
durum/
  hafiza.redb       everything below, as one embedded database (redb)
    INDEX.md          the list of what it knows; sent with every reply (person + score + tags, topics, event count)
    huy.md            coach: what kind of personality it has
    profil.md         profiler: group profile
    duzeltmeler.md    critic: notes to itself
    kendim.md         the bot's own current state
    gundem.md         wanderer: opinions formed while browsing the news
    kisiler/<id>.md   one per person (discord id, doesn't split even if the name changes): score, tags, note, what it knows, recent events
    konular/<ad>.md   dated notes per topic
    olaylar/YYYY-MM.md  one line per finished chat
    taranan.md        server ids already scanned for 14 days of history (so a restart doesn't re-scan)
  arsiv/            raw chunks dropped by summarizing — real files, human-inspection only
```

**What goes into every reply:** the core personality + growth stage + temperament + profile +
index + agenda + its own current state + notes to itself + what was retrieved for that chat +
that chat's moment-to-moment mood + the task. What's retrieved has a fixed budget (6000
characters): the person files of who's talking in the chat, up to 2 topic files matching a
keyword, the month's last 8 events, and up to 12 old lines from the raw context window (the
last 2000 messages) that touch the topic but aren't in the chat.

**Who writes it:** the diarist agent produces a JSON record from every finished chat and from an
observation every 6 hours; the code files it into the person/topic/event files. Score limits are
enforced in code, the favorite is fixed.

**Once a limit is hit:** the summarizer agent shrinks a person file past 1800 characters, a topic
file past 1500, a month's event file past 6000 (target 1000/800 for person/topic; for events the
older 60% is reduced to 3-5 lines). The dropped raw chunk is appended, dated, under `arsiv/`.

## prompts

Live as markdown under `prompts/<dil>/`, embedded into the build with `include_str!`.
Editing the text and rebuilding is enough. Core rules are in `kisilik.md`, each agent has its
own file. Which language is served — both these prompts and every Discord-facing string
(slash command names/descriptions, embeds, buttons; `langs/<dil>.json`) — is picked once at
startup from `BOT_LANG` (`.env`, default `tr`). `tr` and `en` are both filled in; adding
another language is a new `prompts/<dil>/` + `langs/<dil>.json` pair, no code change.

## setup

```
cp .env.example .env   # DISCORD_TOKEN + OPENROUTER_KEY or MISTRAL_KEY (MODEL picks the model; API_URL for a custom router)
                        # optional: GUILD_ID/CHANNELS (locks to a single server/channel), NEWS_CHANNEL, FIRECRAWL_KEY, BOT_LANG (default tr)
cargo run --release
```

The **Message Content** and **Server Members** intents must be enabled in the Discord developer
portal. Put images for pranks into `resimler/` (png, jpg, gif, webp); the folder isn't tracked by git.

## trying it from a terminal

```
cargo run -- chat
```

A chat bench from a terminal, never connecting to Discord. `DISCORD_TOKEN` isn't needed, only a
model key (`OPENROUTER_KEY` or `MISTRAL_KEY`); without one it prints one line and exits.
Input format is `name: text` (without a colon, the speaker defaults to `misafir`), `!quit` or
ctrl-d to exit. Reads `durum/hafiza.redb` normally so the personality feels real, but **writes
nothing to the state contents** (`Bot::setup()` opens/creates it either way; content is never
written from here). The output protocol shows as-is: each
line its own message, `[tepki 💀]`,
`(sustu)`. For seeing personality and prompt changes without trying them on a live server.

```
cargo run -- migrate-durum [--from <dir>] [--to <redb-path>] [--dry-run] [--force]
```

One-time (safe to re-run) import of an older plain-markdown `durum/` tree into
`durum/hafiza.redb`; see "memory architecture" above. Never touches Discord, never deletes or
moves the source `.md` files.

## commands

The bot is managed only via **slash (`/`) commands**, there are no `!`/text commands (plain
messages only ever feed the chat/memory pipeline). Every command returns an **embed card**
visible only to the caller.

- `/sifirla [hepsi]` — resets the channel ban and any open chat; `hepsi` for all channels.
- `/haber` — picks and posts a news item now (HN + agenda).
- `/sorun` — posts a software gripe and asks "how do I fix this" (also happens on its own in 25% of unprompted turns, to the dev channel).
- `/gez` — runs the agenda browse now (gundem.md gets updated).
- `/saka` / `/hack` — an image prank / the hacked bit, now.
- `/ajanlar` — runs the profiler and the coach now.
- conversations stay in `durum/kanallar/<id>.md`; continues with the last 10 lines even if the chat closed or the bot restarted
- `/uyan` — cuts sleep short right now, replies to anyone who mentioned it while asleep. `/uyu [saat]` — puts it to sleep for testing (default 8 hours).
- `/durum` — stage, counters, model, sleep, thinking mode, travel, token metrics (call count, input/cache/output, the biggest spenders by call type), and version.
- `/zihin [test]` — shows its mind as a three-column card (People/Topics/Events), a person-picker menu on top, Topics/Events/Bot summary buttons below. The menu or a button opens the matching **detail modal** — the person card is split into separate fields for Identity/Impression/Tags/Facts/Recent events, nothing gets dumped into a single text box; the modals are display-only, no input is collected from them, an overflowing field is cut at the last line break/space at 4000 characters. `test:true` feeds the channel's last 30 lines straight to the diarist and reports how many people/topics/events were written (a mind-pipeline diagnostic), without waiting 40 minutes.
- `/debug [durum]` — decision traces (willingness score/reason, target, mood, silence/reaction, chat closing) get posted to the channel; `DEBUG_CHANNEL` for a separate channel.
- `/ayarlar` — a settings panel with buttons: thinking mode, debug, sleep.
- `/dusunme [kip]` — thinking mode. `göster` (show): "Düşünüyorum..." while thinking, the thought shown in both a spoiler and a code block alongside the reply. `gizle` (hide): a live word counter while thinking ("Şu ana kadar N kelime düşündüm"), the thought never appears in the message, a "Show Thought Process" button at the end of the reply — opens a code block visible only to the clicker. `sessiz` (silent): thinks in the background, shows no trace at all. `kapat` (off): requests go out with reasoning disabled. The choice is persisted in `durum/dusunme.md`.
- `/model [id]` — the current model; changes it if `id` is given (only the FAVORITE person; "yok öyle model" if it's not on OpenRouter). Persisted in `durum/model.md`, survives a restart.
- `/yardim` — shows the command list as a card.

Local/fast commands (`durum, yardim, ayarlar, zihin, sifirla, dusunme, model, debug`) reply
directly; commands that make a network/model call (`haber, sorun, gez, saka, hack, ajanlar, uyan,
uyu`) defer first since they could exceed the 3s limit, then edit in the result.

## settings

The constants at the top of `src/bot/types/types_settings.rs`: message limit, reply token cap, odds
of joining in, wait times, prank frequency, the favorite person. Full constant list:
`docs/sabitler.md`.

## security

- mentions always go out disabled; only the chat reply's own recipient and a new member in the welcome message can get pinged
- never replies to other bots, webhooks, or DMs — no bot-to-bot loop can form
- generates at most one reply per channel at a time (guaranteed via RAII even on panic); spam can't inflate the API bill
- every request has a `max_tokens` limit (even a chat reply has a cap in release)
- http: no overall time limit (a long thinking stream isn't cut off); the connection itself times out at 15s, the gap between two chunks at 120s; a network error / 429 / 5xx backs off 2s then 4s and retries twice
- an "ignore your rules"-style instruction inside a message is treated as ordinary chat by the personality prompt
- the hacked-prank prompt forbids asking for links or information
- `GUILD_ID`/`CHANNELS` can lock access to a single server/channel
- `.env`, `durum/`, `resimler/`, `bot.log` aren't tracked by git
