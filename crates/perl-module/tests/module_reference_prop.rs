use perl_module::reference::{
    ModuleReferenceKind, extract_module_reference, extract_module_reference_extended,
    find_module_reference, find_module_reference_extended,
};
use proptest::prelude::*;

fn module_name_strategy() -> impl Strategy<Value = String> {
    proptest::collection::vec("[A-Za-z_][A-Za-z0-9_]{0,7}", 1..5)
        .prop_map(|segments| segments.join("::"))
}

fn statement_prefix_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec("[ \t]{0,3}", 0..4).prop_map(|parts| parts.concat())
}

fn parent_base_module_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("Carp".to_string()),
        module_name_strategy().prop_filter("module must contain a package separator", |module| {
            module.contains("::")
        }),
    ]
}

proptest! {
    #[test]
    fn extracts_canonical_module_for_all_cursor_positions_in_use(module in module_name_strategy()) {
        let line = format!("use {module};");
        let start = 4usize;
        let end = start + module.len();

        for cursor in start..=end {
            prop_assert_eq!(extract_module_reference(&line, cursor), Some(module.clone()));

            let reference = find_module_reference(&line, cursor);
            prop_assert_eq!(reference.map(|item| item.kind), Some(ModuleReferenceKind::Use));
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
    fn extracts_require_module_for_all_cursor_positions_in_require(module in module_name_strategy()) {
        let line = format!("require {module};");
        let start = "require ".len();
        let end = start + module.len();

        for cursor in start..=end {
            let reference = find_module_reference(&line, cursor);
            prop_assert_eq!(reference.map(|item| item.kind), Some(ModuleReferenceKind::Require));
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

    #[test]
    fn extended_reference_finds_parent_and_base_module_tokens_across_cursor_positions(
        module in parent_base_module_strategy(),
        statement in prop_oneof![Just("parent"), Just("base")],
        quoting in prop_oneof![Just("single"), Just("double"), Just("qw")],
        prefix in statement_prefix_strategy(),
    ) {
        let body = match quoting {
            "single" => format!("'{module}'"),
            "double" => format!("\"{module}\""),
            "qw" => format!("qw({module})"),
            _ => unreachable!(),
        };
        let text = format!("{prefix}use {statement} {body};");
        let start = text.rfind(&module).unwrap_or(0);
        let end = start + module.len();

        for cursor in start..=end {
            let reference = find_module_reference_extended(&text, cursor);
            prop_assert_eq!(reference.map(|item| item.kind), Some(ModuleReferenceKind::Use));
            prop_assert_eq!(extract_module_reference_extended(&text, cursor), Some(module.clone()));
        }
    }

    #[test]
    fn extended_reference_uses_line_local_offsets_for_parent_and_base(
        module in parent_base_module_strategy(),
        statement in prop_oneof![Just("parent"), Just("base")],
        leading_lines in prop::collection::vec(".*", 0..3),
    ) {
        let prefix = if leading_lines.is_empty() {
            String::new()
        } else {
            format!("{}\n", leading_lines.join("\n"))
        };
        let target_line = format!("use {statement} '{module}';");
        let text = format!("{prefix}{target_line}\n1;");
        let start = prefix.len() + target_line.rfind(&module).unwrap_or(0);
        let end = start + module.len();

        for cursor in start..=end {
            prop_assert_eq!(extract_module_reference_extended(&text, cursor), Some(module.clone()));
        }
    }

    #[test]
    fn direct_extractor_rejects_non_direct_parent_and_base_forms(
        module in parent_base_module_strategy(),
        statement in prop_oneof![Just("parent"), Just("base")],
    ) {
        let text = format!("use {statement} '{module}';");
        let cursor = text.rfind(&module).unwrap_or(0);

        prop_assert_eq!(extract_module_reference(&text, cursor), None);
        prop_assert_eq!(extract_module_reference_extended(&text, cursor), Some(module));
    }
}
