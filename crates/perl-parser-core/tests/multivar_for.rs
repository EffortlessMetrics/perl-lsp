/// Tests for Perl 5.36 multi-variable `for` loops.
///
/// `for my ($x, $y) (@list) { }` and `foreach my ($k, $v) (%h) { }` are
/// valid Perl 5.36 syntax. The parser must accept them without producing any
/// Error / Missing* nodes.
///
/// Actual sexp output (verified with current parser):
///   for my ($x, $y) (@list) { }
///   → (source_file (foreach (my_declaration ((variable $ x) (variable $ y)))
///                            (variable @ list) (block )))
///
///   foreach my ($k, $v) (%h) { }
///   → (source_file (foreach (my_declaration ((variable $ k) (variable $ v)))
///                            (variable % h) (block )))
use perl_parser_core::Parser;
use perl_tdd_support::must;

// --- helpers ---

/// Return the sexp of `src` and assert no error/missing nodes.
fn sexp_clean(src: &str) -> String {
    let mut parser = Parser::new(src);
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();
    let sexp_lower = sexp.to_lowercase();
    assert!(
        !sexp_lower.contains("error") && !sexp_lower.contains("missing"),
        "Expected clean parse for:\n  {}\nbut got sexp:\n  {}\nerrors: {:?}",
        src,
        sexp,
        parser.get_errors(),
    );
    sexp
}

/// Count how many Foreach nodes are in the sexp.
fn count_foreach_in_sexp(sexp: &str) -> usize {
    sexp.matches("(foreach ").count()
}

// ─── multi-variable for loops (Perl 5.36) ────────────────────────────────────

#[test]
fn test_for_my_two_vars_parses_without_error() -> Result<(), Box<dyn std::error::Error>> {
    // for my ($x, $y) (@list) { }
    let sexp = sexp_clean("for my ($x, $y) (@list) { }");
    // Must produce a foreach node, not a c-style for or error
    assert!(count_foreach_in_sexp(&sexp) >= 1, "Expected foreach node, got: {}", sexp);
    Ok(())
}

#[test]
fn test_for_my_two_vars_contains_loop_vars() -> Result<(), Box<dyn std::error::Error>> {
    // Both $x and $y must appear in the sexp
    // Sexp format: (my_declaration ((variable $ x) (variable $ y)))
    let sexp = sexp_clean("for my ($x, $y) (@list) { }");
    assert!(sexp.contains("x"), "Expected 'x' in sexp: {}", sexp);
    assert!(sexp.contains("y"), "Expected 'y' in sexp: {}", sexp);
    // Must be a list declaration (parenthesized vars), not two separate Foreach nodes
    assert!(sexp.contains("my_declaration"), "Expected my_declaration in sexp: {}", sexp,);
    Ok(())
}

#[test]
fn test_for_my_three_vars_parses_without_error() -> Result<(), Box<dyn std::error::Error>> {
    // for my ($a, $b, $c) (some_func()) { }
    let sexp = sexp_clean("for my ($a, $b, $c) (some_func()) { }");
    assert!(count_foreach_in_sexp(&sexp) >= 1, "Expected foreach node, got: {}", sexp);
    // All three variable names must appear
    assert!(sexp.contains(" a"), "Expected 'a' in sexp: {}", sexp);
    assert!(sexp.contains(" b"), "Expected 'b' in sexp: {}", sexp);
    assert!(sexp.contains(" c"), "Expected 'c' in sexp: {}", sexp);
    Ok(())
}

#[test]
fn test_foreach_my_two_vars_parses_without_error() -> Result<(), Box<dyn std::error::Error>> {
    // foreach my ($k, $v) (%h) { }
    let sexp = sexp_clean("foreach my ($k, $v) (%h) { }");
    assert!(count_foreach_in_sexp(&sexp) >= 1, "Expected foreach node, got: {}", sexp);
    assert!(sexp.contains("k"), "Expected 'k' in sexp: {}", sexp);
    assert!(sexp.contains("v"), "Expected 'v' in sexp: {}", sexp);
    Ok(())
}

#[test]
fn test_for_my_two_vars_with_body() -> Result<(), Box<dyn std::error::Error>> {
    // for my ($x, $y) (@list) { print "$x=$y\n"; }
    sexp_clean(r#"for my ($x, $y) (@list) { print "$x=$y\n"; }"#);
    Ok(())
}

// ─── regression: single-variable forms must be unchanged ─────────────────────

#[test]
fn test_regression_for_my_single_var() -> Result<(), Box<dyn std::error::Error>> {
    // for my $x (@list) {}  — classic single-var foreach, must still work
    let sexp = sexp_clean("for my $x (@list) {}");
    assert!(count_foreach_in_sexp(&sexp) >= 1, "Expected foreach node, got: {}", sexp);
    Ok(())
}

#[test]
fn test_regression_foreach_my_single_var() -> Result<(), Box<dyn std::error::Error>> {
    sexp_clean("foreach my $item (@list) { print $item; }");
    Ok(())
}

#[test]
fn test_regression_c_style_for() -> Result<(), Box<dyn std::error::Error>> {
    // C-style for (my $i=0; $i<10; $i++) {} — must NOT be treated as foreach
    let mut parser = Parser::new("for (my $i=0; $i<10; $i++) {}");
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();
    let sexp_lower = sexp.to_lowercase();
    assert!(
        !sexp_lower.contains("error") && !sexp_lower.contains("missing"),
        "C-style for produced errors: {}\nerrors: {:?}",
        sexp,
        parser.get_errors(),
    );
    // Must produce a For (C-style) node (contains "(for " not starting with "(foreach ")
    // The sexp wraps in source_file, so we check the inner structure
    assert!(sexp.contains("(for "), "Expected C-style for node, got: {}", sexp,);
    Ok(())
}

#[test]
fn test_regression_for_implicit_topic() -> Result<(), Box<dyn std::error::Error>> {
    // for (@list) {}  — implicit $_ foreach
    let sexp = sexp_clean("for (@list) {}");
    assert!(count_foreach_in_sexp(&sexp) >= 1, "Expected foreach (implicit) node, got: {}", sexp);
    Ok(())
}
