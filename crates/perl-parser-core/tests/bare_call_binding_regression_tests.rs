mod cpan_test_helpers;

use cpan_test_helpers::{assert_clean_parse, parse};
use perl_parser_core::{Node, NodeKind};

fn program_statements(source: &str) -> Result<Vec<Node>, String> {
    let ast = parse(source);
    match ast.kind {
        NodeKind::Program { statements } => Ok(statements),
        other => Err(format!("expected Program node, got {other:?}")),
    }
}

fn statement_at<'a>(
    statements: &'a [Node],
    index: usize,
    context: &str,
) -> Result<&'a Node, String> {
    statements.get(index).ok_or_else(|| format!("expected statement {index} for {context}"))
}

fn expression_statement<'a>(statement: &'a Node, context: &str) -> Result<&'a Node, String> {
    let NodeKind::ExpressionStatement { expression } = &statement.kind else {
        return Err(format!(
            "expected ExpressionStatement for {context}, got {:?}",
            statement.kind
        ));
    };
    Ok(expression.as_ref())
}

fn variable_initializer<'a>(statement: &'a Node, context: &str) -> Result<&'a Node, String> {
    let NodeKind::VariableDeclaration { initializer, .. } = &statement.kind else {
        return Err(format!(
            "expected VariableDeclaration for {context}, got {:?}",
            statement.kind
        ));
    };
    initializer.as_deref().ok_or_else(|| format!("expected declaration initializer for {context}"))
}

fn function_call<'a>(node: &'a Node, context: &str) -> Result<(&'a str, &'a [Node]), String> {
    let NodeKind::FunctionCall { name, args } = &node.kind else {
        return Err(format!("expected FunctionCall for {context}, got {:?}", node.kind));
    };
    Ok((name.as_str(), args))
}

#[test]
fn bare_call_condition_stays_outside_ternary() -> Result<(), String> {
    let source = "sub is_ready { 1 }; my $x = is_ready $obj ? 1 : 0;";
    assert_clean_parse(source);

    let statements = program_statements(source)?;
    assert!(statements.len() >= 2, "expected at least 2 statements");

    let second = statement_at(&statements, 1, "bare call ternary assignment")?;
    let initializer = variable_initializer(second, "bare call ternary assignment")?;

    match &initializer.kind {
        NodeKind::Ternary { condition, .. } => match &condition.kind {
            NodeKind::FunctionCall { name, args } => {
                assert_eq!(name, "is_ready");
                assert_eq!(args.len(), 1, "expected one bare-call argument");
            }
            other => return Err(format!("expected FunctionCall condition, got {other:?}")),
        },
        other => return Err(format!("expected Ternary initializer, got {other:?}")),
    }
    Ok(())
}

#[test]
fn bare_call_stops_before_word_or_rhs() -> Result<(), String> {
    let source = "do_thing @args or die;";
    assert_clean_parse(source);

    let statements = program_statements(source)?;
    let expr = expression_statement(
        statement_at(&statements, 0, "bare call or expression")?,
        "bare call or expression",
    )?;

    match &expr.kind {
        NodeKind::Binary { op, left, right } => {
            assert_eq!(op, "or");
            match &right.kind {
                NodeKind::FunctionCall { name, .. } => assert_eq!(name, "die"),
                other => return Err(format!("expected die FunctionCall rhs, got {other:?}")),
            }
            match &left.kind {
                NodeKind::FunctionCall { name, args } => {
                    assert_eq!(name, "do_thing");
                    assert_eq!(args.len(), 1);
                }
                other => return Err(format!("expected left FunctionCall, got {other:?}")),
            }
        }
        other => return Err(format!("expected Binary(or), got {other:?}")),
    }
    Ok(())
}

#[test]
fn bare_call_stops_before_word_and_rhs() -> Result<(), String> {
    let source = "do_thing $x and return;";
    assert_clean_parse(source);

    let statements = program_statements(source)?;
    let expr = expression_statement(
        statement_at(&statements, 0, "bare call and expression")?,
        "bare call and expression",
    )?;

    match &expr.kind {
        NodeKind::Binary { op, left, right } => {
            assert_eq!(op, "and");
            assert!(matches!(right.kind, NodeKind::Return { .. }));
            assert!(matches!(left.kind, NodeKind::FunctionCall { .. }));
        }
        other => return Err(format!("expected Binary(and), got {other:?}")),
    }
    Ok(())
}

#[test]
fn bare_call_stops_before_defined_or() -> Result<(), String> {
    let source = "my $v = transform $x // $fallback;";
    assert_clean_parse(source);

    let statements = program_statements(source)?;
    let initializer = variable_initializer(
        statement_at(&statements, 0, "defined-or assignment")?,
        "defined-or assignment",
    )?;

    match &initializer.kind {
        NodeKind::Binary { op, left, .. } => {
            assert_eq!(op, "//");
            assert!(matches!(left.kind, NodeKind::FunctionCall { .. }));
        }
        other => return Err(format!("expected Binary(//), got {other:?}")),
    }
    Ok(())
}

#[test]
fn nested_bare_call_ternary_inside_larger_expression() -> Result<(), String> {
    let source = "my $z = 1 + (is_ready $obj ? 1 : 0);";
    assert_clean_parse(source);

    let statements = program_statements(source)?;
    let initializer = variable_initializer(
        statement_at(&statements, 0, "nested ternary assignment")?,
        "nested ternary assignment",
    )?;

    match &initializer.kind {
        NodeKind::Binary { op, right, .. } => {
            assert_eq!(op, "+");
            match &right.kind {
                NodeKind::Ternary { condition, .. } => {
                    assert!(matches!(condition.kind, NodeKind::FunctionCall { .. }));
                }
                other => return Err(format!("expected ternary rhs, got {other:?}")),
            }
        }
        other => return Err(format!("expected Binary(+), got {other:?}")),
    }
    Ok(())
}

/// When a sort (or other builtin) is immediately followed by `?` it should parse
/// cleanly as `sort()` (no-arg call) and the ternary should apply to the result,
/// rather than blowing up with a MissingExpression / parse error.
#[test]
fn builtin_directly_before_ternary_no_args() {
    // sort with no args followed by ternary: should parse without errors
    let source = "my @x = sort ? @a : @b;";
    // We only require a clean parse (no error nodes). The exact AST shape is
    // implementation-defined but must not contain Error / MissingExpression nodes.
    assert_clean_parse(source);
}

/// Verify that the block-list function (grep) correctly absorbs the ternary
/// as an argument when @arr appears before the ternary, matching Perl semantics:
///   grep { ... } @arr ? 1 : 0
/// must parse as: grep(BLOCK, @arr ? 1 : 0)  — ternary binds tighter than list op
#[test]
fn block_list_func_absorbs_ternary_arg_after_array() -> Result<(), String> {
    let source = "my $r = grep { $_ > 0 } @arr ? 1 : 0;";
    assert_clean_parse(source);
    let statements = program_statements(source)?;
    let initializer = variable_initializer(
        statement_at(&statements, 0, "grep ternary assignment")?,
        "grep ternary assignment",
    )?;
    // Correct Perl semantics: ternary binds tighter than list operators, so
    // grep collects the ternary expression as its list argument.
    let (name, args) = function_call(initializer, "grep initializer")?;
    assert_eq!(name, "grep");
    assert_eq!(args.len(), 2, "expected block + one list arg");
    let second_arg = args.get(1).ok_or_else(|| "expected grep ternary list arg".to_string())?;
    assert!(
        matches!(second_arg.kind, NodeKind::Ternary { .. }),
        "expected Ternary second arg, got {:?}",
        second_arg.kind.kind_name()
    );
    Ok(())
}

/// When `?` follows immediately after a block (no list argument in between),
/// `should_continue_bare_call_after_block` must NOT treat `?` as a continuation.
///   my $r = do { 1 } ? "yes" : "no";
/// must parse cleanly — the `?` starts the ternary over the block's value.
#[test]
fn bare_block_directly_before_ternary_no_error() {
    let source = "my $r = do { 1 } ? \"yes\" : \"no\";";
    assert_clean_parse(source);
}

/// Nested ternary in the bare-call condition — no parens.
///   foo $x ? $a ? $b : $c : $d
/// must parse as: (foo $x) ? ($a ? $b : $c) : $d
/// i.e. the outer ternary's condition is FunctionCall(foo, [$x]).
#[test]
fn bare_call_nested_ternary_outside_call() -> Result<(), String> {
    let source = "my $r = is_ready $obj ? 1 ? 2 : 3 : 4;";
    assert_clean_parse(source);
    let statements = program_statements(source)?;
    let initializer = variable_initializer(
        statement_at(&statements, 0, "nested bare-call ternary")?,
        "nested bare-call ternary",
    )?;
    match &initializer.kind {
        NodeKind::Ternary { condition, then_expr, else_expr } => {
            // Outer condition must be the bare call
            assert!(
                matches!(condition.kind, NodeKind::FunctionCall { .. }),
                "outer ternary condition must be FunctionCall, got {:?}",
                condition.kind.kind_name()
            );
            // then-branch is another ternary (1 ? 2 : 3)
            assert!(
                matches!(then_expr.kind, NodeKind::Ternary { .. }),
                "then-branch must be nested Ternary, got {:?}",
                then_expr.kind.kind_name()
            );
            // else-branch is a literal 4
            assert!(
                matches!(else_expr.kind, NodeKind::Number { .. }),
                "else-branch must be Number, got {:?}",
                else_expr.kind.kind_name()
            );
        }
        other => return Err(format!("expected outer Ternary, got {other:?}")),
    }
    Ok(())
}
