// Regression tests for lvalue builtins called WITH parentheses used as
// assignment targets. Part of #752 (finding 1); resolves #751 Bug 3.
//
// Perl lvalue builtins that accept `= RHS` assignment:
//   pos($s)        = 0      (regex match position)
//   substr($s,0,5) = "x"   (string substitution)
//   vec($s,0,8)    = 0xFF  (bit manipulation)
//
// The no-parens forms (`pos $s = 0`, `substr $s,0,5 = "x"`) already work.
// The with-parens forms were previously broken for `pos` (ERROR) and
// silently wrong for `substr`/`vec` (assignment absorbed inside args).

mod cpan_test_helpers;
use cpan_test_helpers::*;
use perl_parser_core::{Node, NodeKind};

/// Extract the single top-level expression from a one-statement source.
fn first_expression(source: &str) -> Result<Node, Box<dyn std::error::Error>> {
    let ast = parse(source);
    let sexp = ast.to_sexp();
    let NodeKind::Program { mut statements } = ast.kind else {
        return Err(format!("expected Program, got {sexp}").into());
    };
    if statements.len() != 1 {
        return Err(format!("expected 1 statement, got {} in {sexp}", statements.len()).into());
    }
    let statement = statements.remove(0);
    let NodeKind::ExpressionStatement { expression } = statement.kind else {
        return Err(
            format!("expected ExpressionStatement, got {}", statement.kind.kind_name()).into()
        );
    };
    Ok(*expression)
}

/// Assert that `source` parses to:
///   Assignment { lhs: FunctionCall { name, args[arg_count] }, rhs: _, op }
fn assert_lvalue_builtin_assignment(
    source: &str,
    func_name: &str,
    expected_arg_count: usize,
    expected_op: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let expr = first_expression(source)?;
    let sexp = {
        let ast = parse(source);
        ast.to_sexp()
    };
    let NodeKind::Assignment { lhs, rhs: _, op } = expr.kind else {
        return Err(format!(
            "expected Assignment node at top level for `{source}`, \
                 got something else. sexp:\n{sexp}"
        )
        .into());
    };
    assert_eq!(op, expected_op, "wrong assignment operator in `{source}`");
    let NodeKind::FunctionCall { name, args } = lhs.kind else {
        return Err(format!(
            "expected FunctionCall lhs for `{source}`, got something else. sexp:\n{sexp}"
        )
        .into());
    };
    assert_eq!(name, func_name, "wrong function name in lhs for `{source}`");
    assert_eq!(args.len(), expected_arg_count, "wrong arg count in {name}() lhs for `{source}`");
    Ok(())
}

// ── pos($t) = 0 ──────────────────────────────────────────────────────────────

#[test]
fn test_pos_parens_assign_zero() -> Result<(), Box<dyn std::error::Error>> {
    // pos($t) = 0  — most common corpus pattern (JSON::PP, Text::Wrap, etc.)
    assert_lvalue_builtin_assignment(r"pos($text) = 0;", "pos", 1, "=")
}

#[test]
fn test_pos_parens_assign_expr() -> Result<(), Box<dyn std::error::Error>> {
    // pos($s) = $n - 1
    assert_lvalue_builtin_assignment(r"pos($s) = $n - 1;", "pos", 1, "=")
}

#[test]
fn test_pos_parens_augmented_assign() -> Result<(), Box<dyn std::error::Error>> {
    // pos($s) += 1
    assert_lvalue_builtin_assignment(r"pos($s) += 1;", "pos", 1, "+=")
}

// ── substr($s,0,5) = "x" ─────────────────────────────────────────────────────

#[test]
fn test_substr_parens_assign() -> Result<(), Box<dyn std::error::Error>> {
    // substr($s,0,5) = "x"  — the assignment must be OUTSIDE the call, not
    // silently absorbed as the 4th argument.
    assert_lvalue_builtin_assignment(r#"substr($s, 0, 5) = "x";"#, "substr", 3, "=")
}

#[test]
fn test_substr_parens_assign_two_arg() -> Result<(), Box<dyn std::error::Error>> {
    // substr($s, 0) = "x"  — two-arg form
    assert_lvalue_builtin_assignment(r#"substr($s, 0) = "x";"#, "substr", 2, "=")
}

// ── vec($s,0,8) = 0xFF ───────────────────────────────────────────────────────

#[test]
fn test_vec_parens_assign() -> Result<(), Box<dyn std::error::Error>> {
    // vec($s,0,8) = 0xFF  — the assignment must be OUTSIDE the call.
    assert_lvalue_builtin_assignment(r"vec($s, 0, 8) = 0xFF;", "vec", 3, "=")
}

#[test]
fn test_vec_parens_augmented_assign() -> Result<(), Box<dyn std::error::Error>> {
    // vec($s,0,8) |= 1
    assert_lvalue_builtin_assignment(r"vec($s, 0, 8) |= 1;", "vec", 3, "|=")
}

// ── Regression: no-parens forms still work ───────────────────────────────────

#[test]
fn test_pos_no_parens_assign_still_works() -> Result<(), Box<dyn std::error::Error>> {
    // `pos $t = 0` without parens must keep working
    assert_lvalue_builtin_assignment(r"pos $t = 0;", "pos", 1, "=")
}

#[test]
fn test_pos_parens_no_assignment_still_works() {
    // `pos($s)` without an assignment is still a clean parse
    assert_clean_parse(r"my $p = pos($s);");
    assert_clean_parse(r"pos($s);");
}

#[test]
fn test_substr_parens_no_assignment_still_works() {
    // `substr($s,0,3)` without an assignment is still a clean parse
    assert_clean_parse(r#"my $sub = substr($s, 0, 3);"#);
    assert_clean_parse(r"substr($s, 0, 3);");
}

#[test]
fn test_vec_no_assignment_still_works() {
    // `vec($s,0,8)` without an assignment is still a clean parse
    assert_clean_parse(r"my $v = vec($s, 0, 8);");
    assert_clean_parse(r"vec($s, 0, 8);");
}

// ── Non-lvalue builtins must NOT swallow `=` ─────────────────────────────────

#[test]
fn test_length_parens_not_lvalue() {
    // `length($x) = 5` is not valid Perl; the parser should either error or
    // leave `= 5` for outer context — crucially it must NOT produce a clean
    // assignment AST that claims `length($x)` is an lvalue.
    let ast = parse(r"length($x) = 5;");
    let sexp = ast.to_sexp();
    // Either an error node or the assignment is NOT "Assignment { lhs: length() }"
    if let NodeKind::Program { statements } = &ast.kind {
        for stmt in statements {
            if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
                if let NodeKind::Assignment { lhs, .. } = &expression.kind {
                    if let NodeKind::FunctionCall { name, .. } = &lhs.kind {
                        assert_ne!(
                            name.as_str(),
                            "length",
                            "length() must NOT be treated as an lvalue; sexp:\n{sexp}"
                        );
                    }
                }
            }
        }
    }
}

// ── User-defined function with parens: not affected ──────────────────────────

#[test]
fn test_user_defined_func_parens_not_affected() {
    // `foo($x) = $y` — a user-defined function call followed by `=` should
    // NOT be treated as a lvalue-builtin assignment.
    let ast = parse(r"foo($x) = $y;");
    let sexp = ast.to_sexp();
    if let NodeKind::Program { statements } = &ast.kind {
        for stmt in statements {
            if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
                if let NodeKind::Assignment { lhs, .. } = &expression.kind {
                    if let NodeKind::FunctionCall { name, .. } = &lhs.kind {
                        assert!(
                            !matches!(name.as_str(), "pos" | "substr" | "vec"),
                            "unexpected lvalue builtin treatment for user func; sexp:\n{sexp}"
                        );
                    }
                }
            }
        }
    }
}
