//! Comprehensive unit tests for `perl-module-token` public API.
//!
//! Covers: `module_variant_pairs`, `contains_module_token`, `replace_module_token`
//! with edge cases, boundary conditions, and real-world Perl patterns.

use perl_module::token::{contains_module_token, module_variant_pairs, replace_module_token};

// ---------------------------------------------------------------------------
// module_variant_pairs
// ---------------------------------------------------------------------------

#[test]
fn variant_pairs_canonical_two_segments() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("Foo::Bar", "Baz::Qux");
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0], ("Foo::Bar".into(), "Baz::Qux".into()));
    assert_eq!(pairs[1], ("Foo'Bar".into(), "Baz'Qux".into()));
    Ok(())
}

#[test]
fn variant_pairs_three_segments() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("A::B::C", "X::Y::Z");
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0], ("A::B::C".into(), "X::Y::Z".into()));
    assert_eq!(pairs[1], ("A'B'C".into(), "X'Y'Z".into()));
    Ok(())
}

#[test]
fn variant_pairs_legacy_input_normalizes_to_canonical() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("Foo'Bar", "Baz'Qux");
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0], ("Foo::Bar".into(), "Baz::Qux".into()));
    assert_eq!(pairs[1], ("Foo'Bar".into(), "Baz'Qux".into()));
    Ok(())
}

#[test]
fn variant_pairs_mixed_separators() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("Foo'Bar::Baz", "New'Mod::Path");
    // Should produce canonical and legacy forms
    assert!(!pairs.is_empty());
    // Canonical form uses :: throughout
    assert_eq!(pairs[0].0, "Foo::Bar::Baz");
    assert_eq!(pairs[0].1, "New::Mod::Path");
    Ok(())
}

#[test]
fn variant_pairs_single_segment_deduplicates() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("strict", "warnings");
    // No separators → canonical and legacy are identical → deduplicated to 1
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0], ("strict".into(), "warnings".into()));
    Ok(())
}

#[test]
fn variant_pairs_empty_strings() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("", "");
    // Empty → no separators → single dedup pair
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0], ("".into(), "".into()));
    Ok(())
}

#[test]
fn variant_pairs_deeply_nested_module() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("A::B::C::D::E", "V::W::X::Y::Z");
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0].0, "A::B::C::D::E");
    assert_eq!(pairs[1].0, "A'B'C'D'E");
    assert_eq!(pairs[0].1, "V::W::X::Y::Z");
    assert_eq!(pairs[1].1, "V'W'X'Y'Z");
    Ok(())
}

#[test]
fn variant_pairs_numeric_segments() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("Perl5::Module", "Perl6::Module");
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0], ("Perl5::Module".into(), "Perl6::Module".into()));
    assert_eq!(pairs[1], ("Perl5'Module".into(), "Perl6'Module".into()));
    Ok(())
}

#[test]
fn variant_pairs_underscore_segments() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("My::_Private::Mod", "My::_Internal::Mod");
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0].0, "My::_Private::Mod");
    assert_eq!(pairs[0].1, "My::_Internal::Mod");
    Ok(())
}

// ---------------------------------------------------------------------------
// contains_module_token
// ---------------------------------------------------------------------------

#[test]
fn contains_standalone_canonical_use_statement() -> Result<(), Box<dyn std::error::Error>> {
    assert!(contains_module_token("use Foo::Bar;", "Foo::Bar"));
    Ok(())
}

#[test]
fn contains_standalone_in_parent_pragma() -> Result<(), Box<dyn std::error::Error>> {
    assert!(contains_module_token("use parent 'Foo::Bar';", "Foo::Bar"));
    Ok(())
}

#[test]
fn contains_standalone_in_base_pragma() -> Result<(), Box<dyn std::error::Error>> {
    assert!(contains_module_token("use base qw(Foo::Bar);", "Foo::Bar"));
    Ok(())
}

#[test]
fn contains_rejects_prefix_match() -> Result<(), Box<dyn std::error::Error>> {
    assert!(!contains_module_token("use Foo::Barista;", "Foo::Bar"));
    Ok(())
}

#[test]
fn contains_rejects_suffix_match() -> Result<(), Box<dyn std::error::Error>> {
    assert!(!contains_module_token("use XFoo::Bar;", "Foo::Bar"));
    Ok(())
}

#[test]
fn contains_rejects_infix_match() -> Result<(), Box<dyn std::error::Error>> {
    assert!(!contains_module_token("use XFoo::BarY;", "Foo::Bar"));
    Ok(())
}

#[test]
fn contains_empty_module_name() -> Result<(), Box<dyn std::error::Error>> {
    assert!(!contains_module_token("use Foo::Bar;", ""));
    Ok(())
}

#[test]
fn contains_empty_line() -> Result<(), Box<dyn std::error::Error>> {
    assert!(!contains_module_token("", "Foo::Bar"));
    Ok(())
}

#[test]
fn contains_both_empty() -> Result<(), Box<dyn std::error::Error>> {
    assert!(!contains_module_token("", ""));
    Ok(())
}

#[test]
fn contains_module_at_line_start() -> Result<(), Box<dyn std::error::Error>> {
    assert!(contains_module_token("Foo::Bar->new()", "Foo::Bar"));
    Ok(())
}

#[test]
fn contains_module_at_line_end() -> Result<(), Box<dyn std::error::Error>> {
    assert!(contains_module_token("my $obj = Foo::Bar", "Foo::Bar"));
    Ok(())
}

#[test]
fn contains_module_surrounded_by_whitespace() -> Result<(), Box<dyn std::error::Error>> {
    assert!(contains_module_token("  Foo::Bar  ", "Foo::Bar"));
    Ok(())
}

#[test]
fn contains_single_segment_module() -> Result<(), Box<dyn std::error::Error>> {
    assert!(contains_module_token("use strict;", "strict"));
    Ok(())
}

#[test]
fn contains_single_segment_rejects_substring() -> Result<(), Box<dyn std::error::Error>> {
    assert!(!contains_module_token("use strictly;", "strict"));
    Ok(())
}

#[test]
fn contains_module_in_comment_line() -> Result<(), Box<dyn std::error::Error>> {
    // The function is line-level, not comment-aware
    assert!(contains_module_token("# use Foo::Bar;", "Foo::Bar"));
    Ok(())
}

#[test]
fn contains_legacy_separator() -> Result<(), Box<dyn std::error::Error>> {
    assert!(contains_module_token("use Foo'Bar;", "Foo'Bar"));
    Ok(())
}

#[test]
fn contains_multiple_occurrences_on_line() -> Result<(), Box<dyn std::error::Error>> {
    assert!(contains_module_token("use Foo::Bar; my $x = Foo::Bar->new;", "Foo::Bar"));
    Ok(())
}

#[test]
fn contains_module_after_arrow_operator() -> Result<(), Box<dyn std::error::Error>> {
    // Foo::Bar inside Foo::Bar::method is not standalone (it's a prefix of a longer token)
    assert!(!contains_module_token("$obj->Foo::Bar::method()", "Foo::Bar"));
    // But a standalone method call is detected
    assert!(contains_module_token("$obj->Foo::Bar", "Foo::Bar"));
    Ok(())
}

#[test]
fn contains_module_in_isa_check() -> Result<(), Box<dyn std::error::Error>> {
    assert!(contains_module_token("$obj->isa('Foo::Bar')", "Foo::Bar"));
    Ok(())
}

#[test]
fn contains_module_in_require() -> Result<(), Box<dyn std::error::Error>> {
    assert!(contains_module_token("require Foo::Bar;", "Foo::Bar"));
    Ok(())
}

#[test]
fn contains_module_with_tab_boundary() -> Result<(), Box<dyn std::error::Error>> {
    assert!(contains_module_token("use\tFoo::Bar;", "Foo::Bar"));
    Ok(())
}

// ---------------------------------------------------------------------------
// replace_module_token
// ---------------------------------------------------------------------------

#[test]
fn replace_basic_use_statement() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) = replace_module_token("use Foo::Bar;", "Foo::Bar", "New::Mod");
    assert!(changed);
    assert_eq!(out, "use New::Mod;");
    Ok(())
}

#[test]
fn replace_preserves_surrounding_context() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) =
        replace_module_token("use parent qw(Foo::Bar Baz::Qux);", "Foo::Bar", "New::Mod");
    assert!(changed);
    assert_eq!(out, "use parent qw(New::Mod Baz::Qux);");
    Ok(())
}

#[test]
fn replace_multiple_occurrences() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) =
        replace_module_token("use Foo::Bar; my $x = Foo::Bar->new;", "Foo::Bar", "X::Y");
    assert!(changed);
    assert_eq!(out, "use X::Y; my $x = X::Y->new;");
    Ok(())
}

#[test]
fn replace_does_not_modify_partial_prefix() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) = replace_module_token("use Foo::Barista;", "Foo::Bar", "X::Y");
    assert!(!changed);
    assert_eq!(out, "use Foo::Barista;");
    Ok(())
}

#[test]
fn replace_does_not_modify_partial_suffix() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) = replace_module_token("use XFoo::Bar;", "Foo::Bar", "X::Y");
    assert!(!changed);
    assert_eq!(out, "use XFoo::Bar;");
    Ok(())
}

#[test]
fn replace_empty_from_returns_unchanged() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) = replace_module_token("use Foo::Bar;", "", "X::Y");
    assert!(!changed);
    assert_eq!(out, "use Foo::Bar;");
    Ok(())
}

#[test]
fn replace_empty_line_returns_unchanged() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) = replace_module_token("", "Foo::Bar", "X::Y");
    assert!(!changed);
    assert_eq!(out, "");
    Ok(())
}

#[test]
fn replace_both_empty_returns_unchanged() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) = replace_module_token("", "", "X::Y");
    assert!(!changed);
    assert_eq!(out, "");
    Ok(())
}

#[test]
fn replace_with_same_value_reports_changed() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) = replace_module_token("use Foo::Bar;", "Foo::Bar", "Foo::Bar");
    // The function finds the token and replaces it (even with same value)
    assert!(changed);
    assert_eq!(out, "use Foo::Bar;");
    Ok(())
}

#[test]
fn replace_with_longer_replacement() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) = replace_module_token("use Foo;", "Foo", "Very::Long::Module::Name::Here");
    assert!(changed);
    assert_eq!(out, "use Very::Long::Module::Name::Here;");
    Ok(())
}

#[test]
fn replace_with_shorter_replacement() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) =
        replace_module_token("use Very::Long::Module::Name;", "Very::Long::Module::Name", "X");
    assert!(changed);
    assert_eq!(out, "use X;");
    Ok(())
}

#[test]
fn replace_legacy_separator_not_matched_when_child_exists() -> Result<(), Box<dyn std::error::Error>>
{
    let (out, changed) = replace_module_token("use Foo'Bar'Baz;", "Foo'Bar", "X'Y");
    assert!(!changed);
    assert_eq!(out, "use Foo'Bar'Baz;");
    Ok(())
}

#[test]
fn replace_module_at_start_of_line() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) = replace_module_token("Foo::Bar->new()", "Foo::Bar", "X::Y");
    assert!(changed);
    assert_eq!(out, "X::Y->new()");
    Ok(())
}

#[test]
fn replace_module_at_end_of_line() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) = replace_module_token("my $class = Foo::Bar", "Foo::Bar", "X::Y");
    assert!(changed);
    assert_eq!(out, "my $class = X::Y");
    Ok(())
}

#[test]
fn replace_module_only_content() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) = replace_module_token("Foo::Bar", "Foo::Bar", "X::Y");
    assert!(changed);
    assert_eq!(out, "X::Y");
    Ok(())
}

#[test]
fn replace_in_require_statement() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) = replace_module_token("require Foo::Bar;", "Foo::Bar", "New::Mod");
    assert!(changed);
    assert_eq!(out, "require New::Mod;");
    Ok(())
}

#[test]
fn replace_in_isa_expression() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) = replace_module_token("$obj->isa('Foo::Bar')", "Foo::Bar", "New::Mod");
    assert!(changed);
    assert_eq!(out, "$obj->isa('New::Mod')");
    Ok(())
}

#[test]
fn replace_in_method_call() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) =
        replace_module_token("my $obj = Foo::Bar->new(%args);", "Foo::Bar", "X::Y");
    assert!(changed);
    assert_eq!(out, "my $obj = X::Y->new(%args);");
    Ok(())
}

#[test]
fn replace_preserves_indentation() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) = replace_module_token("    use Foo::Bar;", "Foo::Bar", "X::Y");
    assert!(changed);
    assert_eq!(out, "    use X::Y;");
    Ok(())
}

#[test]
fn replace_preserves_tab_indentation() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) = replace_module_token("\tuse Foo::Bar;", "Foo::Bar", "X::Y");
    assert!(changed);
    assert_eq!(out, "\tuse X::Y;");
    Ok(())
}

#[test]
fn replace_in_string_with_double_quotes() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) = replace_module_token(r#"my $class = "Foo::Bar";"#, "Foo::Bar", "New::Mod");
    assert!(changed);
    assert_eq!(out, r#"my $class = "New::Mod";"#);
    Ok(())
}

#[test]
fn replace_module_adjacent_to_semicolon() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) = replace_module_token("Foo::Bar;", "Foo::Bar", "X::Y");
    assert!(changed);
    assert_eq!(out, "X::Y;");
    Ok(())
}

#[test]
fn replace_module_adjacent_to_parens() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) = replace_module_token("(Foo::Bar)", "Foo::Bar", "X::Y");
    assert!(changed);
    assert_eq!(out, "(X::Y)");
    Ok(())
}

#[test]
fn replace_module_adjacent_to_comma() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) = replace_module_token("Foo::Bar,Baz::Qux", "Foo::Bar", "X::Y");
    assert!(changed);
    assert_eq!(out, "X::Y,Baz::Qux");
    Ok(())
}

#[test]
fn replace_single_segment_module() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) = replace_module_token("use strict;", "strict", "warnings");
    assert!(changed);
    assert_eq!(out, "use warnings;");
    Ok(())
}

#[test]
fn replace_no_match_returns_original_string() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) = replace_module_token("use Foo::Bar;", "Baz::Qux", "X::Y");
    assert!(!changed);
    assert_eq!(out, "use Foo::Bar;");
    Ok(())
}

// ---------------------------------------------------------------------------
// Interaction: variant_pairs + replace_module_token (rename workflow)
// ---------------------------------------------------------------------------

#[test]
fn full_rename_workflow_canonical() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("My::Module", "My::Renamed");
    let line = "use My::Module;";

    let mut result = line.to_string();
    for (old, new) in &pairs {
        let (candidate, changed) = replace_module_token(&result, old, new);
        if changed {
            result = candidate;
        }
    }
    assert_eq!(result, "use My::Renamed;");
    Ok(())
}

#[test]
fn full_rename_workflow_legacy() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("My::Module", "My::Renamed");
    let line = "use My'Module;";

    let mut result = line.to_string();
    for (old, new) in &pairs {
        let (candidate, changed) = replace_module_token(&result, old, new);
        if changed {
            result = candidate;
        }
    }
    assert_eq!(result, "use My'Renamed;");
    Ok(())
}

#[test]
fn full_rename_workflow_no_match() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("My::Module", "My::Renamed");
    let line = "use Other::Module;";

    let mut result = line.to_string();
    for (old, new) in &pairs {
        let (candidate, changed) = replace_module_token(&result, old, new);
        if changed {
            result = candidate;
        }
    }
    assert_eq!(result, "use Other::Module;");
    Ok(())
}

// ---------------------------------------------------------------------------
// Consistency: contains_module_token ↔ replace_module_token agreement
// ---------------------------------------------------------------------------

#[test]
fn contains_and_replace_agree_on_presence() -> Result<(), Box<dyn std::error::Error>> {
    let cases = [
        ("use Foo::Bar;", "Foo::Bar"),
        ("use Foo::Barista;", "Foo::Bar"),
        ("Foo::Bar", "Foo::Bar"),
        ("", "Foo::Bar"),
        ("use Foo::Bar;", ""),
        ("use strict;", "strict"),
        ("use strictly;", "strict"),
        ("  Foo::Bar  ", "Foo::Bar"),
    ];

    for (line, module) in &cases {
        let present = contains_module_token(line, module);
        let (_, changed) = replace_module_token(line, module, module);
        assert_eq!(
            present, changed,
            "Disagreement for line={line:?}, module={module:?}: contains={present}, replace_changed={changed}"
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Unicode and special characters
// ---------------------------------------------------------------------------

#[test]
fn replace_preserves_unicode_context() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) = replace_module_token("# café Foo::Bar résumé", "Foo::Bar", "X::Y");
    assert!(changed);
    assert_eq!(out, "# café X::Y résumé");
    Ok(())
}

#[test]
fn contains_with_unicode_context() -> Result<(), Box<dyn std::error::Error>> {
    assert!(contains_module_token("# café Foo::Bar résumé", "Foo::Bar"));
    Ok(())
}

// ---------------------------------------------------------------------------
// Real-world Perl patterns
// ---------------------------------------------------------------------------

#[test]
fn replace_in_moose_extends() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) = replace_module_token("extends 'Foo::Bar';", "Foo::Bar", "New::Base");
    assert!(changed);
    assert_eq!(out, "extends 'New::Base';");
    Ok(())
}

#[test]
fn replace_in_moose_with() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) =
        replace_module_token("with 'Foo::Bar::Role';", "Foo::Bar::Role", "New::Role");
    assert!(changed);
    assert_eq!(out, "with 'New::Role';");
    Ok(())
}

#[test]
fn replace_in_package_declaration() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) = replace_module_token("package Foo::Bar;", "Foo::Bar", "New::Mod");
    assert!(changed);
    assert_eq!(out, "package New::Mod;");
    Ok(())
}

#[test]
fn replace_in_use_with_import_list() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) =
        replace_module_token("use Foo::Bar qw(func1 func2);", "Foo::Bar", "New::Mod");
    assert!(changed);
    assert_eq!(out, "use New::Mod qw(func1 func2);");
    Ok(())
}

#[test]
fn replace_in_fully_qualified_call() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) = replace_module_token("Foo::Bar::some_function($arg);", "Foo::Bar", "X::Y");
    // "Foo::Bar" is a prefix of the larger token "Foo::Bar::some_function"
    // so boundary checking should prevent the match
    assert!(!changed);
    assert_eq!(out, "Foo::Bar::some_function($arg);");
    Ok(())
}

#[test]
fn replace_in_eval_string() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) = replace_module_token(r#"eval "use Foo::Bar";"#, "Foo::Bar", "New::Mod");
    assert!(changed);
    assert_eq!(out, r#"eval "use New::Mod";"#);
    Ok(())
}

#[test]
fn replace_multiple_different_modules_sequentially() -> Result<(), Box<dyn std::error::Error>> {
    let line = "use Foo::Bar; use Baz::Qux;";
    let (step1, changed1) = replace_module_token(line, "Foo::Bar", "New::A");
    assert!(changed1);
    let (step2, changed2) = replace_module_token(&step1, "Baz::Qux", "New::B");
    assert!(changed2);
    assert_eq!(step2, "use New::A; use New::B;");
    Ok(())
}

#[test]
fn replace_in_qw_list() -> Result<(), Box<dyn std::error::Error>> {
    let (out, changed) =
        replace_module_token("use base qw(Foo::Bar Foo::Baz);", "Foo::Bar", "X::Y");
    assert!(changed);
    assert_eq!(out, "use base qw(X::Y Foo::Baz);");
    Ok(())
}
