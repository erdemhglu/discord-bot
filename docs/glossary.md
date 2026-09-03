# Glossary (runtime terms that stay in Turkish)

Identifiers on the code side are English (see AGENTS.md item 8). This glossary is no longer
a code glossary: it explains the bot's surface that **stays in Turkish** (prompts, `durum/`
file fields, category labels shown on Discord, slash command names) to a developer reading
in English.

| Turkish term | Meaning / where it appears |
|---|---|
| durum | state — both the old name of the shared `State` struct and the name of the `durum/` folder |
| hafiza | memory — both the old name of `src/memory.rs` and the concept of "memory" |
| sohbet | chat — an open conversation in a channel |
| gecmis | history — a chat's messages (`Chat.history`) |
| sayac | counter — the number of messages the bot has sent in that chat |
| hackli | "hacked" — the number of rounds remaining in the hack prank |
| mesgul | busy — a reply is currently being generated in the channel |
| yasakli | banned — (no longer used, the channel ban was removed) |
| profil | group profile — `profil.md`, produced by the profiler agent |
| huy | temperament — `huy.md`, produced by the coach agent, the bot's evolving personality |
| duzeltmeler | corrections — the critic agent's `duzeltmeler.md` |
| kendim | "myself" — the diarist agent's `kendim.md`, the bot's own current state |
| gundem | agenda — opinions the wanderer agent writes after browsing the internet (`gundem.md`) |
| planlar / uyuyor | sleep plans / is asleep |
| son_yol_mesaji / duyurulan_seyahat | last on-the-road message day / announced trip (old field names, now `last_road_message`/`announced_trip`) |
| gezgin | wanderer — the agent that browses the internet (its code name is now `wander`/`wanderer_cycle`) |
| haberci | news agent — its code name is now `news_agent` |
| profilci | profiler — still appears as a category label in the `!durum` breakdown shown on Discord |
| gunlukcu | diarist — the agent that writes the chat into memory; stays as a category label |
| hoca | coach — the agent that shapes the personality; stays as a category label |
| elestirmen | critic — stays as a category label |
| ozetleyici | summarizer/compactor agent |
| resimci | image commenter agent; stays as a category label |
| kisi / kisiler | person / people — `kisiler/<id>.md` |
| konu / konular | topic(s) — `konular/<slug>.md` |
| olay / olaylar | event(s) — `olaylar/YYYY-AA.md` |
| arsiv / arsivle | archive |
| kanaat / puan / not | opinion / score / note — person file fields |
| bilgiler / etiket | facts / tags — person file fields |
| tarih / ay / saat | date / month / time |
| uyku / uyanik_mi / uykusuz | sleep / awake? / sleepless |
| gergin | tense — a personality-linked insomnia trigger |
| seyahat / yolda / gidiyorum | travel / on the road / I'm leaving |
| etkinlik | event (calendar) |
| yer / sebep | place / reason |
| favori | favorite — the always-favored user |
| ayar | setting — an env variable or the `/ayarlar` panel |
| talimat | instruction — a call's task text |
| kaynak | source |
| gelisim / evre / hak edilen | growth / stage / earned stage |
| dogum | birth — the moment it first ran (unix timestamp) |

## Turkish field names in the model's JSON output (deliberately not translated)
These fields are Turkish in the Rust structs too: because the prompts are Turkish, the model
produces JSON with these names, and serde matches by field name — if they were translated it
would silently come back empty/0.
- Willingness (`WILLINGNESS`): `puan` (0-10 score), `sebep` (single-sentence justification)
- Target selection (`TARGET_PICK`): `hedef` (person's name)
- Mood (`MOOD`): `durum` (e.g. "confusion"), `yogunluk` (1-10)
- Waking (`WAKING`): `ilgi` (0-10), `konu`
- Diarist (`DIARIST`): `olay`, `kisiler[].isim/puan_degisimi/not/bilgiler/etiketler`,
  `konular[].ad/not`, `kendim`

## The Turkish surface shown on Discord
Slash command names (`durum, yardim, zihin, ayarlar, sifirla, haber, sorun, gez, saka, hack,
ajanlar, uyan, uyu, dusunme, model, debug`), option names/labels, embed title/field text,
button/menu labels, and of course every reply the model produces — all of it stays Turkish
(see AGENTS.md item 8, README.md "commands"). The category labels in the `!durum` breakdown
(`sohbet, isteklilik, profilci, gunlukcu, hoca, elestirmen, ozetleyici_*, haber_sec, hedef_sec,
ruh_hali, uyanis, uyandim, isim_sec, hack_giris, sorun, laf, gozlem`) are also deliberately
kept Turkish/in-code — they both show up on Discord and changing them would break backward
compatibility of the metrics breakdown.
