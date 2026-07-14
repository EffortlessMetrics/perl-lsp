//! Tests for goto form distinction — issue #1732.
//!
//! Perl has three semantically distinct goto forms:
//!   - `goto LABEL`    — label jump (control flow)
//!   - `goto &sub`     — frame replacement (tail call, reuses @_)
//!   - `goto $expr`    — dynamic computed target
//!
//! These tests verify the parser populates `NodeKind::Goto { form, .. }` with
//! the correct `GotoTargetForm` discriminant so that semantic analysis and DAP
//! can distinguish the tail-call form from a layer jump.

mod cpan_test_helpers;

use perl_parser_core::{NodeKind, Parser};
use perl_tdd_support::must;

// Import the form enum from perl_ast (re-exported via perl_parser_core).
use perl_ast::GotoTargetForm;

// -----------------------------------------------------------------------
// Helper: parse code and find the first Goto node, returning a clone of
// its (target, form) fields.
// -----------------------------------------------------------------------

fn find_goto_form(code: &str) -> GotoTargetForm {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    // Walk recursively to find the first Goto node.
    fn find(node: &perl_parser_core::Node) -> Option<GotoTargetForm> {
        if let NodeKind::Goto { form, .. } = &node.kind {
            return Some(form.clone());
        }
        for child in node.children() {
            if let Some(f) = find(child) {
                return Some(f);
            }
        }
        None
    }

    must(find(&ast).ok_or("no Goto node found in AST"))
}

// -----------------------------------------------------------------------
// goto LABEL form
// -----------------------------------------------------------------------

#[test]
fn test_goto_plain_label_produces_label_form() {
    // Classic goto LABEL — should produce GotoTargetForm::Label
    let form = find_goto_form(
        r#"
RETRY:
    goto RETRY;
"#,
    );
    assert!(
        matches!(form, GotoTargetForm::Label),
        "goto LABEL should produce GotoTargetForm::Label, got: {form:?}"
    );
}

// -----------------------------------------------------------------------
// goto &sub form — bare name, qualified, and coderef-in-variable
// -----------------------------------------------------------------------

#[test]
fn test_goto_ampersand_simple_name_produces_sub_form() {
    // `goto &helper;` — frame replacement, should be GotoTargetForm::Sub
    let form = find_goto_form(
        r#"
sub wrapper { goto &helper; }
"#,
    );
    assert!(
        matches!(form, GotoTargetForm::Sub),
        "goto &helper should produce GotoTargetForm::Sub, got: {form:?}"
    );
}

#[test]
fn test_goto_ampersand_qualified_name_produces_sub_form() {
    // `goto &Pkg::sub;` — qualified form, still frame replacement
    let form = find_goto_form(
        r#"
sub wrapper { goto &Pkg::helper; }
"#,
    );
    assert!(
        matches!(form, GotoTargetForm::Sub),
        "goto &Pkg::helper should produce GotoTargetForm::Sub, got: {form:?}"
    );
}

#[test]
fn test_goto_ampersand_coderef_variable_produces_sub_form() {
    // `goto &$coderef;` — dynamic sub reference, still frame replacement form
    let form = find_goto_form(
        r#"
sub wrapper { goto &$dispatch; }
"#,
    );
    assert!(
        matches!(form, GotoTargetForm::Sub),
        "goto &$coderef should produce GotoTargetForm::Sub, got: {form:?}"
    );
}

// -----------------------------------------------------------------------
// goto $expr form (dynamic / computed target)
// -----------------------------------------------------------------------

#[test]
fn test_goto_scalar_variable_produces_expr_form() {
    // `goto $label_name;` — computed label target
    let form = find_goto_form(
        r#"
sub run { goto $target; }
"#,
    );
    assert!(
        matches!(form, GotoTargetForm::Expr),
        "goto $target should produce GotoTargetForm::Expr, got: {form:?}"
    );
}

// -----------------------------------------------------------------------
// goto $expr form — additional edge cases
// -----------------------------------------------------------------------

#[test]
fn test_goto_paren_expr_produces_expr_form() {
    // `goto +($cond ? $a : $b)` — prefix `+` forces Expr interpretation.
    // The unary `+` token is not BitwiseAnd or Identifier, so the form is Expr.
    let form = find_goto_form(
        r#"
sub run { goto +($flag ? $a : $b); }
"#,
    );
    assert!(
        matches!(form, GotoTargetForm::Expr),
        "goto +(expr) should produce GotoTargetForm::Expr, got: {form:?}"
    );
}

// -----------------------------------------------------------------------
// Regression: existing goto LABEL still parses cleanly
// -----------------------------------------------------------------------

#[test]
fn test_goto_label_still_parses_without_errors() {
    cpan_test_helpers::assert_clean_parse(
        r#"
DONE:
    goto DONE;
"#,
    );
}

#[test]
fn test_goto_sub_still_parses_without_errors() {
    cpan_test_helpers::assert_clean_parse(
        r#"
sub wrapper { goto &real_impl; }
"#,
    );
}

#[test]
fn test_goto_sub_qualified_still_parses_without_errors() {
    cpan_test_helpers::assert_clean_parse(
        r#"
sub wrapper { goto &Foo::Bar::real_impl; }
"#,
    );
}

// -----------------------------------------------------------------------
// Form distinction does not affect to_sexp output (regression guard)
// -----------------------------------------------------------------------

#[test]
fn test_goto_label_sexp_contains_goto() {
    let mut parser = Parser::new("goto DONE;");
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("goto"),
        "to_sexp() for goto LABEL should still contain 'goto', got: {sexp}"
    );
}

#[test]
fn test_goto_sub_sexp_contains_goto() {
    let mut parser = Parser::new("sub wrapper { goto &helper; }");
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("goto"),
        "to_sexp() for goto &sub should still contain 'goto', got: {sexp}"
    );
}
