use perl_module::rename::{apply_module_rename_edits, plan_module_rename_edits};
use proptest::prelude::*;

fn module_name_strategy() -> impl Strategy<Value = String> {
    proptest::collection::vec("[A-Za-z_][A-Za-z0-9_]{0,7}", 1..5)
        .prop_map(|segments| segments.join("::"))
}

fn identifier_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z_][A-Za-z0-9_]{0,10}".prop_map(|segment| segment)
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

    #[test]
    fn rename_is_noop_when_module_not_present(
        old_module in module_name_strategy(),
        new_module in module_name_strategy(),
        left in identifier_strategy(),
        right in identifier_strategy(),
    ) {
        prop_assume!(old_module != new_module);
        prop_assume!(left != old_module && left != new_module);
        prop_assume!(right != old_module && right != new_module);

        let source = format!(
            "package {left};\nmy $value = {right}::helper();\n# extends {right}\n"
        );

        let edits = plan_module_rename_edits(&source, &old_module, &new_module);
        let rewritten = apply_module_rename_edits(&source, &edits);

        prop_assert!(edits.is_empty());
        prop_assert_eq!(rewritten, source);
    }

    #[test]
    fn applying_same_rename_plan_is_idempotent(
        old_module in module_name_strategy(),
        new_module in module_name_strategy(),
        trailer in "[a-z]{0,8}",
    ) {
        prop_assume!(old_module != new_module);

        let source = format!(
            "use {old_module};\n{old_module}::work();\npackage {old_module};\n# {trailer}\n"
        );

        let edits = plan_module_rename_edits(&source, &old_module, &new_module);
        let rewritten_once = apply_module_rename_edits(&source, &edits);
        let rewritten_twice = apply_module_rename_edits(&rewritten_once, &edits);

        prop_assert_eq!(rewritten_once, rewritten_twice);
    }
}
