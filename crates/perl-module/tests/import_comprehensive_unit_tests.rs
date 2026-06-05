//! Comprehensive unit tests for `perl-module-import`.
//!
//! Covers the full public API surface: `parse_module_import_head`,
//! `ModuleImportKind`, and `ModuleImportHead`.

use perl_module::import::{
    ModuleImportHead, ModuleImportKind, RequireForm, parse_module_import_head,
};

// ---------------------------------------------------------------------------
// Helper: assert a successful parse and validate all fields
// ---------------------------------------------------------------------------

fn assert_parse(
    line: &str,
    expected_kind: ModuleImportKind,
    expected_token: &str,
    expected_start: usize,
    expected_end: usize,
) -> Result<(), String> {
    let head = parse_module_import_head(line)
        .ok_or_else(|| format!("expected Some for line: {line:?}"))?;

    if head.kind != expected_kind {
        return Err(format!("kind mismatch: {:?} != {:?}", head.kind, expected_kind));
    }
    if head.token != expected_token {
        return Err(format!("token mismatch: {:?} != {:?}", head.token, expected_token));
    }
    if head.token_start != expected_start {
        return Err(format!("token_start mismatch: {} != {}", head.token_start, expected_start));
    }
    if head.token_end != expected_end {
        return Err(format!("token_end mismatch: {} != {}", head.token_end, expected_end));
    }
    // Cross-check: slice must equal token
    if &line[head.token_start..head.token_end] != head.token {
        return Err(format!(
            "slice mismatch: {:?} != {:?}",
            &line[head.token_start..head.token_end],
            head.token,
        ));
    }
    Ok(())
}

// ===========================================================================
// 1. ModuleImportKind — trait derivations
// ===========================================================================

#[test]
fn kind_debug_displays_variant_name() -> Result<(), String> {
    let dbg = format!("{:?}", ModuleImportKind::Use);
    if !dbg.contains("Use") {
        return Err(format!("unexpected Debug output: {dbg}"));
    }
    Ok(())
}

#[test]
fn kind_clone_produces_equal_value() -> Result<(), String> {
    let original = ModuleImportKind::Require;
    let cloned = original;
    if original != cloned {
        return Err("Clone/Copy equality failed".into());
    }
    Ok(())
}

#[test]
fn kind_partial_eq_distinguishes_all_variants() -> Result<(), String> {
    let variants = [
        ModuleImportKind::Use,
        ModuleImportKind::Require,
        ModuleImportKind::UseParent,
        ModuleImportKind::UseBase,
    ];
    for (i, a) in variants.iter().enumerate() {
        for (j, b) in variants.iter().enumerate() {
            if (i == j) != (*a == *b) {
                return Err(format!("PartialEq broken for variants {i} and {j}"));
            }
        }
    }
    Ok(())
}

// ===========================================================================
// 2. ModuleImportHead — struct field access & trait derivations
// ===========================================================================

#[test]
fn head_debug_includes_all_fields() -> Result<(), String> {
    let head = parse_module_import_head("use Foo;").ok_or_else(|| "expected Some".to_string())?;
    let dbg = format!("{head:?}");
    if !dbg.contains("Foo") || !dbg.contains("Use") {
        return Err(format!("Debug missing expected content: {dbg}"));
    }
    Ok(())
}

#[test]
fn head_clone_produces_equal_value() -> Result<(), String> {
    let head = parse_module_import_head("use Foo;").ok_or_else(|| "expected Some".to_string())?;
    let cloned: ModuleImportHead<'_> = head;
    if head != cloned {
        return Err("Clone/Copy equality failed".into());
    }
    Ok(())
}

// ===========================================================================
// 3. Basic `use` statement parsing
// ===========================================================================

#[test]
fn use_simple_module() -> Result<(), String> {
    assert_parse("use Foo;", ModuleImportKind::Use, "Foo", 4, 7)
}

#[test]
fn use_qualified_module() -> Result<(), String> {
    assert_parse("use Foo::Bar;", ModuleImportKind::Use, "Foo::Bar", 4, 12)
}

#[test]
fn use_deeply_qualified_module() -> Result<(), String> {
    assert_parse("use A::B::C::D::E;", ModuleImportKind::Use, "A::B::C::D::E", 4, 17)
}

#[test]
fn use_with_import_list() -> Result<(), String> {
    // Parenthesis is a delimiter so token stops before it
    assert_parse("use POSIX(floor);", ModuleImportKind::Use, "POSIX", 4, 9)
}

#[test]
fn use_with_space_before_parens() -> Result<(), String> {
    assert_parse("use POSIX (floor);", ModuleImportKind::Use, "POSIX", 4, 9)
}

#[test]
fn use_module_no_semicolon() -> Result<(), String> {
    // Token runs to end of line when no delimiter follows
    assert_parse("use Foo::Bar", ModuleImportKind::Use, "Foo::Bar", 4, 12)
}

#[test]
fn use_with_version_number_first() -> Result<(), String> {
    // `use 5.010;` — the version number is the token
    assert_parse("use 5.010;", ModuleImportKind::Use, "5.010", 4, 9)
}

#[test]
fn use_strict() -> Result<(), String> {
    assert_parse("use strict;", ModuleImportKind::Use, "strict", 4, 10)
}

#[test]
fn use_warnings() -> Result<(), String> {
    assert_parse("use warnings;", ModuleImportKind::Use, "warnings", 4, 12)
}

// ===========================================================================
// 4. Basic `require` statement parsing
// ===========================================================================

#[test]
fn require_simple_module() -> Result<(), String> {
    assert_parse("require Foo;", ModuleImportKind::Require, "Foo", 8, 11)
}

#[test]
fn require_qualified_module() -> Result<(), String> {
    assert_parse("require Foo::Bar;", ModuleImportKind::Require, "Foo::Bar", 8, 16)
}

#[test]
fn require_module_no_semicolon() -> Result<(), String> {
    assert_parse("require Foo::Bar", ModuleImportKind::Require, "Foo::Bar", 8, 16)
}

// ===========================================================================
// 5. `use parent` / `use base` specializations
// ===========================================================================

#[test]
fn use_parent_with_qw() -> Result<(), String> {
    assert_parse("use parent qw(Foo::Bar);", ModuleImportKind::UseParent, "parent", 4, 10)
}

#[test]
fn use_parent_with_single_quote() -> Result<(), String> {
    assert_parse("use parent 'Foo::Bar';", ModuleImportKind::UseParent, "parent", 4, 10)
}

#[test]
fn use_base_with_qw() -> Result<(), String> {
    assert_parse("use base qw(Foo::Bar Baz::Quux);", ModuleImportKind::UseBase, "base", 4, 8)
}

#[test]
fn use_base_with_quoted_string() -> Result<(), String> {
    assert_parse("use base 'Foo::Bar';", ModuleImportKind::UseBase, "base", 4, 8)
}

// ===========================================================================
// 6. Leading whitespace handling — offset correctness
// ===========================================================================

#[test]
fn use_with_single_space_indent() -> Result<(), String> {
    assert_parse(" use Foo;", ModuleImportKind::Use, "Foo", 5, 8)
}

#[test]
fn use_with_tab_indent() -> Result<(), String> {
    assert_parse("\tuse Foo;", ModuleImportKind::Use, "Foo", 5, 8)
}

#[test]
fn use_with_four_space_indent() -> Result<(), String> {
    assert_parse("    use Foo;", ModuleImportKind::Use, "Foo", 8, 11)
}

#[test]
fn require_with_two_space_indent() -> Result<(), String> {
    assert_parse("  require Foo::Bar;", ModuleImportKind::Require, "Foo::Bar", 10, 18)
}

#[test]
fn use_parent_with_indent() -> Result<(), String> {
    assert_parse("  use parent 'Base';", ModuleImportKind::UseParent, "parent", 6, 12)
}

#[test]
fn use_with_multiple_spaces_between_keyword_and_token() -> Result<(), String> {
    assert_parse("use   Foo::Bar;", ModuleImportKind::Use, "Foo::Bar", 6, 14)
}

#[test]
fn use_with_tab_between_keyword_and_token() -> Result<(), String> {
    assert_parse("use\tFoo::Bar;", ModuleImportKind::Use, "Foo::Bar", 4, 12)
}

#[test]
fn require_with_mixed_whitespace() -> Result<(), String> {
    assert_parse(" \t require  Foo;", ModuleImportKind::Require, "Foo", 12, 15)
}

// ===========================================================================
// 7. Rejection cases — lines that are NOT imports
// ===========================================================================

#[test]
fn rejects_empty_string() -> Result<(), String> {
    if parse_module_import_head("").is_some() {
        return Err("expected None for empty string".into());
    }
    Ok(())
}

#[test]
fn rejects_whitespace_only() -> Result<(), String> {
    if parse_module_import_head("   ").is_some() {
        return Err("expected None for whitespace-only".into());
    }
    Ok(())
}

#[test]
fn rejects_comment_line() -> Result<(), String> {
    if parse_module_import_head("# use Foo;").is_some() {
        return Err("expected None for comment line".into());
    }
    Ok(())
}

#[test]
fn rejects_package_statement() -> Result<(), String> {
    if parse_module_import_head("package Foo::Bar;").is_some() {
        return Err("expected None for package statement".into());
    }
    Ok(())
}

#[test]
fn rejects_no_module() -> Result<(), String> {
    if parse_module_import_head("no strict;").is_some() {
        return Err("expected None for 'no' statement".into());
    }
    Ok(())
}

#[test]
fn rejects_user_not_use() -> Result<(), String> {
    if parse_module_import_head("user Foo::Bar;").is_some() {
        return Err("expected None for 'user'".into());
    }
    Ok(())
}

#[test]
fn rejects_required_not_require() -> Result<(), String> {
    if parse_module_import_head("required Foo::Bar;").is_some() {
        return Err("expected None for 'required'".into());
    }
    Ok(())
}

#[test]
fn rejects_useful_not_use() -> Result<(), String> {
    if parse_module_import_head("useful Foo::Bar;").is_some() {
        return Err("expected None for 'useful'".into());
    }
    Ok(())
}

#[test]
fn rejects_use_without_whitespace_before_token() -> Result<(), String> {
    // "useFoo" — no whitespace boundary after keyword
    if parse_module_import_head("useFoo;").is_some() {
        return Err("expected None for 'useFoo'".into());
    }
    Ok(())
}

#[test]
fn rejects_require_without_whitespace_before_token() -> Result<(), String> {
    if parse_module_import_head("requireFoo;").is_some() {
        return Err("expected None for 'requireFoo'".into());
    }
    Ok(())
}

#[test]
fn rejects_use_with_only_semicolon() -> Result<(), String> {
    if parse_module_import_head("use ;").is_some() {
        return Err("expected None for 'use ;'".into());
    }
    Ok(())
}

#[test]
fn rejects_use_with_only_paren() -> Result<(), String> {
    if parse_module_import_head("use (").is_some() {
        return Err("expected None for 'use ('".into());
    }
    Ok(())
}

#[test]
fn rejects_require_with_only_whitespace() -> Result<(), String> {
    if parse_module_import_head("require   ").is_some() {
        return Err("expected None for 'require   '".into());
    }
    Ok(())
}

#[test]
fn rejects_require_alone() -> Result<(), String> {
    if parse_module_import_head("require").is_some() {
        return Err("expected None for bare 'require'".into());
    }
    Ok(())
}

#[test]
fn rejects_use_alone() -> Result<(), String> {
    if parse_module_import_head("use").is_some() {
        return Err("expected None for bare 'use'".into());
    }
    Ok(())
}

#[test]
fn rejects_use_followed_by_only_semicolons() -> Result<(), String> {
    if parse_module_import_head("use ;;;").is_some() {
        return Err("expected None for 'use ;;;'".into());
    }
    Ok(())
}

#[test]
fn rejects_arbitrary_text() -> Result<(), String> {
    if parse_module_import_head("my $x = 42;").is_some() {
        return Err("expected None for arbitrary code".into());
    }
    Ok(())
}

#[test]
fn rejects_method_call_with_use() -> Result<(), String> {
    if parse_module_import_head("$obj->use('arg');").is_some() {
        return Err("expected None for method call".into());
    }
    Ok(())
}

#[test]
fn rejects_function_call_use_module() -> Result<(), String> {
    if parse_module_import_head("my $x = use_module();").is_some() {
        return Err("expected None for use_module()".into());
    }
    Ok(())
}

// ===========================================================================
// 8. Token offset invariants
// ===========================================================================

#[test]
fn token_start_less_than_token_end_for_all_valid_parses() -> Result<(), String> {
    let cases = [
        "use Foo;",
        "  use Foo::Bar;",
        "\tuse parent 'X';",
        "require Baz;",
        "  require Qux::Quux;",
        "use base qw(A);",
    ];
    for line in &cases {
        let head =
            parse_module_import_head(line).ok_or_else(|| format!("expected Some for: {line:?}"))?;
        if head.token_start >= head.token_end {
            return Err(format!(
                "token_start ({}) >= token_end ({}) for {line:?}",
                head.token_start, head.token_end,
            ));
        }
    }
    Ok(())
}

#[test]
fn token_end_within_line_bounds() -> Result<(), String> {
    let cases = ["use Foo;", "use Foo::Bar", "  require Baz", "use parent"];
    for line in &cases {
        if let Some(head) = parse_module_import_head(line) {
            if head.token_end > line.len() {
                return Err(format!(
                    "token_end ({}) > line.len() ({}) for {line:?}",
                    head.token_end,
                    line.len(),
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn token_slice_equals_token_field() -> Result<(), String> {
    let cases = [
        "use Foo;",
        "  use Foo::Bar::Baz;",
        "\trequire Module;",
        "use parent 'X';",
        "use base qw(Y);",
        "use   Spaced::Module;",
    ];
    for line in &cases {
        let head =
            parse_module_import_head(line).ok_or_else(|| format!("expected Some for: {line:?}"))?;
        let slice = &line[head.token_start..head.token_end];
        if slice != head.token {
            return Err(format!("slice {slice:?} != token {:?} for {line:?}", head.token));
        }
    }
    Ok(())
}

// ===========================================================================
// 9. Token delimiter behaviour
// ===========================================================================

#[test]
fn semicolon_terminates_token() -> Result<(), String> {
    assert_parse("use Foo;bar", ModuleImportKind::Use, "Foo", 4, 7)
}

#[test]
fn open_paren_terminates_token() -> Result<(), String> {
    assert_parse("use Foo(bar)", ModuleImportKind::Use, "Foo", 4, 7)
}

#[test]
fn close_paren_terminates_token() -> Result<(), String> {
    assert_parse("use Foo)bar", ModuleImportKind::Use, "Foo", 4, 7)
}

#[test]
fn space_terminates_token() -> Result<(), String> {
    assert_parse("use Foo bar", ModuleImportKind::Use, "Foo", 4, 7)
}

// ===========================================================================
// 10. Real-world Perl idioms
// ===========================================================================

#[test]
fn use_carp_qw_croak() -> Result<(), String> {
    assert_parse("use Carp qw(croak);", ModuleImportKind::Use, "Carp", 4, 8)
}

#[test]
fn use_scalar_util() -> Result<(), String> {
    assert_parse("use Scalar::Util 'blessed';", ModuleImportKind::Use, "Scalar::Util", 4, 16)
}

#[test]
fn use_file_basename() -> Result<(), String> {
    assert_parse("use File::Basename;", ModuleImportKind::Use, "File::Basename", 4, 18)
}

#[test]
fn use_moose() -> Result<(), String> {
    assert_parse("use Moose;", ModuleImportKind::Use, "Moose", 4, 9)
}

#[test]
fn use_moo() -> Result<(), String> {
    assert_parse("use Moo;", ModuleImportKind::Use, "Moo", 4, 7)
}

#[test]
fn use_test_more() -> Result<(), String> {
    assert_parse("use Test::More tests => 42;", ModuleImportKind::Use, "Test::More", 4, 14)
}

#[test]
fn require_carp() -> Result<(), String> {
    assert_parse("require Carp;", ModuleImportKind::Require, "Carp", 8, 12)
}

#[test]
fn use_parent_multiple_bases() -> Result<(), String> {
    assert_parse(
        "use parent qw(Foo::Base Bar::Base);",
        ModuleImportKind::UseParent,
        "parent",
        4,
        10,
    )
}

#[test]
fn use_base_exporter() -> Result<(), String> {
    assert_parse("use base 'Exporter';", ModuleImportKind::UseBase, "base", 4, 8)
}

// ===========================================================================
// 11. Edge cases: unusual but valid token content
// ===========================================================================

#[test]
fn use_single_char_module() -> Result<(), String> {
    assert_parse("use X;", ModuleImportKind::Use, "X", 4, 5)
}

#[test]
fn require_single_char_module() -> Result<(), String> {
    assert_parse("require X;", ModuleImportKind::Require, "X", 8, 9)
}

#[test]
fn use_underscore_module() -> Result<(), String> {
    assert_parse("use _Private;", ModuleImportKind::Use, "_Private", 4, 12)
}

#[test]
fn use_module_with_numbers() -> Result<(), String> {
    assert_parse("use Encode::JP2K;", ModuleImportKind::Use, "Encode::JP2K", 4, 16)
}

#[test]
fn use_module_token_at_end_of_line() -> Result<(), String> {
    // No trailing delimiter — token extends to end of string
    assert_parse("use Foo::Bar", ModuleImportKind::Use, "Foo::Bar", 4, 12)
}

#[test]
fn require_module_token_at_end_of_line() -> Result<(), String> {
    assert_parse("require Foo::Bar", ModuleImportKind::Require, "Foo::Bar", 8, 16)
}

// ===========================================================================
// 12. Keyword boundary precision
// ===========================================================================

#[test]
fn rejects_uses_not_use() -> Result<(), String> {
    if parse_module_import_head("uses Foo;").is_some() {
        return Err("expected None for 'uses'".into());
    }
    Ok(())
}

#[test]
fn rejects_using_not_use() -> Result<(), String> {
    if parse_module_import_head("using Foo;").is_some() {
        return Err("expected None for 'using'".into());
    }
    Ok(())
}

#[test]
fn rejects_requires_not_require() -> Result<(), String> {
    if parse_module_import_head("requires Foo;").is_some() {
        return Err("expected None for 'requires'".into());
    }
    Ok(())
}

#[test]
fn rejects_requiring_not_require() -> Result<(), String> {
    if parse_module_import_head("requiring Foo;").is_some() {
        return Err("expected None for 'requiring'".into());
    }
    Ok(())
}

// ===========================================================================
// 13. Whitespace varieties after keyword
// ===========================================================================

#[test]
fn use_with_multiple_tabs_after_keyword() -> Result<(), String> {
    assert_parse("use\t\tFoo;", ModuleImportKind::Use, "Foo", 5, 8)
}

#[test]
fn require_with_tab_after_keyword() -> Result<(), String> {
    assert_parse("require\tFoo;", ModuleImportKind::Require, "Foo", 8, 11)
}

// ===========================================================================
// 14. Consecutive delimiter-only content after keyword
// ===========================================================================

#[test]
fn rejects_use_followed_by_parens_only() -> Result<(), String> {
    if parse_module_import_head("use ();").is_some() {
        return Err("expected None for 'use ();'".into());
    }
    Ok(())
}

#[test]
fn rejects_require_followed_by_semicolon() -> Result<(), String> {
    if parse_module_import_head("require ;").is_some() {
        return Err("expected None for 'require ;'".into());
    }
    Ok(())
}

// ===========================================================================
// 15. Batch validation — many lines, mix of valid/invalid
// ===========================================================================

#[test]
fn batch_valid_lines_all_parse() -> Result<(), String> {
    let lines = [
        "use Foo;",
        "use Foo::Bar;",
        "use parent 'X';",
        "use base qw(Y);",
        "require Baz;",
        "  use Indented;",
        "\trequire Tabbed;",
    ];
    for line in &lines {
        if parse_module_import_head(line).is_none() {
            return Err(format!("expected Some for: {line:?}"));
        }
    }
    Ok(())
}

#[test]
fn batch_invalid_lines_all_reject() -> Result<(), String> {
    let lines = [
        "",
        " ",
        "# comment",
        "package Foo;",
        "no strict;",
        "my $x = 1;",
        "sub foo {}",
        "user Foo;",
        "required Foo;",
        "useFoo;",
        "requireFoo;",
        "use ;",
        "require",
        "use",
        "use ;;;",
        "use ()",
    ];
    for line in &lines {
        if parse_module_import_head(line).is_some() {
            return Err(format!("expected None for: {line:?}"));
        }
    }
    Ok(())
}

// ===========================================================================
// 16. token_as_module_name — Option B accessor
// ===========================================================================

#[test]
fn token_as_module_name_converts_pm_path() -> Result<(), String> {
    let head = parse_module_import_head(r#"require "Foo/Bar.pm";"#)
        .ok_or_else(|| "expected Some for quoted require".to_string())?;
    // Raw token must stay as the file path (token_start/token_end invariant preserved)
    if head.token != "Foo/Bar.pm" {
        return Err(format!("token should be raw path, got {:?}", head.token));
    }
    // Accessor converts it to module name
    let name = head.token_as_module_name();
    if name != "Foo::Bar" {
        return Err(format!("expected 'Foo::Bar', got {:?}", name));
    }
    if head.require_form() != Some(RequireForm::FilePath) {
        return Err("expected FilePath require_form".into());
    }
    Ok(())
}

#[test]
fn token_as_module_name_single_segment_pm() -> Result<(), String> {
    let head = parse_module_import_head(r#"require "warnings.pm";"#)
        .ok_or_else(|| "expected Some".to_string())?;
    let name = head.token_as_module_name();
    if name != "warnings" {
        return Err(format!("expected 'warnings', got {:?}", name));
    }
    Ok(())
}

#[test]
fn token_as_module_name_deeply_nested_pm() -> Result<(), String> {
    let head = parse_module_import_head(r#"require "HTTP/Request/Common.pm";"#)
        .ok_or_else(|| "expected Some".to_string())?;
    let name = head.token_as_module_name();
    if name != "HTTP::Request::Common" {
        return Err(format!("expected 'HTTP::Request::Common', got {:?}", name));
    }
    Ok(())
}

#[test]
fn token_as_module_name_does_not_convert_pl_path() -> Result<(), String> {
    let head = parse_module_import_head(r#"require "foo.pl";"#)
        .ok_or_else(|| "expected Some".to_string())?;
    let name = head.token_as_module_name();
    // .pl files are script includes, not module names — must not convert
    if name != "foo.pl" {
        return Err(format!("expected raw 'foo.pl' (no conversion), got {:?}", name));
    }
    Ok(())
}

#[test]
fn token_as_module_name_does_not_convert_extensionless_path() -> Result<(), String> {
    let head = parse_module_import_head(r#"require "somelib";"#)
        .ok_or_else(|| "expected Some".to_string())?;
    let name = head.token_as_module_name();
    if name != "somelib" {
        return Err(format!("expected raw 'somelib', got {:?}", name));
    }
    Ok(())
}

#[test]
fn token_as_module_name_passthrough_for_module_name_form() -> Result<(), String> {
    let head =
        parse_module_import_head("require Foo::Bar;").ok_or_else(|| "expected Some".to_string())?;
    let name = head.token_as_module_name();
    if name != "Foo::Bar" {
        return Err(format!("expected 'Foo::Bar' passthrough, got {:?}", name));
    }
    Ok(())
}
