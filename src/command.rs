// The slash command manager's body is split across src/command/*.rs (registration table /
// card commands / action commands / setting commands / shared helpers). Same reasoning
// and same method as the bot/*.rs split in main.rs: `include!` (not real `mod`) so
// `use super::*` and visibility never change — these files compile as if written inline
// in this module.
include!("command/registration.rs");
include!("command/cards.rs");
include!("command/actions.rs");
include!("command/settings.rs");
include!("command/remaining.rs");

#[cfg(test)]
mod test {
    use super::*;

    /// Verifies `definitions()` has no duplicate command names (Discord would reject a duplicate registration).
    #[test]
    fn table_names_are_unique() {
        let mut names: Vec<&str> = definitions().iter().map(|k| k.name).collect();
        let before = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), before, "duplicate name in the command table");
    }

    /// Verifies every command's `options` builder in `definitions()` runs without panicking.
    #[test]
    fn options_dont_panic() {
        for k in definitions() {
            let _ = (k.options)();
        }
    }

    /// Verifies `/dusunme`'s slash-option choice values exactly match what `ThinkingMode::from_arg` recognizes.
    #[test]
    fn thinking_mode_options_match_from_arg() {
        // the slash option values (goster/gizle/sessiz/kapat — Discord-facing, stay
        // Turkish) must exactly match the strings ThinkingMode::from_arg recognizes,
        // or the selected mode is silently never applied
        for value in ["goster", "gizle", "sessiz", "kapat"] {
            assert!(
                ThinkingMode::from_arg(value).is_some(),
                "from_arg does not recognize {value}"
            );
        }
    }
}
