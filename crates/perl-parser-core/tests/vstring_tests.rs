mod cpan_test_helpers;
use cpan_test_helpers::*;

use perl_parser_core::Parser;
use perl_parser_core::hir::{HirKind, lower_ast};

#[test]
fn test_use_vstring() {
    let source = r#"use v5.38.0;"#;
    assert_clean_parse(source);
}

#[test]
fn test_vstring_in_expression() {
    let source = r#"my $v = v1.2.3;"#;
    assert_clean_parse(source);
}

#[test]
fn test_vstring_comparison() {
    let source = r#"$^V ge v5.10.0"#;
    assert_clean_parse(source);
}

#[test]
fn test_vstring_semantic_type() {
    // Test that v-strings are parsed as NodeKind::VString, not NodeKind::String.
    // Uses sexp output to verify the distinct (vstring ...) node is emitted.
    let source = r#"my $vstr = v1.2.3;"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("(vstring \"v1.2.3\")"),
        "expected (vstring \"v1.2.3\") in sexp but got: {}",
        sexp
    );
    assert!(
        !sexp.contains("(string \"v1.2.3\")"),
        "v-string must NOT be emitted as (string ...) but got: {}",
        sexp
    );
}

#[test]
fn test_vstring_long_form_semantic_type() {
    // Multi-component v-string: v1.2.3.4.5
    let source = r#"my $v = v1.2.3.4.5;"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("(vstring \"v1.2.3.4.5\")"),
        "expected (vstring \"v1.2.3.4.5\") in sexp but got: {}",
        sexp
    );
}

#[test]
fn test_ordinary_float_is_not_vstring() {
    // Ordinary floats (single dot) must remain Number nodes, not VString
    let source = r#"my $f = 3.14;"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("vstring"),
        "ordinary float 3.14 must not be classified as vstring, but got: {}",
        sexp
    );
}

#[test]
fn test_range_is_not_vstring() {
    // Ranges like 1..10 must not be misclassified as version strings
    let source = r#"my @r = 1..10;"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("vstring"),
        "range 1..10 must not contain a vstring node, but got: {}",
        sexp
    );
}

#[test]
fn test_vstring_no_dot_single_component() {
    // Bare single-component v-string: v5 is chr(5) in Perl — must parse cleanly
    // and be classified as VString, not Identifier or Number.
    let source = r#"my $chr5 = v5;"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("(vstring \"v5\")"),
        "single-component vstring v5 should produce (vstring \"v5\") but got: {}",
        sexp
    );
}

/// ripr+ gap 1: lower_ast must accept NodeKind::VString nodes produced by the
/// parser without panic.  This test exercises the `TokenKind::VString →
/// NodeKind::VString` arm in `primary.rs` (the core PR change) and then feeds
/// the resulting AST through `lower_ast`, verifying that:
///   1. The parser emits VString (sexp contains `(vstring ...)`), proving the
///      new arm in primary.rs was reached.  The test FAILS if the arm is
///      reverted — the sexp would contain `(string ...)` instead.
///   2. `lower_ast` completes without panic and produces a HIR file with at
///      least one `VariableDecl` item (from `my $v = v1.2.3;`), proving the
///      VString literal is lowered gracefully through the HIR pipeline.
#[test]
fn test_vstring_lowered_by_lower_ast() {
    let source = "my $v = v1.2.3;";
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();

    // Verify the parser emitted the VString node (exercises primary.rs arm).
    // If the PR's VString arm were reverted this assertion would fail because
    // the sexp would contain `(string "v1.2.3")` not `(vstring "v1.2.3")`.
    let sexp = output.ast.to_sexp();
    assert!(
        sexp.contains("(vstring \"v1.2.3\")"),
        "primary.rs VString arm must emit (vstring ...) in AST; got: {}",
        sexp
    );

    // lower_ast must not panic and must produce a HIR file.
    let hir = lower_ast(&output.ast);

    // The declaration `my $v = v1.2.3` must produce a VariableDecl item.
    // This confirms the VString was encountered and lowered without error.
    let has_var_decl = hir.items.iter().any(|item| matches!(item.kind, HirKind::VariableDecl(_)));
    assert!(
        has_var_decl,
        "lower_ast must emit a VariableDecl item for `my $v = v1.2.3`; \
         VString literal in the initializer must not cause lowering to skip the declaration"
    );
}
