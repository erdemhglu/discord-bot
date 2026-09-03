impl Bot {
    // !uyan and the settings panel: doesn't delete the plan (deleting it would just let
    // it get rebuilt a minute later and put the bot back to sleep) — instead stays
    // "forced awake" until the planned sleep window would have ended
    /// Input: `&self`. Output: none (sets `state.forced_awake_until` to the end of the
    /// current sleep window, or 6h from now if none is active). Uses: `self.state()`,
    /// `now_unix`. Used by: `Bot::cmd_wake` (`command/actions.rs`), `Handler::setting_button`
    /// (`handler_buttons.rs`).
    pub fn wake(&self) {
        let mut state = self.state();
        let now = now_unix();
        let end = state
            .plans
            .iter()
            .filter(|plan| plan.start <= now && now < plan.end)
            .map(|plan| plan.end)
            .max()
            .unwrap_or(now + 6 * 3600);
        state.forced_awake_until = end;
    }

    // !uyu [saat] and the settings panel: a temporary sleep plan for testing.
    // `hours` is how long from now the plan should last.
    /// Input: `&self`; `hours: i64`. Output: none (clears any forced-awake override, adds
    /// a `sleep::Plan` running from now for `hours`). Uses: `self.state()`, `now_unix`,
    /// `sleep::Plan`. Used by: `Bot::cmd_sleep` (`command/actions.rs`),
    /// `Handler::setting_button` (`handler_buttons.rs`).
    pub fn put_to_sleep(&self, hours: i64) {
        let mut state = self.state();
        let now = now_unix();
        state.forced_awake_until = 0;
        state.plans.push(sleep::Plan {
            day: -1,
            insomnia_start: None,
            start: now,
            end: now + hours * 3600,
        });
    }

    // !debug ac|kapat (toggles if empty); persisted in durum/debug.md. Returns the new state.
    /// Input: `&self`; `arg: &str` — `"ac"`/`"kapat"`/variants, or `""` to toggle. Output:
    /// `bool` — the new debug state. Uses: `self.state()`, `memory::write`. Used by:
    /// `Bot::cmd_debug` (`command/settings.rs`), `Handler::setting_button`
    /// (`handler_buttons.rs`).
    pub fn set_debug(&self, arg: &str) -> bool {
        let mut state = self.state();
        let new_value = match arg.trim().to_lowercase().as_str() {
            "aç" | "ac" | "açık" | "acik" | "on" => true,
            "kapat" | "kapalı" | "kapali" | "off" => false,
            _ => !state.debug,
        };
        state.debug = new_value;
        memory::write("debug.md", if new_value { "acik" } else { "kapali" });
        new_value
    }

    // whether `id` is in openrouter's model catalog (queries the live /models list)
    /// Input: `&self`; `id: &str` — a model id to check. Output: `bool` — `true` if `id` is
    /// listed, or if the catalog request itself failed (fails open so an unrelated network
    /// hiccup doesn't block `/model`). Uses: `self.http`. Used by: `Bot::cmd_model`
    /// (`command/settings.rs`), the only caller.
    async fn model_exists(&self, id: &str) -> bool {
        #[derive(Deserialize)]
        struct ModelList {
            data: Vec<ModelEntry>,
        }
        #[derive(Deserialize)]
        struct ModelEntry {
            id: String,
        }
        match self
            .http
            .get("https://openrouter.ai/api/v1/models")
            .send()
            .await
        {
            Ok(resp) => resp
                .json::<ModelList>()
                .await
                .map(|list| list.data.iter().any(|m| m.id == id))
                .unwrap_or(false),
            Err(_) => true, // if the list can't be fetched, don't block on it
        }
    }
}
