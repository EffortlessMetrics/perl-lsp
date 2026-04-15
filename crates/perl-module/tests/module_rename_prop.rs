use perl_module::rename::{apply_module_rename_edits, plan_module_rename_edits};
use proptest::prelude::*;

fn module_name_strategy() -> impl Strategy<Value = String> {
    proptest::collection::vec("[A-Za-z_][A-Za-z0-9_]{0,7}", 1..5)
        .prop_map(|segments| segments.join("::"))
}

proptest! {
    #[test]
    fn rename_preserves_non_target_lines(old_module in module_name_strategy(), new_module in module_name_strategy()) {
        prop_assume!(old_module != new_module);

        let source = format!(
            "use {old_module};\nuse parent '{old_module}';\nmy $x = 1;\n"
        );

        let edits = plan_module_rename_edits(&source, &old_module, &new_module);
        let rewritten = apply_module_rename_edits(&source, &edits);

        let expected = format!(
            "use {new_module};\nuse parent '{new_module}';\nmy $x = 1;\n"
        );

        prop_assert_eq!(rewritten, expected);
    }

    #[test]
    fn planned_edits_are_line_bounded(source in "(?s).{0,512}", old_module in module_name_strategy(), new_module in module_name_strategy()) {
        prop_assume!(old_module != new_module);

        let line_count = source.lines().count();
        let edits = plan_module_rename_edits(&source, &old_module, &new_module);

        for edit in edits {
            prop_assert!(edit.line < line_count);
            prop_assert_eq!(edit.start_character, 0);
        }
    }
}
