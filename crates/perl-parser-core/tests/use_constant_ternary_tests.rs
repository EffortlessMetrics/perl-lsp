mod cpan_test_helpers;
use cpan_test_helpers::parse;

/// Strict assertion that catches all ERROR variants in sexp output,
/// including uppercase `(ERROR "...")` from error recovery.
fn assert_use_args(source: &str, expected_use: &str) {
    let ast = parse(source);
    let sexp = ast.to_sexp();

    let error_markers = [
        "(error ",
        "(Error ",
        "(ERROR ",
        "(missing_expression",
        "(missing_statement",
        "(missing_identifier",
        "(missing_block",
        "MissingExpression",
        "MissingStatement",
        "MissingIdentifier",
        "MissingBlock",
    ];

    for marker in &error_markers {
        assert!(
            !sexp.contains(marker),
            "Parse produced error for source:\n{}\n\nMarker: {}\nSexp: {}",
            source,
            marker,
            sexp,
        );
    }

    assert!(
        sexp.contains(expected_use),
        "Expected use clause for source:\n{}\n\nExpected fragment: {}\nSexp: {}",
        source,
        expected_use,
        sexp,
    );
}

#[test]
fn test_use_constant_ternary_with_number_lhs() {
    // Number followed by ternary operator — the `?` must not be left orphaned
    assert_use_args("use constant FOO => 1 ? 'a' : 'b';", "(use constant (FOO 1 ? 'a' : 'b'))");
}

#[test]
fn test_use_constant_ternary_with_string_lhs() {
    // String followed by ternary — same root cause as number
    assert_use_args("use constant FOO => 'yes' ? 1 : 0;", "(use constant (FOO 'yes' ? 1 : 0))");
}

#[test]
fn test_use_constant_number_with_binary_op() {
    // Number followed by arithmetic operator
    assert_use_args("use constant BAR => 1 + 2;", "(use constant (BAR 1 + 2))");
}

#[test]
fn test_use_constant_string_with_concat_op() {
    // String followed by concatenation
    assert_use_args(
        "use constant BAZ => 'hello' . ' world';",
        "(use constant (BAZ 'hello' . ' world'))",
    );
}

#[test]
fn test_use_constant_number_comparison() {
    // Number followed by comparison
    assert_use_args("use constant OLD => 5.008 < 5.016;", "(use constant (OLD 5.008 < 5.016))");
}

#[test]
fn test_use_constant_ternary_nested() {
    // Nested ternary in use constant value
    assert_use_args(
        "use constant X => 1 ? 2 ? 'a' : 'b' : 'c';",
        "(use constant (X 1 ? 2 ? 'a' : 'b' : 'c'))",
    );
}

#[test]
fn test_use_constant_ternary_with_nested_fat_arrows() {
    assert_use_args(
        "use constant MAP => 1 ? { foo => 'a' } : { bar => 'b' };",
        "(use constant (MAP 1 ? { foo => 'a' } : { bar => 'b' }))",
    );
}

// --- Tests for issue #1895: ternary with variable/expression conditions ---

#[test]
fn test_use_constant_ternary_variable_condition() {
    // Variable as ternary condition — the core issue #1895 pattern
    let source = r#"use constant FOO => $bar ? 1 : 0;"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();
    for marker in &["(error ", "(Error ", "(ERROR ", "MissingExpression", "MissingStatement"] {
        assert!(
            !sexp.contains(marker),
            "Parse produced error for variable ternary:\n{}\nSexp: {}",
            source,
            sexp,
        );
    }
}

#[test]
fn test_use_constant_ternary_env_hash_condition() {
    let source = r#"use constant MODE => $ENV{DEBUG} ? 'debug' : 'release';"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();
    for marker in &["(error ", "(Error ", "(ERROR ", "MissingExpression", "MissingStatement"] {
        assert!(
            !sexp.contains(marker),
            "Parse produced error for ENV ternary:\n{}\nSexp: {}",
            source,
            sexp,
        );
    }
}

#[test]
fn test_use_constant_ternary_perl_version_condition() {
    let source = r#"use constant HAS_FEATURE => $] >= 5.010 ? 1 : 0;"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();
    for marker in &["(error ", "(Error ", "(ERROR ", "MissingExpression", "MissingStatement"] {
        assert!(
            !sexp.contains(marker),
            "Parse produced error for version ternary:\n{}\nSexp: {}",
            source,
            sexp,
        );
    }
}

#[test]
fn test_use_constant_ternary_eval_condition() {
    let source = r#"use constant CAN_DO => eval { require Some::Module; 1 } ? 1 : 0;"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();
    for marker in &["(error ", "(Error ", "(ERROR ", "MissingExpression", "MissingStatement"] {
        assert!(
            !sexp.contains(marker),
            "Parse produced error for eval ternary:\n{}\nSexp: {}",
            source,
            sexp,
        );
    }
}

#[test]
fn test_use_constant_ternary_defined_or_condition() {
    let source = r#"use constant VAL => defined($x) ? $x : 'default';"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();
    for marker in &["(error ", "(Error ", "(ERROR ", "MissingExpression", "MissingStatement"] {
        assert!(
            !sexp.contains(marker),
            "Parse produced error for defined-or ternary:\n{}\nSexp: {}",
            source,
            sexp,
        );
    }
}
