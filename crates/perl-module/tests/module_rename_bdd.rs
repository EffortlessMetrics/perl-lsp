use perl_module::rename::{apply_module_rename_edits, plan_module_rename_edits};

#[test]
fn given_use_statement_when_module_is_renamed_then_import_is_rewritten() {
    let source = "use My::Module;\n";

    let edits = plan_module_rename_edits(source, "My::Module", "My::Renamed");
    let rewritten = apply_module_rename_edits(source, &edits);

    assert_eq!(rewritten, "use My::Renamed;\n");
}

#[test]
fn given_parent_and_base_statements_when_module_is_renamed_then_all_references_are_rewritten() {
    let source = "use parent 'My::Module';\nuse base \"My::Module\";\nuse parent qw(My::Module Other::Base);\n";

    let edits = plan_module_rename_edits(source, "My::Module", "My::Renamed");
    let rewritten = apply_module_rename_edits(source, &edits);

    let expected = "use parent 'My::Renamed';\nuse base \"My::Renamed\";\nuse parent qw(My::Renamed Other::Base);\n";
    assert_eq!(rewritten, expected);
}

#[test]
fn given_non_import_lines_when_module_is_renamed_then_source_is_unchanged() {
    let source = "package My::Module;\nmy $s = 'My::Module';\n";

    let edits = plan_module_rename_edits(source, "My::Module", "My::Renamed");
    let rewritten = apply_module_rename_edits(source, &edits);

    assert!(edits.is_empty());
    assert_eq!(rewritten, source);
}

#[test]
fn given_legacy_separator_import_when_module_is_renamed_then_legacy_style_is_preserved() {
    let source = "use My'Module;\n";

    let edits = plan_module_rename_edits(source, "My::Module", "My::Renamed");
    let rewritten = apply_module_rename_edits(source, &edits);

    assert_eq!(rewritten, "use My'Renamed;\n");
}

#[test]
fn given_partial_legacy_module_name_when_module_is_renamed_then_line_is_unchanged() {
    let source = "use My'Module'Child;\n";

    let edits = plan_module_rename_edits(source, "My::Module", "My::Renamed");
    let rewritten = apply_module_rename_edits(source, &edits);

    assert!(edits.is_empty());
    assert_eq!(rewritten, source);
}
