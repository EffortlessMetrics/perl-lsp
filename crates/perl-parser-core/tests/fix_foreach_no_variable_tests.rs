mod cpan_test_helpers;
use cpan_test_helpers::*;

/// Strict clean parse check that also catches `(ERROR ...)` nodes.
fn assert_strict_clean_parse(source: &str) {
    let ast = parse(source);
    let sexp = ast.to_sexp();
    let markers = [
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
    for marker in &markers {
        assert!(
            !sexp.contains(marker),
            "Clean-parse assertion failed: found '{}' in sexp for source:\n{}\n\nsexp:\n{}",
            marker,
            source,
            sexp,
        );
    }
}

// --- foreach with variable declarators ---

#[test]
fn test_foreach_our_variable() {
    assert_strict_clean_parse("foreach our $item (@list) { print $item; }");
}

#[test]
fn test_for_our_variable() {
    assert_strict_clean_parse("for our $item (@list) { print $item; }");
}

// --- foreach/for with implicit $_ ---

#[test]
fn test_foreach_implicit_topic() {
    assert_strict_clean_parse("foreach (@list) { print; }");
}

#[test]
fn test_for_implicit_topic() {
    assert_strict_clean_parse("for (@array) { print; }");
}

// --- foreach/for with bare scalar ---

#[test]
fn test_foreach_bare_scalar() {
    assert_strict_clean_parse("foreach $item (@list) { print $item; }");
}

#[test]
fn test_for_bare_scalar() {
    assert_strict_clean_parse("for $item (@list) { print $item; }");
}

// --- foreach/for my (standard) ---

#[test]
fn test_foreach_my_variable() {
    assert_strict_clean_parse("foreach my $item (@list) { print $item; }");
}

#[test]
fn test_for_my_variable() {
    assert_strict_clean_parse("for my $item (@list) { print $item; }");
}

// --- nested, continue, complex lists ---

#[test]
fn test_foreach_nested_implicit() {
    assert_strict_clean_parse("for (@outer) { for (@inner) { print; } }");
}

#[test]
fn test_foreach_with_continue() {
    assert_strict_clean_parse(
        r#"foreach my $item (@list) { print $item; } continue { print "next\n"; }"#,
    );
}
