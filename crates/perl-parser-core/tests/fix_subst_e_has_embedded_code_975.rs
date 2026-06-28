/// Regression tests for #975: s///e and s///ee must set has_embedded_code = true.
///
/// The `e` modifier evaluates the replacement as Perl code (equivalent to `eval`),
/// so the substitution has embedded code regardless of what is in the pattern body.
///
/// Run with: cargo test -p perl-parser-core -- subst_e_has_embedded_code
use perl_parser_core::{NodeKind, Parser};
use perl_tdd_support::must;

/// Walk an AST and find the first Substitution node.
fn find_first_substitution(node: &perl_parser_core::Node) -> Option<(bool, String)> {
    if let NodeKind::Substitution { has_embedded_code, modifiers, .. } = &node.kind {
        return Some((*has_embedded_code, modifiers.clone()));
    }
    for child in node.children() {
        if let Some(result) = find_first_substitution(child) {
            return Some(result);
        }
    }
    None
}

fn parse_subst(source: &str) -> perl_parser_core::Node {
    let mut parser = Parser::new(source);
    must(parser.parse())
}

// ── Site 1: primary.rs path (s/// as standalone expression with =~) ──────────

/// `s/(\w+)/uc($1)/e` — single `e` modifier must set has_embedded_code=true.
#[test]
fn subst_e_modifier_sets_has_embedded_code() {
    let ast = parse_subst(r#"$s =~ s/(\w+)/uc($1)/e;"#);
    let (embedded, mods) = find_first_substitution(&ast).expect("should find a Substitution node");
    assert!(embedded, "s///e must set has_embedded_code=true (modifiers={:?})", mods);
}

/// `s///ee` — double-eval form must also set has_embedded_code=true.
#[test]
fn subst_ee_modifier_sets_has_embedded_code() {
    let ast = parse_subst(r#"$t =~ s/\$(\w+)/$$1/ee;"#);
    let (embedded, mods) = find_first_substitution(&ast).expect("should find a Substitution node");
    assert!(embedded, "s///ee must set has_embedded_code=true (modifiers={:?})", mods);
}

/// `s/a/b/ge` — combined modifiers: has_embedded_code must be true.
#[test]
fn subst_ge_modifier_sets_has_embedded_code() {
    let ast = parse_subst(r#"$s =~ s/a/b/ge;"#);
    let (embedded, mods) = find_first_substitution(&ast).expect("should find a Substitution node");
    assert!(embedded, "s///ge must set has_embedded_code=true (modifiers={:?})", mods);
}

/// `s/a/b/gr` — no `e` modifier: has_embedded_code must remain false (regression guard).
#[test]
fn subst_no_e_modifier_does_not_set_has_embedded_code() {
    let ast = parse_subst(r#"$s =~ s/a/b/gr;"#);
    let (embedded, mods) = find_first_substitution(&ast).expect("should find a Substitution node");
    assert!(!embedded, "s///gr must NOT set has_embedded_code (modifiers={:?})", mods);
}

/// `s/(?{1+1})/b/g` — `(?{...})` in pattern with no `e`: has_embedded_code=true
/// via the existing pattern-body path (unchanged behavior, regression guard).
#[test]
fn subst_embedded_code_in_pattern_stays_true() {
    let ast = parse_subst(r#"$s =~ s/(?{1+1})/b/g;"#);
    let (embedded, mods) = find_first_substitution(&ast).expect("should find a Substitution node");
    assert!(
        embedded,
        "s///g with (?{{...}}) in pattern must keep has_embedded_code=true (modifiers={:?})",
        mods
    );
}

// ── Site 2: quotes.rs path (s{}{} form — no =~) ──────────────────────────────

/// `s{(\w+)}{uc($1)}e` — brace-delimited form exercises the quotes.rs code path.
#[test]
fn subst_quote_operator_form_e_sets_has_embedded_code() {
    let ast = parse_subst(r#"s{(\w+)}{uc($1)}e;"#);
    let (embedded, mods) = find_first_substitution(&ast).expect("should find a Substitution node");
    assert!(
        embedded,
        "s{{}}{{}}e (quote-operator form) must set has_embedded_code=true (modifiers={:?})",
        mods
    );
}

// ── Sexp marker test ─────────────────────────────────────────────────────────

/// The `(risk:code)` marker must appear in the sexp for `s///e`.
#[test]
fn subst_e_sexp_contains_risk_code_marker() {
    let mut parser = Parser::new(r#"$s =~ s/(\w+)/uc($1)/e;"#);
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("risk:code"),
        "sexp for s///e must contain '(risk:code)' marker; got:\n{}",
        sexp
    );
}

// ── Site 2 supplemental: quotes.rs false branch and both-true path ───────────

/// `s{a}{b}g` — brace-delimited form with NO `e` modifier: has_embedded_code must be false.
/// This covers the `modifiers.contains('e')` → false branch in quotes.rs.
#[test]
fn subst_quote_operator_form_no_e_does_not_set_has_embedded_code() {
    let ast = parse_subst(r#"s{a}{b}g;"#);
    let (embedded, mods) = find_first_substitution(&ast).expect("should find a Substitution node");
    assert!(
        !embedded,
        "s{{}}{{}}g (quote-operator, no e modifier) must NOT set has_embedded_code (modifiers={:?})",
        mods
    );
}

/// `s{a}{b}ee` — brace-delimited form with `ee` double-eval modifier: has_embedded_code must be true.
/// This covers the `ee` path in quotes.rs (modifiers.contains('e') is true for 'ee').
#[test]
fn subst_quote_operator_form_ee_sets_has_embedded_code() {
    let ast = parse_subst(r#"s{a}{b}ee;"#);
    let (embedded, mods) = find_first_substitution(&ast).expect("should find a Substitution node");
    assert!(
        embedded,
        "s{{}}{{}}ee (quote-operator, double-eval) must set has_embedded_code=true (modifiers={:?})",
        mods
    );
}

/// `s{(?{1+1})}{b}e` — both `(?{...})` in pattern body AND `e` modifier active simultaneously.
/// Verifies the `||` does not short-circuit incorrectly when both sides are true (quotes.rs path).
#[test]
fn subst_quote_operator_form_both_embedded_code_and_e_modifier() {
    let ast = parse_subst(r#"s{(?{1+1})}{b}e;"#);
    let (embedded, mods) = find_first_substitution(&ast).expect("should find a Substitution node");
    assert!(
        embedded,
        "s{{(?{{...}})}}{{}}e (both pattern body and e modifier) must set has_embedded_code=true (modifiers={:?})",
        mods
    );
}

/// `find_first_substitution` returns `None` when the AST has no Substitution node.
/// Pins the `None` branch of the helper: a non-substitution expression must yield None.
/// RIPR seam: `find_first_substitution` return_value None at line 20.
#[test]
fn find_first_substitution_returns_none_for_non_subst_ast() {
    // Parse a plain assignment — no s/// anywhere in the AST.
    let ast = parse_subst(r#"my $x = 42;"#);
    assert_eq!(
        find_first_substitution(&ast),
        None,
        "find_first_substitution must return None for an AST with no Substitution node"
    );
}
