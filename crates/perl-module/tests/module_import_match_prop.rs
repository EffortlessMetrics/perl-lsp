use perl_module::import_match::line_references_module_import;
use proptest::prelude::*;

fn module_name_strategy() -> impl Strategy<Value = String> {
    proptest::collection::vec("[A-Za-z_][A-Za-z0-9_]{0,8}", 1..5)
        .prop_map(|segments| segments.join("::"))
}

proptest! {
    #[test]
    fn prop_direct_use_and_require_match_exact_module(module in module_name_strategy()) {
        let use_line = format!("use {module};");
        let require_line = format!("require {module};");

        prop_assert!(line_references_module_import(&use_line, &module));
        prop_assert!(line_references_module_import(&require_line, &module));
    }

    #[test]
    fn prop_partial_direct_import_never_matches(module in module_name_strategy(), suffix in "[A-Za-z_][A-Za-z0-9_]{0,4}") {
        let line = format!("use {module}{suffix};");
        prop_assert!(!line_references_module_import(&line, &module));
    }

    #[test]
    fn prop_parent_qw_matches_target_module(module in module_name_strategy(), other in module_name_strategy()) {
        let line = format!("use parent qw({module} {other});");
        prop_assert!(line_references_module_import(&line, &module));
    }
}
