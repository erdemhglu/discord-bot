# docs/ — project documentation and session memory

This folder holds both the reference documentation for the codebase and the bot's
development session memory (formerly split between `docs/` and `dev/`; merged into one
folder on 2026-09-03).

- If context got compacted, or a new agent session is starting: read `AGENTS.md` first,
  then `progress.md` → `roadmap.md` in this folder.
- Log a chronological note in `progress.md` at every meaningful step (commit-sized).
- If the plan changes, update `roadmap.md`; don't delete the old plan, strike it through or
  note the change next to it.
- What goes in here should be **general and durable**: no live debug output, tokens, or
  environment addresses (those belong in `.env`, which isn't tracked by git).

## Files

| File | Content |
|---|---|
| `progress.md` | Chronology of what's been done: date, commit, what+why, verification status |
| `roadmap.md` | Open plan: next steps, priority, dependencies, known risks |
| `architecture.md` | Big picture, layers, data flow |
| `modules.md` | What each function does, who calls it, locking rules |
| `flows.md` | Step-by-step behavior when something happens (message, chat, sleep, travel, prank, news) |
| `state-files.md` | `durum/` file formats, limits, summarization |
| `prompts.md` | Which prompt is used where, placeholders, max_tokens |
| `constants.md` | All constants and what they mean |
| `decisions.md` | Why things were built this way (decisions + rationale) |
| `development.md` | Adding a new agent/prompt/cycle/state file, pitfalls, checklist |
| `glossary.md` | Runtime vocabulary that stays Turkish on purpose (prompts, `durum/` fields, agent names) |
