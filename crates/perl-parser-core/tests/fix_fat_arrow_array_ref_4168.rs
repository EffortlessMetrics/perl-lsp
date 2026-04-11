mod cpan_test_helpers;
use cpan_test_helpers::{assert_clean_parse, parse};

// Tests for issue #4168: fat arrow (=>) inside array ref literals should be
// treated as list context (comma with autoquoting), not hash/block context.
//
// Valid Perl: `perl -e 'my $r = [foo => 1]; print $r->[0]'` prints "foo".
// The fat arrow acts as a comma and auto-quotes the bare identifier on the left.
//
// Before the fix, `[$k => $v]` was parsed as `[{$k => $v}]` — one element
// (a hash literal) instead of two elements ($k and $v).

/// Helper: count how many elements are in the outermost ArrayLiteral in the AST.
fn count_array_elements(source: &str) -> usize {
    use perl_parser_core::NodeKind;
    let ast = parse(source);
    // Walk: Program -> my_declaration -> rhs which should be ArrayLiteral
    let sexp = ast.to_sexp();
    // Count via sexp for simplicity: find the array node's direct children
    // We'll verify via sexp structure instead
    let _ = sexp;
    // Actually count via AST children
    fn find_array(node: &perl_parser_core::Node) -> Option<usize> {
        if let NodeKind::ArrayLiteral { elements } = &node.kind {
            return Some(elements.len());
        }
        for child in node.children() {
            if let Some(count) = find_array(child) {
                return Some(count);
            }
        }
        None
    }
    find_array(&ast).unwrap_or(0)
}

#[test]
fn test_fat_arrow_in_array_ref_simple() {
    // [$k => $v] — variable key, variable value
    // Should produce ArrayLiteral with 2 elements, not 1 hash element
    assert_clean_parse(r#"my $pair = [$k => $v];"#);
    assert_eq!(
        count_array_elements(r#"my $pair = [$k => $v];"#),
        2,
        "[$k => $v] should produce 2 elements in the array ref"
    );
}

#[test]
fn test_fat_arrow_in_array_ref_bareword_key() {
    // [key => 'val'] — bareword key (auto-quoted), should be 2 elements
    assert_clean_parse(r#"my $pair = [key => 'val'];"#);
    assert_eq!(
        count_array_elements(r#"my $pair = [key => 'val'];"#),
        2,
        "[key => 'val'] should produce 2 elements"
    );
}

#[test]
fn test_fat_arrow_in_array_ref_multiple_pairs() {
    // Multiple pairs: [key => 'val', other => 42] should be 4 elements
    assert_clean_parse(r#"my $arr = [key => 'val', other => 42];"#);
    assert_eq!(
        count_array_elements(r#"my $arr = [key => 'val', other => 42];"#),
        4,
        "[key => 'val', other => 42] should produce 4 elements"
    );
}

#[test]
fn test_fat_arrow_in_array_ref_string_key() {
    // String key with fat arrow
    assert_clean_parse(r#"my $pair = ['foo' => 'bar'];"#);
    assert_eq!(
        count_array_elements(r#"my $pair = ['foo' => 'bar'];"#),
        2,
        "['foo' => 'bar'] should produce 2 elements"
    );
}

#[test]
fn test_fat_arrow_in_array_ref_trailing_comma() {
    // Trailing comma after last pair
    assert_clean_parse(r#"my $arr = [key => 'val',];"#);
    assert_eq!(
        count_array_elements(r#"my $arr = [key => 'val',];"#),
        2,
        "[key => 'val',] should produce 2 elements"
    );
}

#[test]
fn test_fat_arrow_in_array_ref_push_context() {
    // Fat arrow inside array ref passed to a function
    assert_clean_parse(r#"push @list, [$key => $value];"#);
}

#[test]
fn test_fat_arrow_in_array_ref_assignment_chain() {
    // Array ref with fat arrow assigned to hash value
    assert_clean_parse(r#"$hash{key} = [method => sub { 1 }];"#);
}

#[test]
fn test_fat_arrow_in_array_ref_nested_hashref() {
    // Nested structures with fat arrow in array ref
    assert_clean_parse(r#"my $mixed = [{a => 1}, [x => 'y'], $scalar];"#);
}

#[test]
fn test_plain_array_ref_still_works() {
    // Regression: plain comma-separated array ref should still work
    assert_clean_parse(r#"my $arr = [1, 2, 3];"#);
    assert_eq!(count_array_elements(r#"my $arr = [1, 2, 3];"#), 3);
}

#[test]
fn test_plain_array_ref_trailing_comma() {
    assert_clean_parse(r#"my $arr = [1, 2, 3,];"#);
    assert_eq!(count_array_elements(r#"my $arr = [1, 2, 3,];"#), 3);
}
