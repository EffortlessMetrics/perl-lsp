use perl_module::import::{ModuleImportKind, parse_module_import_head};
use proptest::prelude::*;

fn module_name_strategy() -> impl Strategy<Value = String> {
    proptest::collection::vec("[A-Za-z_][A-Za-z0-9_]{0,8}", 1..5)
        .prop_map(|segments| segments.join("::"))
}

proptest! {
    #[test]
    fn prop_use_head_extracts_first_module_token_with_correct_range(
        leading_ws in "[ \\t]{0,6}",
        module in module_name_strategy(),
        trailing in "[; \\t\\)]{0,3}",
    ) {
        let line = format!("{leading_ws}use {module}{trailing}");

        let parsed = parse_module_import_head(&line);
        prop_assert!(parsed.is_some());
        if let Some(head) = parsed {
            prop_assert_eq!(head.kind, ModuleImportKind::Use);
            prop_assert_eq!(head.token, module.as_str());
            prop_assert_eq!(&line[head.token_start..head.token_end], head.token);
        }
    }

    #[test]
    fn prop_require_head_extracts_first_module_token_with_correct_range(
        leading_ws in "[ \\t]{0,6}",
        module in module_name_strategy(),
        trailing in "[; \\t\\)]{0,3}",
    ) {
        let line = format!("{leading_ws}require {module}{trailing}");

        let parsed = parse_module_import_head(&line);
        prop_assert!(parsed.is_some());
        if let Some(head) = parsed {
            prop_assert_eq!(head.kind, ModuleImportKind::Require);
            prop_assert_eq!(head.token, module.as_str());
            prop_assert_eq!(&line[head.token_start..head.token_end], head.token);
        }
    }

    #[test]
    fn prop_keyword_requires_word_boundary(module in module_name_strategy()) {
        let use_like = format!("user {module};");
        let require_like = format!("required {module};");

        prop_assert!(parse_module_import_head(&use_like).is_none());
        prop_assert!(parse_module_import_head(&require_like).is_none());
    }
}
