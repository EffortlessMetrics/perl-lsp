//! Regression tests for Perl 5.36+ `builtin::` namespace function parsing.
//!
//! # Characterization (issue #796)
//!
//! The scout was uncertain whether `builtin::func(...)` calls and `use builtin qw(...)`
//! statements would parse correctly.  Manual characterization confirmed that ALL tested
//! forms parse cleanly — `builtin::` is treated as an ordinary package-qualified name by
//! the lexer, so no parser fix was required.
//!
//! These tests LOCK the behaviour so future refactors cannot silently regress it.
//!
//! ## Contexts tested
//! - `use builtin qw(...)` declaration
//! - nullary constant (`builtin::true`, `builtin::false`) as rvalue
//! - qualified call in scalar assignment (`builtin::is_bool($x)`)
//! - qualified call as `print` argument (`builtin::blessed($obj)`)
//! - qualified call in `if` condition (`builtin::is_bool($x)`)
//! - qualified call as hash value (`k => builtin::true`)
//! - statement-level qualified call (`builtin::weaken($ref)`)
//! - qualified call with numeric literal arg (`builtin::ceil(3.7)`)
//! - `use feature "builtin"` followed by `use builtin qw(...)`
//! - chained multi-statement program

mod cpan_test_helpers;
use cpan_test_helpers::assert_clean_parse;

use perl_parser_core::Parser;
use perl_tdd_support::must;

/// Helper: parse source and return the sexp, asserting no ERROR node.
fn parse_ok(src: &str) -> String {
    let mut parser = Parser::new(src);
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "Expected clean parse for: {src}\ngot: {sexp}");
    sexp
}

// ── use-declaration forms ────────────────────────────────────────────────────

#[test]
fn test_use_builtin_qw_true_false_clean() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse("use builtin qw(true false);");
    Ok(())
}

#[test]
fn test_use_builtin_qw_multiple_funcs_clean() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse("use builtin qw(true false ceil floor);");
    Ok(())
}

#[test]
fn test_use_builtin_qw_all_common_funcs_clean() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse(
        "use builtin qw(true false is_bool blessed reftype weaken unweaken isweak \
         created_as_string created_as_number stringify_infnan floor ceil trim);",
    );
    Ok(())
}

#[test]
fn test_use_builtin_sexp_structure() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = parse_ok("use builtin qw(true false);");
    assert!(sexp.contains("(use builtin"), "expected use-builtin node in sexp, got: {sexp}");
    Ok(())
}

// ── nullary constants ────────────────────────────────────────────────────────

#[test]
fn test_builtin_true_as_rvalue_clean() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse("my $x = builtin::true;");
    Ok(())
}

#[test]
fn test_builtin_false_as_rvalue_clean() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse("my $x = builtin::false;");
    Ok(())
}

#[test]
fn test_builtin_true_identifier_in_sexp() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = parse_ok("my $x = builtin::true;");
    assert!(
        sexp.contains("builtin::true"),
        "expected qualified identifier 'builtin::true' in sexp, got: {sexp}"
    );
    Ok(())
}

// ── scalar-assignment contexts ───────────────────────────────────────────────

#[test]
fn test_builtin_is_bool_scalar_assignment_clean() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse("my $b = builtin::is_bool($x);");
    Ok(())
}

#[test]
fn test_builtin_trim_scalar_assignment_clean() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse("my $t = builtin::trim($s);");
    Ok(())
}

#[test]
fn test_builtin_reftype_scalar_assignment_clean() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse("my $t = builtin::reftype($ref);");
    Ok(())
}

#[test]
fn test_builtin_ceil_numeric_arg_clean() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse("my $c = builtin::ceil(3.7);");
    Ok(())
}

#[test]
fn test_builtin_floor_numeric_arg_clean() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse("my $c = builtin::floor(3.2);");
    Ok(())
}

// ── function-argument contexts ───────────────────────────────────────────────

#[test]
fn test_builtin_blessed_as_print_arg_clean() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse("print builtin::blessed($obj);");
    Ok(())
}

#[test]
fn test_builtin_blessed_as_print_arg_sexp() -> Result<(), Box<dyn std::error::Error>> {
    // The sexp renders qualified calls as ambiguous_function_call_expression with a
    // (function) child — the qualified name text is the node's text value, not shown
    // verbatim in the sexp.  Assert structural shape rather than name appearance.
    let sexp = parse_ok("print builtin::blessed($obj);");
    assert!(
        sexp.contains("(call print ("),
        "expected print call wrapping the builtin::blessed call, got: {sexp}"
    );
    assert!(
        sexp.contains("(ambiguous_function_call_expression (function) (variable $ obj))"),
        "expected ambiguous_function_call_expression inside print for builtin::blessed($obj), got: {sexp}"
    );
    Ok(())
}

// ── conditional contexts ─────────────────────────────────────────────────────

#[test]
fn test_builtin_is_bool_in_if_condition_clean() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse("if (builtin::is_bool($x)) { 1 }");
    Ok(())
}

#[test]
fn test_builtin_is_bool_in_if_condition_sexp() -> Result<(), Box<dyn std::error::Error>> {
    // The sexp renders qualified calls as ambiguous_function_call_expression with a
    // (function) child — qualified name text is the node's text value, not shown verbatim.
    // Assert structural shape: if-node with a function-call condition.
    let sexp = parse_ok("if (builtin::is_bool($x)) { 1 }");
    assert!(sexp.contains("(if "), "expected if-node in sexp, got: {sexp}");
    assert!(
        sexp.contains("(ambiguous_function_call_expression (function) (variable $ x))"),
        "expected qualified call in if condition for builtin::is_bool($x), got: {sexp}"
    );
    Ok(())
}

#[test]
fn test_builtin_isweak_in_unless_condition_clean() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse("unless (builtin::isweak($ref)) { warn 'strong'; }");
    Ok(())
}

// ── hash-value contexts ──────────────────────────────────────────────────────

#[test]
fn test_builtin_true_as_hash_value_clean() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse("my %h = (k => builtin::true);");
    Ok(())
}

#[test]
fn test_builtin_false_as_hash_value_clean() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse("my %h = (enabled => builtin::false);");
    Ok(())
}

#[test]
fn test_builtin_true_as_hash_value_sexp() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = parse_ok("my %h = (k => builtin::true);");
    assert!(
        sexp.contains("builtin::true"),
        "expected 'builtin::true' as hash value in sexp, got: {sexp}"
    );
    Ok(())
}

// ── statement-level contexts ─────────────────────────────────────────────────

#[test]
fn test_builtin_weaken_statement_clean() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse("builtin::weaken($ref);");
    Ok(())
}

#[test]
fn test_builtin_unweaken_statement_clean() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse("builtin::unweaken($ref);");
    Ok(())
}

#[test]
fn test_builtin_export_lexically_statement_clean() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse(r#"builtin::export_lexically(true => \&true);"#);
    Ok(())
}

// ── use feature + use builtin combination ───────────────────────────────────

#[test]
fn test_use_feature_builtin_then_use_builtin_qw_clean() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse(r#"use feature "builtin"; use builtin qw(true false);"#);
    Ok(())
}

#[test]
fn test_use_builtin_then_no_warnings_experimental_clean() -> Result<(), Box<dyn std::error::Error>>
{
    assert_clean_parse(r#"use builtin qw(true false); no warnings "experimental";"#);
    Ok(())
}

// ── multi-statement / chained programs ──────────────────────────────────────

#[test]
fn test_builtin_chained_multi_statement_clean() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse(
        r#"my $x = builtin::true;
if (builtin::is_bool($x)) {
    print builtin::blessed($x) // "none";
}"#,
    );
    Ok(())
}

#[test]
fn test_builtin_full_preamble_program_clean() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse(
        r#"use v5.36;
use builtin qw(true false is_bool blessed weaken);
no warnings "experimental";

my $val = builtin::true;
if (builtin::is_bool($val)) {
    my $class = builtin::blessed($val) // "none";
    print "class: $class\n";
}
builtin::weaken($val);"#,
    );
    Ok(())
}
