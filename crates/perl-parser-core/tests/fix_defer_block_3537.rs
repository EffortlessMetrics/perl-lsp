mod cpan_test_helpers;
use cpan_test_helpers::*;

use perl_parser_core::NodeKind;

// Tests for `defer { }` block support (Perl 5.36+ experimental, stable in 5.40)
// Issue #3537: Missing lexer support for defer block

#[test]
fn test_defer_basic_block() {
    let source = r#"
use feature 'defer';
defer { cleanup(); };
"#;
    assert_clean_parse(source);
}

#[test]
fn test_defer_in_sub() {
    let source = r#"
use feature 'defer';
sub process {
    defer { cleanup(); };
    do_work();
}
"#;
    assert_clean_parse(source);
}

#[test]
fn test_defer_multiple_statements_in_block() {
    let source = r#"
use feature 'defer';
defer {
    close($fh);
    unlink($tmp);
    log_exit();
};
"#;
    assert_clean_parse(source);
}

#[test]
fn test_defer_nested() {
    let source = r#"
use feature 'defer';
sub outer {
    defer { outer_cleanup(); };
    sub inner_sub {
        defer { inner_cleanup(); };
        do_inner_work();
    }
    do_outer_work();
}
"#;
    assert_clean_parse(source);
}

#[test]
fn test_defer_node_kind_name() {
    let source = "use feature 'defer'; defer { cleanup(); };";
    let ast = parse(source);
    // Walk the AST to find the defer node
    let mut found_defer = false;
    fn walk(node: &perl_parser_core::Node, found: &mut bool) {
        if node.kind.kind_name() == "Defer" {
            *found = true;
        }
        node.for_each_child(|child| walk(child, found));
    }
    walk(&ast, &mut found_defer);
    assert!(
        found_defer,
        "Expected a Defer node in the AST, but none was found"
    );
}

#[test]
fn test_defer_sexp_output() {
    let source = "use feature 'defer'; defer { 1; };";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("(defer"),
        "Expected s-expression to contain '(defer ...)', got: {}",
        sexp
    );
}

#[test]
fn test_defer_not_a_function_call() {
    let source = "use feature 'defer'; defer { cleanup(); };";
    let ast = parse(source);
    // Walk and verify there's no FunctionCall named "defer"
    let mut found_defer_call = false;
    fn walk(node: &perl_parser_core::Node, found: &mut bool) {
        if let NodeKind::FunctionCall { name, .. } = &node.kind {
            if name == "defer" {
                *found = true;
            }
        }
        node.for_each_child(|child| walk(child, found));
    }
    walk(&ast, &mut found_defer_call);
    assert!(
        !found_defer_call,
        "defer should parse as NodeKind::Defer, not as FunctionCall"
    );
}
