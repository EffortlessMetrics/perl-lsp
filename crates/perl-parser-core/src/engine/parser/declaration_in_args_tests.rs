//! Tests for variable declarations (my/our/local/state) inside function call
//! argument lists.

use super::*;

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// Helper: parse code and return the full AST.
    fn parse_program(code: &str) -> Node {
        let mut parser = Parser::new(code);
        parser.parse().unwrap_or_else(|e| panic!("Parse failed for `{code}`: {e:?}"))
    }

    /// Helper: assert that the AST sexp contains no ERROR nodes.
    fn assert_no_errors(code: &str) {
        let ast = parse_program(code);
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "Parse of `{code}` produced ERROR nodes: {sexp}",);
    }

    /// Helper: parse code and return the first statement node.
    fn first_stmt(code: &str) -> Node {
        let ast = parse_program(code);
        match ast.kind {
            NodeKind::Program { mut statements } if !statements.is_empty() => {
                statements.swap_remove(0)
            }
            _ => panic!("Expected Program with statements, got: {}", ast.to_sexp()),
        }
    }

    // ---------------------------------------------------------------
    // Basic: `foo(my $x, $y)`
    // ---------------------------------------------------------------
    #[test]
    fn my_scalar_in_call_args() {
        let code = "foo(my $x, $y);";
        assert_no_errors(code);

        let stmt = first_stmt(code);
        let sexp = stmt.to_sexp();
        // Sexp format uses `my_declaration` for VariableDeclaration with declarator "my"
        assert!(sexp.contains("my_declaration"), "Expected my_declaration in sexp, got: {sexp}",);
    }

    // ---------------------------------------------------------------
    // With initializer: `sort(my @arr = @data)`
    // ---------------------------------------------------------------
    #[test]
    fn my_array_with_initializer_in_call_args() {
        let code = "sort(my @arr = @data);";
        assert_no_errors(code);

        let stmt = first_stmt(code);
        let sexp = stmt.to_sexp();
        assert!(sexp.contains("my_declaration"), "Expected my_declaration in sexp, got: {sexp}",);
    }

    // ---------------------------------------------------------------
    // Multiple args with declaration: `push(my @arr, 1, 2, 3)`
    // ---------------------------------------------------------------
    #[test]
    fn my_array_with_trailing_args() {
        let code = "push(my @arr, 1, 2, 3);";
        assert_no_errors(code);

        let stmt = first_stmt(code);
        let sexp = stmt.to_sexp();
        assert!(sexp.contains("my_declaration"), "Expected my_declaration in sexp, got: {sexp}",);
        // Verify that 1, 2, 3 are separate args, not consumed into the declaration
        assert!(
            sexp.contains("1") && sexp.contains("2") && sexp.contains("3"),
            "Expected numeric args 1, 2, 3 in sexp, got: {sexp}",
        );
    }

    // ---------------------------------------------------------------
    // our keyword: `print(our $fh, "hello")`
    // ---------------------------------------------------------------
    #[test]
    fn our_scalar_in_call_args() {
        let code = "print(our $fh, \"hello\");";
        assert_no_errors(code);

        let stmt = first_stmt(code);
        let sexp = stmt.to_sexp();
        assert!(sexp.contains("our_declaration"), "Expected our_declaration in sexp, got: {sexp}",);
    }

    // ---------------------------------------------------------------
    // state keyword: `foo(state $count = 0)`
    // ---------------------------------------------------------------
    #[test]
    fn state_scalar_with_initializer() {
        let code = "foo(state $count = 0);";
        assert_no_errors(code);

        let stmt = first_stmt(code);
        let sexp = stmt.to_sexp();
        assert!(
            sexp.contains("state_declaration"),
            "Expected state_declaration in sexp, got: {sexp}",
        );
    }

    // ---------------------------------------------------------------
    // local keyword: `foo(local $var)`
    // ---------------------------------------------------------------
    #[test]
    fn local_scalar_in_call_args() {
        let code = "foo(local $var);";
        assert_no_errors(code);

        let stmt = first_stmt(code);
        let sexp = stmt.to_sexp();
        assert!(
            sexp.contains("local_declaration"),
            "Expected local_declaration in sexp, got: {sexp}",
        );
    }

    #[test]
    fn local_parenthesized_lvalue_in_call_args() {
        let code = "foo(local($ENV{PATH}) = '/tmp/bin', $next);";
        assert_no_errors(code);

        let stmt = first_stmt(code);
        let sexp = stmt.to_sexp();
        assert!(
            sexp.contains("local_declaration"),
            "Expected local_declaration in sexp, got: {sexp}",
        );
        assert!(
            sexp.contains("PATH"),
            "Expected localized hash element key in sexp, got: {sexp}",
        );
        assert!(
            sexp.contains("(variable $ next)"),
            "Expected trailing argument to stay separate, got: {sexp}",
        );
    }

    // ---------------------------------------------------------------
    // Declaration with initializer and trailing args:
    // `foo(my $x = 1, $y)`
    // ---------------------------------------------------------------
    #[test]
    fn my_scalar_initializer_then_more_args() {
        let code = "foo(my $x = 1, $y);";
        assert_no_errors(code);

        let stmt = first_stmt(code);
        let sexp = stmt.to_sexp();
        assert!(sexp.contains("my_declaration"), "Expected my_declaration in sexp, got: {sexp}",);
        // $y must be a separate argument, not part of the declaration initializer
        // The sexp should contain both the declaration and a separate variable for $y
        assert!(
            sexp.contains("(variable $ y)"),
            "Expected separate (variable $ y) in sexp, got: {sexp}",
        );
    }

    // ---------------------------------------------------------------
    // List declaration in args: `foo(my ($a, $b) = @_)`
    // ---------------------------------------------------------------
    #[test]
    fn my_list_declaration_in_call_args() {
        let code = "foo(my ($a, $b) = @_);";
        assert_no_errors(code);

        let stmt = first_stmt(code);
        let sexp = stmt.to_sexp();
        // VariableListDeclaration sexp format should contain the declarator
        assert!(sexp.contains("my"), "Expected 'my' in sexp, got: {sexp}",);
        assert!(sexp.contains("(variable $ a)"), "Expected variable $a in sexp, got: {sexp}",);
        assert!(sexp.contains("(variable $ b)"), "Expected variable $b in sexp, got: {sexp}",);
    }
}
