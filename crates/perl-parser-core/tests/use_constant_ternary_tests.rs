mod cpan_test_helpers;
use cpan_test_helpers::parse;

/// Strict assertion that catches all ERROR variants in sexp output,
/// including uppercase `(ERROR "...")` from error recovery.
fn assert_no_errors(source: &str) {
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
}

#[test]
fn test_use_constant_ternary_with_number_lhs() {
    // Number followed by ternary operator — the `?` must not be left orphaned
    assert_no_errors("use constant FOO => 1 ? 'a' : 'b';");
}

#[test]
fn test_use_constant_ternary_with_string_lhs() {
    // String followed by ternary — same root cause as number
    assert_no_errors("use constant FOO => 'yes' ? 1 : 0;");
}

#[test]
fn test_use_constant_number_with_binary_op() {
    // Number followed by arithmetic operator
    assert_no_errors("use constant BAR => 1 + 2;");
}

#[test]
fn test_use_constant_string_with_concat_op() {
    // String followed by concatenation
    assert_no_errors("use constant BAZ => 'hello' . ' world';");
}

#[test]
fn test_use_constant_number_comparison() {
    // Number followed by comparison
    assert_no_errors("use constant OLD => 5.008 < 5.016;");
}

#[test]
fn test_use_constant_ternary_nested() {
    // Nested ternary in use constant value
    assert_no_errors("use constant X => 1 ? 2 ? 'a' : 'b' : 'c';");
}
