use perl_module_reference::extract_module_reference;
use proptest::prelude::*;

fn module_name_strategy() -> impl Strategy<Value = String> {
    proptest::collection::vec("[A-Za-z_][A-Za-z0-9_]{0,7}", 1..5)
        .prop_map(|segments| segments.join("::"))
}

proptest! {
    #[test]
    fn extracts_canonical_module_for_all_cursor_positions_in_use(module in module_name_strategy()) {
        let line = format!("use {module};");
        let start = 4usize;
        let end = start + module.len();

        for cursor in start..=end {
            prop_assert_eq!(extract_module_reference(&line, cursor), Some(module.clone()));
        }
    }

    #[test]
    fn extracts_canonical_module_for_legacy_separator_inputs(module in module_name_strategy()) {
        let legacy = module.replace("::", "'");
        let line = format!("use {legacy};");
        let start = 4usize;
        let end = start + legacy.len();

        for cursor in start..=end {
            prop_assert_eq!(extract_module_reference(&line, cursor), Some(module.clone()));
        }
    }

    #[test]
    fn cursor_outside_module_token_does_not_match(module in module_name_strategy(), prefix in "[ a-zA-Z0-9_]{0,16}") {
        let line = format!("{prefix} use {module};");
        let module_start = prefix.len() + 5;
        prop_assume!(module_start > 0);

        let before_token = module_start - 1;
        prop_assert_eq!(extract_module_reference(&line, before_token), None);
    }
}
