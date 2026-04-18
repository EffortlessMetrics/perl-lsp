use perl_module::rename::{
    apply_module_rename_edits, line_references_qualified_call, plan_module_rename_edits,
    replace_module_name_prefix,
};

// ──────────────────────────────────────────────────────────────
// Import rewriting
// ──────────────────────────────────────────────────────────────

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

// ──────────────────────────────────────────────────────────────
// Qualified call false-positive prevention (#4423)
// ──────────────────────────────────────────────────────────────

#[test]
fn given_qualified_call_in_comment_when_checked_then_not_matched() {
    assert!(!line_references_qualified_call("# My::Module::func()", "My::Module"));
}

#[test]
fn given_qualified_call_in_single_quoted_string_when_checked_then_not_matched() {
    assert!(!line_references_qualified_call("my $s = 'My::Module::func';", "My::Module"));
}

#[test]
fn given_qualified_call_in_double_quoted_string_when_checked_then_not_matched() {
    assert!(!line_references_qualified_call("my $s = \"My::Module::func\";", "My::Module"));
}

#[test]
fn given_package_declaration_line_when_checked_for_qualified_call_then_not_matched() {
    assert!(!line_references_qualified_call("package Foo::Bar::Baz;", "Foo::Bar"));
}

#[test]
fn given_use_import_line_when_checked_for_qualified_call_then_not_matched() {
    assert!(!line_references_qualified_call("use Foo::Bar::Baz;", "Foo::Bar"));
    assert!(!line_references_qualified_call("require Foo::Bar::Baz;", "Foo::Bar"));
}

#[test]
fn given_qualified_call_in_code_when_checked_then_matched() {
    assert!(line_references_qualified_call("My::Module::func();", "My::Module"));
}

#[test]
fn given_qualified_call_after_string_when_checked_then_matched() {
    assert!(line_references_qualified_call("my $s = 'text'; My::Module::func();", "My::Module"));
}

#[test]
fn given_qualified_call_with_escaped_quote_when_checked_then_context_tracked_correctly() {
    assert!(!line_references_qualified_call("my $s = 'it\\'s My::Module::func';", "My::Module"));
}

// ──────────────────────────────────────────────────────────────
// replace_module_name_prefix false-positive prevention (#4423)
// ──────────────────────────────────────────────────────────────

#[test]
fn given_qualified_call_in_code_when_replaced_then_prefix_updated() {
    let result = replace_module_name_prefix("My::Module::func();", "My::Module", "My::Renamed");
    assert_eq!(result, "My::Renamed::func();");
}

#[test]
fn given_qualified_call_in_string_when_replaced_then_string_unchanged() {
    let result =
        replace_module_name_prefix("my $s = 'My::Module::func';", "My::Module", "My::Renamed");
    assert_eq!(result, "my $s = 'My::Module::func';");
}

#[test]
fn given_qualified_call_in_comment_when_replaced_then_comment_unchanged() {
    let result = replace_module_name_prefix("# My::Module::func()", "My::Module", "My::Renamed");
    assert_eq!(result, "# My::Module::func()");
}

#[test]
fn given_package_declaration_when_replaced_then_line_unchanged() {
    let result = replace_module_name_prefix("package Foo::Bar::Baz;", "Foo::Bar", "New::Name");
    assert_eq!(result, "package Foo::Bar::Baz;");
}

// ──────────────────────────────────────────────────────────────
// End-to-end: qualified calls in plan_module_rename_edits
// ──────────────────────────────────────────────────────────────

#[test]
fn given_qualified_call_in_source_when_module_is_renamed_then_call_is_rewritten() {
    let source = "My::Module::func();\n";

    let edits = plan_module_rename_edits(source, "My::Module", "My::Renamed");
    let rewritten = apply_module_rename_edits(source, &edits);

    assert_eq!(rewritten, "My::Renamed::func();\n");
}

#[test]
fn given_comment_with_qualified_name_when_module_is_renamed_then_comment_is_unchanged() {
    let source = "# calls My::Module::func()\nuse My::Module;\n";

    let edits = plan_module_rename_edits(source, "My::Module", "My::Renamed");
    let rewritten = apply_module_rename_edits(source, &edits);

    assert_eq!(rewritten, "# calls My::Module::func()\nuse My::Renamed;\n");
}

#[test]
fn given_string_with_qualified_name_when_module_is_renamed_then_string_is_unchanged() {
    let source = "my $s = 'My::Module::func';\nuse My::Module;\n";

    let edits = plan_module_rename_edits(source, "My::Module", "My::Renamed");
    let rewritten = apply_module_rename_edits(source, &edits);

    assert_eq!(rewritten, "my $s = 'My::Module::func';\nuse My::Renamed;\n");
}
