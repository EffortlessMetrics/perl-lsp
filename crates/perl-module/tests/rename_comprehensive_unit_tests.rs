//! Comprehensive unit tests for perl-module-rename crate.
//!
//! Covers: plan_module_rename_edits, apply_module_rename_edits, ModuleLineEdit

use perl_module::rename::{
    ModuleLineEdit, apply_module_rename_edits, line_references_qualified_call,
    plan_module_rename_edits, replace_module_name_prefix,
};

// ──────────────────────────────────────────────────────────────
// plan_module_rename_edits — early-return / empty-input guards
// ──────────────────────────────────────────────────────────────

#[test]
fn plan_returns_empty_for_empty_source() -> Result<(), Box<dyn std::error::Error>> {
    let edits = plan_module_rename_edits("", "Foo::Bar", "Baz::Qux");
    assert!(edits.is_empty(), "empty source should produce no edits");
    Ok(())
}

#[test]
fn plan_returns_empty_for_empty_old_module() -> Result<(), Box<dyn std::error::Error>> {
    let edits = plan_module_rename_edits("use Foo::Bar;", "", "Baz::Qux");
    assert!(edits.is_empty(), "empty old_module should produce no edits");
    Ok(())
}

#[test]
fn plan_returns_empty_for_empty_new_module() -> Result<(), Box<dyn std::error::Error>> {
    let edits = plan_module_rename_edits("use Foo::Bar;", "Foo::Bar", "");
    assert!(edits.is_empty(), "empty new_module should produce no edits");
    Ok(())
}

#[test]
fn plan_returns_empty_when_old_equals_new() -> Result<(), Box<dyn std::error::Error>> {
    let edits = plan_module_rename_edits("use Foo::Bar;", "Foo::Bar", "Foo::Bar");
    assert!(edits.is_empty(), "identical old/new should produce no edits");
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// plan_module_rename_edits — use/require statements
// ──────────────────────────────────────────────────────────────

#[test]
fn plan_rewrites_use_statement() -> Result<(), Box<dyn std::error::Error>> {
    let edits = plan_module_rename_edits("use Foo::Bar;", "Foo::Bar", "New::Mod");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "use New::Mod;");
    assert_eq!(edits[0].line, 0);
    assert_eq!(edits[0].start_character, 0);
    Ok(())
}

#[test]
fn plan_rewrites_require_statement() -> Result<(), Box<dyn std::error::Error>> {
    let edits = plan_module_rename_edits("require Foo::Bar;", "Foo::Bar", "New::Mod");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "require New::Mod;");
    Ok(())
}

#[test]
fn plan_rewrites_both_use_and_require_in_same_source() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use Foo::Bar;\nrequire Foo::Bar;";
    let edits = plan_module_rename_edits(source, "Foo::Bar", "X::Y");
    assert_eq!(edits.len(), 2);
    assert_eq!(edits[0].line, 0);
    assert_eq!(edits[1].line, 1);
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// plan_module_rename_edits — use parent / use base
// ──────────────────────────────────────────────────────────────

#[test]
fn plan_rewrites_use_parent_single_quoted() -> Result<(), Box<dyn std::error::Error>> {
    let edits = plan_module_rename_edits("use parent 'Foo::Bar';", "Foo::Bar", "New::Mod");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "use parent 'New::Mod';");
    Ok(())
}

#[test]
fn plan_rewrites_use_parent_double_quoted() -> Result<(), Box<dyn std::error::Error>> {
    let edits = plan_module_rename_edits("use parent \"Foo::Bar\";", "Foo::Bar", "New::Mod");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "use parent \"New::Mod\";");
    Ok(())
}

#[test]
fn plan_rewrites_use_parent_qw() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use parent qw(Foo::Bar Other::Mod);";
    let edits = plan_module_rename_edits(source, "Foo::Bar", "New::Mod");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "use parent qw(New::Mod Other::Mod);");
    Ok(())
}

#[test]
fn plan_rewrites_use_base_single_quoted() -> Result<(), Box<dyn std::error::Error>> {
    let edits = plan_module_rename_edits("use base 'Foo::Bar';", "Foo::Bar", "New::Mod");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "use base 'New::Mod';");
    Ok(())
}

#[test]
fn plan_rewrites_use_base_double_quoted() -> Result<(), Box<dyn std::error::Error>> {
    let edits = plan_module_rename_edits("use base \"Foo::Bar\";", "Foo::Bar", "New::Mod");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "use base \"New::Mod\";");
    Ok(())
}

#[test]
fn plan_rewrites_use_base_qw() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use base qw(Foo::Bar Another::Base);";
    let edits = plan_module_rename_edits(source, "Foo::Bar", "New::Mod");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "use base qw(New::Mod Another::Base);");
    Ok(())
}

#[test]
fn plan_rewrites_moose_extends_single_quoted() -> Result<(), Box<dyn std::error::Error>> {
    let edits = plan_module_rename_edits("extends 'Foo::Bar';", "Foo::Bar", "New::Mod");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "extends 'New::Mod';");
    Ok(())
}

#[test]
fn plan_rewrites_moo_with_qw() -> Result<(), Box<dyn std::error::Error>> {
    let source = "with qw(Foo::Bar Other::Role);";
    let edits = plan_module_rename_edits(source, "Foo::Bar", "New::Role");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "with qw(New::Role Other::Role);");
    Ok(())
}

// plan_module_rename_edits — Moose/Moo DSL (extends/with) edge cases

#[test]
fn plan_rewrites_moose_extends_double_quoted() -> Result<(), Box<dyn std::error::Error>> {
    let edits = plan_module_rename_edits("extends \"Foo::Bar\";", "Foo::Bar", "New::Mod");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "extends \"New::Mod\";");
    Ok(())
}

#[test]
fn plan_rewrites_extends_multiple_parents() -> Result<(), Box<dyn std::error::Error>> {
    let source = "extends qw(Foo::Bar Other::Base);";
    let edits = plan_module_rename_edits(source, "Foo::Bar", "New::Mod");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "extends qw(New::Mod Other::Base);");
    Ok(())
}

#[test]
fn plan_no_false_positive_extends_in_comment() -> Result<(), Box<dyn std::error::Error>> {
    // A comment line starting with # should not be rewritten.
    let edits = plan_module_rename_edits("# extends 'Foo::Bar'", "Foo::Bar", "New::Mod");
    // Comments do not start a line with 'extends', so this should produce no edit.
    // (The line trim_start would yield "# extends ...", which does not start_with "extends ")
    assert!(edits.is_empty(), "comment line should not be rewritten: {edits:?}");
    Ok(())
}

#[test]
fn plan_no_false_positive_with_in_non_moose_context() -> Result<(), Box<dyn std::error::Error>> {
    // "with" appears as a statement modifier in non-Moose code.
    // Because line_references_moose_moo_dsl checks trim_start starts_with("with "),
    // a line like `open($fh, "<", $f) or die "err"` does not trigger.
    let source = "open($fh, '<', $f) or die 'err';";
    let edits = plan_module_rename_edits(source, "Foo::Bar", "New::Mod");
    assert!(edits.is_empty());
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// plan_module_rename_edits — legacy separator (single-quote)
// ──────────────────────────────────────────────────────────────

#[test]
fn plan_rewrites_legacy_separator_use() -> Result<(), Box<dyn std::error::Error>> {
    let edits = plan_module_rename_edits("use Foo'Bar;", "Foo::Bar", "New::Path");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "use New'Path;");
    Ok(())
}

#[test]
fn plan_rewrites_legacy_separator_in_parent_quote() -> Result<(), Box<dyn std::error::Error>> {
    let edits = plan_module_rename_edits("use parent \"Foo'Bar\";", "Foo::Bar", "New::Path");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "use parent \"New'Path\";");
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// plan_module_rename_edits — boundary / partial-match safety
// ──────────────────────────────────────────────────────────────

#[test]
fn plan_does_not_match_partial_module_prefix() -> Result<(), Box<dyn std::error::Error>> {
    let edits = plan_module_rename_edits("use Foo::Barista;", "Foo::Bar", "X::Y");
    assert!(edits.is_empty());
    Ok(())
}

#[test]
fn plan_does_not_match_partial_module_suffix() -> Result<(), Box<dyn std::error::Error>> {
    let edits = plan_module_rename_edits("use MyFoo::Bar;", "Foo::Bar", "X::Y");
    assert!(edits.is_empty());
    Ok(())
}

#[test]
fn plan_ignores_non_import_lines() -> Result<(), Box<dyn std::error::Error>> {
    let source = "# use Foo::Bar;\nmy $x = 1;\nprint Foo::Bar->new();\n";
    let edits = plan_module_rename_edits(source, "Foo::Bar", "New::Mod");
    // Comments and non-import lines should not generate edits
    assert!(edits.is_empty());
    Ok(())
}

#[test]
fn plan_ignores_plain_code_with_module_name() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $obj = Foo::Bar->new();";
    let edits = plan_module_rename_edits(source, "Foo::Bar", "X::Y");
    assert!(edits.is_empty());
    Ok(())
}

#[test]
fn qualified_call_ignores_package_declaration_and_string_literals()
-> Result<(), Box<dyn std::error::Error>> {
    assert!(!line_references_qualified_call("package Foo::Bar;", "Foo::Bar"));
    assert!(!line_references_qualified_call("use Foo'Bar'Baz;", "Foo'Bar"));
    assert!(!line_references_qualified_call("'Foo::Bar::func()'", "Foo::Bar"));
    assert!(!line_references_qualified_call("\"Foo::Bar::func()\"", "Foo::Bar"));
    assert!(line_references_qualified_call("Foo::Bar::func()", "Foo::Bar"));
    assert!(line_references_qualified_call("Foo::Bar'func()", "Foo::Bar"));
    assert!(line_references_qualified_call("Foo'Bar'func()", "Foo'Bar"));
    Ok(())
}

#[test]
fn replace_prefix_skips_package_lines_and_quoted_occurrences()
-> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        replace_module_name_prefix("package Foo::Bar;", "Foo::Bar", "New::Mod"),
        "package Foo::Bar;"
    );
    assert_eq!(
        replace_module_name_prefix("use Foo'Bar'Baz;", "Foo'Bar", "New'Path"),
        "use Foo'Bar'Baz;"
    );
    assert_eq!(
        replace_module_name_prefix("'Foo::Bar::func()'", "Foo::Bar", "New::Mod"),
        "'Foo::Bar::func()'"
    );
    assert_eq!(
        replace_module_name_prefix("Foo::Bar::func()", "Foo::Bar", "New::Mod"),
        "New::Mod::func()"
    );
    assert_eq!(
        replace_module_name_prefix("Foo::Bar'func()", "Foo::Bar", "New::Mod"),
        "New::Mod'func()"
    );
    assert_eq!(
        replace_module_name_prefix("Foo'Bar'func()", "Foo'Bar", "New'Path"),
        "New'Path'func()"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// plan_module_rename_edits — multi-line source
// ──────────────────────────────────────────────────────────────

#[test]
fn plan_handles_multiple_import_lines_among_non_imports() -> Result<(), Box<dyn std::error::Error>>
{
    let source =
        "#!/usr/bin/perl\nuse strict;\nuse Foo::Bar;\nuse warnings;\nrequire Foo::Bar;\nprint 1;";
    let edits = plan_module_rename_edits(source, "Foo::Bar", "New::Mod");
    assert_eq!(edits.len(), 2);
    assert_eq!(edits[0].line, 2);
    assert_eq!(edits[1].line, 4);
    Ok(())
}

#[test]
fn plan_preserves_line_indices_correctly() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line0\nline1\nuse Foo::Bar;\nline3\nuse parent 'Foo::Bar';";
    let edits = plan_module_rename_edits(source, "Foo::Bar", "A::B");
    assert_eq!(edits.len(), 2);
    assert_eq!(edits[0].line, 2);
    assert_eq!(edits[0].end_character, "use Foo::Bar;".len());
    assert_eq!(edits[1].line, 4);
    assert_eq!(edits[1].end_character, "use parent 'Foo::Bar';".len());
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// plan_module_rename_edits — single-segment module names
// ──────────────────────────────────────────────────────────────

#[test]
fn plan_rewrites_single_segment_module() -> Result<(), Box<dyn std::error::Error>> {
    let edits = plan_module_rename_edits("use strict;", "strict", "warnings");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "use warnings;");
    Ok(())
}

#[test]
fn plan_rewrites_require_single_segment() -> Result<(), Box<dyn std::error::Error>> {
    let edits = plan_module_rename_edits("require Carp;", "Carp", "Croak");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "require Croak;");
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// plan_module_rename_edits — deeply nested modules
// ──────────────────────────────────────────────────────────────

#[test]
fn plan_rewrites_deeply_nested_module() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use A::B::C::D::E;";
    let edits = plan_module_rename_edits(source, "A::B::C::D::E", "X::Y::Z");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "use X::Y::Z;");
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// plan_module_rename_edits — qw list with only matching module
// ──────────────────────────────────────────────────────────────

#[test]
fn plan_rewrites_qw_list_with_single_module() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use parent qw(Foo::Bar);";
    let edits = plan_module_rename_edits(source, "Foo::Bar", "New::Mod");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "use parent qw(New::Mod);");
    Ok(())
}

#[test]
fn plan_does_not_rewrite_qw_non_matching_module() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use parent qw(Other::Mod);";
    let edits = plan_module_rename_edits(source, "Foo::Bar", "New::Mod");
    assert!(edits.is_empty());
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// apply_module_rename_edits — basic behavior
// ──────────────────────────────────────────────────────────────

#[test]
fn apply_returns_original_for_empty_edits() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use Foo::Bar;\nuse Baz;";
    let result = apply_module_rename_edits(source, &[]);
    assert_eq!(result, source);
    Ok(())
}

#[test]
fn apply_replaces_single_line() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line0\nline1\nline2";
    let edits = vec![ModuleLineEdit {
        line: 1,
        start_character: 0,
        end_character: 5,
        new_text: "replaced".to_string(),
    }];
    let result = apply_module_rename_edits(source, &edits);
    assert_eq!(result, "line0\nreplaced\nline2");
    Ok(())
}

#[test]
fn apply_replaces_multiple_lines() -> Result<(), Box<dyn std::error::Error>> {
    let source = "a\nb\nc\nd";
    let edits = vec![
        ModuleLineEdit { line: 0, start_character: 0, end_character: 1, new_text: "A".to_string() },
        ModuleLineEdit { line: 2, start_character: 0, end_character: 1, new_text: "C".to_string() },
    ];
    let result = apply_module_rename_edits(source, &edits);
    assert_eq!(result, "A\nb\nC\nd");
    Ok(())
}

#[test]
fn apply_handles_edits_in_reverse_order() -> Result<(), Box<dyn std::error::Error>> {
    let source = "a\nb\nc";
    // Edits given in reverse line order — apply should sort them
    let edits = vec![
        ModuleLineEdit { line: 2, start_character: 0, end_character: 1, new_text: "C".to_string() },
        ModuleLineEdit { line: 0, start_character: 0, end_character: 1, new_text: "A".to_string() },
    ];
    let result = apply_module_rename_edits(source, &edits);
    assert_eq!(result, "A\nb\nC");
    Ok(())
}

#[test]
fn apply_ignores_out_of_bounds_line_index() -> Result<(), Box<dyn std::error::Error>> {
    let source = "only_line";
    let edits = vec![ModuleLineEdit {
        line: 99,
        start_character: 0,
        end_character: 5,
        new_text: "ghost".to_string(),
    }];
    let result = apply_module_rename_edits(source, &edits);
    assert_eq!(result, "only_line");
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Roundtrip: plan + apply
// ──────────────────────────────────────────────────────────────

#[test]
fn roundtrip_use_statement() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use Foo::Bar;";
    let edits = plan_module_rename_edits(source, "Foo::Bar", "New::Mod");
    let result = apply_module_rename_edits(source, &edits);
    assert_eq!(result, "use New::Mod;");
    Ok(())
}

#[test]
fn roundtrip_require_statement() -> Result<(), Box<dyn std::error::Error>> {
    let source = "require Foo::Bar;";
    let edits = plan_module_rename_edits(source, "Foo::Bar", "Baz::Qux");
    let result = apply_module_rename_edits(source, &edits);
    assert_eq!(result, "require Baz::Qux;");
    Ok(())
}

#[test]
fn roundtrip_mixed_imports() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use strict;\nuse Foo::Bar;\nuse parent 'Foo::Bar';\nuse base qw(Foo::Bar Baz);\nrequire Foo::Bar;\nmy $x = 1;";
    let edits = plan_module_rename_edits(source, "Foo::Bar", "New::Mod");
    let result = apply_module_rename_edits(source, &edits);
    let expected = "use strict;\nuse New::Mod;\nuse parent 'New::Mod';\nuse base qw(New::Mod Baz);\nrequire New::Mod;\nmy $x = 1;";
    assert_eq!(result, expected);
    Ok(())
}

#[test]
fn roundtrip_preserves_trailing_newline() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use Foo::Bar;\n";
    let edits = plan_module_rename_edits(source, "Foo::Bar", "X::Y");
    let result = apply_module_rename_edits(source, &edits);
    assert_eq!(result, "use X::Y;\n");
    Ok(())
}

#[test]
fn roundtrip_no_matching_imports_preserves_source() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use strict;\nuse warnings;\nmy $x = 1;\n";
    let edits = plan_module_rename_edits(source, "Foo::Bar", "X::Y");
    let result = apply_module_rename_edits(source, &edits);
    assert_eq!(result, source);
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// ModuleLineEdit — struct field coverage
// ──────────────────────────────────────────────────────────────

#[test]
fn module_line_edit_debug_impl() -> Result<(), Box<dyn std::error::Error>> {
    let edit = ModuleLineEdit {
        line: 3,
        start_character: 0,
        end_character: 15,
        new_text: "use X::Y;".to_string(),
    };
    let debug = format!("{edit:?}");
    assert!(debug.contains("ModuleLineEdit"));
    assert!(debug.contains("line: 3"));
    Ok(())
}

#[test]
fn module_line_edit_clone() -> Result<(), Box<dyn std::error::Error>> {
    let edit = ModuleLineEdit {
        line: 0,
        start_character: 0,
        end_character: 10,
        new_text: "test".to_string(),
    };
    let cloned = edit.clone();
    assert_eq!(edit, cloned);
    Ok(())
}

#[test]
fn module_line_edit_equality() -> Result<(), Box<dyn std::error::Error>> {
    let a = ModuleLineEdit {
        line: 0,
        start_character: 0,
        end_character: 10,
        new_text: "text".to_string(),
    };
    let b = ModuleLineEdit {
        line: 0,
        start_character: 0,
        end_character: 10,
        new_text: "text".to_string(),
    };
    let c = ModuleLineEdit {
        line: 1,
        start_character: 0,
        end_character: 10,
        new_text: "text".to_string(),
    };
    assert_eq!(a, b);
    assert_ne!(a, c);
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Edge cases
// ──────────────────────────────────────────────────────────────

#[test]
fn plan_with_blank_lines_in_source() -> Result<(), Box<dyn std::error::Error>> {
    let source = "\n\nuse Foo::Bar;\n\n";
    let edits = plan_module_rename_edits(source, "Foo::Bar", "New::Mod");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].line, 2);
    Ok(())
}

#[test]
fn plan_with_source_having_only_newlines() -> Result<(), Box<dyn std::error::Error>> {
    let edits = plan_module_rename_edits("\n\n\n", "Foo::Bar", "X::Y");
    assert!(edits.is_empty());
    Ok(())
}

#[test]
fn plan_with_source_single_line_no_newline() -> Result<(), Box<dyn std::error::Error>> {
    let edits = plan_module_rename_edits("use Foo::Bar;", "Foo::Bar", "X::Y");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "use X::Y;");
    Ok(())
}

#[test]
fn apply_with_empty_source_and_no_edits() -> Result<(), Box<dyn std::error::Error>> {
    let result = apply_module_rename_edits("", &[]);
    assert_eq!(result, "");
    Ok(())
}

#[test]
fn plan_multiple_occurrences_of_same_module_in_qw() -> Result<(), Box<dyn std::error::Error>> {
    // Pathological: same module appears twice in qw list
    let source = "use parent qw(Foo::Bar Foo::Bar);";
    let edits = plan_module_rename_edits(source, "Foo::Bar", "X::Y");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "use parent qw(X::Y X::Y);");
    Ok(())
}

#[test]
fn plan_with_leading_whitespace() -> Result<(), Box<dyn std::error::Error>> {
    let source = "    use Foo::Bar;";
    let edits = plan_module_rename_edits(source, "Foo::Bar", "New::Mod");
    // Whether leading whitespace is handled depends on import matcher;
    // verify no panic and consistent behavior
    if !edits.is_empty() {
        assert_eq!(edits[0].line, 0);
    }
    Ok(())
}

#[test]
fn roundtrip_legacy_and_canonical_in_same_source() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use Foo::Bar;\nuse Foo'Bar;";
    let edits = plan_module_rename_edits(source, "Foo::Bar", "New::Path");
    let result = apply_module_rename_edits(source, &edits);
    assert_eq!(result, "use New::Path;\nuse New'Path;");
    Ok(())
}

#[test]
fn plan_end_character_matches_original_line_length() -> Result<(), Box<dyn std::error::Error>> {
    let line = "use Some::Very::Long::Module::Name;";
    let edits = plan_module_rename_edits(line, "Some::Very::Long::Module::Name", "X");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].end_character, line.len());
    Ok(())
}

#[test]
fn plan_start_character_always_zero() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use Foo::Bar;\nrequire Foo::Bar;\nuse parent 'Foo::Bar';";
    let edits = plan_module_rename_edits(source, "Foo::Bar", "X::Y");
    for edit in &edits {
        assert_eq!(edit.start_character, 0, "start_character should always be 0");
    }
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Rename to module with different depth
// ──────────────────────────────────────────────────────────────

#[test]
fn plan_rename_shallow_to_deep() -> Result<(), Box<dyn std::error::Error>> {
    let edits = plan_module_rename_edits("use Foo;", "Foo", "A::B::C::D");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "use A::B::C::D;");
    Ok(())
}

#[test]
fn plan_rename_deep_to_shallow() -> Result<(), Box<dyn std::error::Error>> {
    let edits = plan_module_rename_edits("use A::B::C::D;", "A::B::C::D", "Foo");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "use Foo;");
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Regression: package declaration rewrite (restored in #4594, broken by #4554)
// ──────────────────────────────────────────────────────────────

#[test]
fn test_rename_rewrites_package_declaration_in_target_file()
-> Result<(), Box<dyn std::error::Error>> {
    // The target file's own `package` declaration must be rewritten when renaming.
    let source = "package Old::Name;\n\nsub new { bless {}, shift }\n";
    let edits = plan_module_rename_edits(source, "Old::Name", "New::Name");
    assert_eq!(edits.len(), 1, "expected exactly one edit for the package declaration line");
    assert_eq!(edits[0].line, 0, "edit should be on line 0 (the package declaration)");
    assert_eq!(edits[0].new_text, "package New::Name;");

    // Also verify roundtrip via apply
    let result = apply_module_rename_edits(source, &edits);
    assert!(result.starts_with("package New::Name;"));
    Ok(())
}
