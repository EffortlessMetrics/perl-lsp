//! Regression test for issue #3748: `my $a = 1, $b;` mis-parses the
//! initializer RHS of `$a` as the whole comma list `1, $b` instead of
//! stopping at `1`.
//!
//! Ground truth (perl 5.42.2, `-MO=Deparse,-p`):
//!
//!   $ perl -MO=Deparse,-p -e 'my $a = 1, $b;'
//!   ((my($a) = 1), $b);
//!
//!   $ perl -MO=Deparse,-p -e 'my $a = (1, $b);'
//!   (my($a) = ('???', $b));
//!
//! perlop: `=` binds tighter than `,`. So `my $a = 1, $b;` binds as
//! `((my $a = 1), $b)` — the initializer of `$a` is just the scalar literal
//! `1`, and `$b` is a separate trailing comma term at the statement level.
//! Only a PARENTHESIZED RHS (`my $a = (1, $b);`) keeps the list as the
//! initializer, because the parenthesized group is a single primary term.

mod cpan_test_helpers;

use cpan_test_helpers::assert_clean_parse;
use perl_parser_core::{Node, NodeKind, Parser};

fn parse_program(source: &str) -> Result<Node, String> {
    let mut parser = Parser::new(source);
    parser.parse().map_err(|error| error.to_string())
}

fn program_statements(ast: &Node) -> Result<&[Node], String> {
    match &ast.kind {
        NodeKind::Program { statements } => Ok(statements),
        other => Err(format!("expected Program node, got {other:?}")),
    }
}

#[test]
fn unparenthesized_rhs_stops_initializer_at_comma() -> Result<(), String> {
    let source = "my $a = 1, $b;";
    assert_clean_parse(source);

    let ast = parse_program(source)?;
    let statements = program_statements(&ast)?;
    let top = statements.first().ok_or_else(|| "expected one top-level statement".to_string())?;

    // The comma-continuation wraps the declaration + trailing `$b` in an
    // ExpressionStatement { ArrayLiteral { [VariableDeclaration, $b] } }.
    let expr = match &top.kind {
        NodeKind::ExpressionStatement { expression } => expression.as_ref(),
        NodeKind::VariableDeclaration { .. } => top,
        other => {
            return Err(format!(
                "expected ExpressionStatement or VariableDeclaration, got {other:?}"
            ));
        }
    };

    let elements = match &expr.kind {
        NodeKind::ArrayLiteral { elements } => elements.as_slice(),
        NodeKind::VariableDeclaration { .. } => std::slice::from_ref(expr),
        other => {
            return Err(format!("expected ArrayLiteral or VariableDeclaration, got {other:?}"));
        }
    };

    assert_eq!(
        elements.len(),
        2,
        "expected two top-level comma terms: (my $a = 1) and $b; got {elements:?}"
    );

    // First term: the `my $a = 1` declaration, whose initializer must be
    // JUST the scalar literal `1` — not a comma list / ArrayLiteral folding
    // in `$b`.
    let decl = elements.first().ok_or_else(|| "expected first comma term".to_string())?;
    match &decl.kind {
        NodeKind::VariableDeclaration { declarator, initializer, .. } => {
            assert_eq!(declarator, "my");
            let init = initializer
                .as_ref()
                .ok_or_else(|| "expected $a to have an initializer".to_string())?;
            match &init.kind {
                NodeKind::Number { value } => {
                    assert_eq!(value, "1", "initializer must be the scalar literal 1");
                }
                other => {
                    return Err(format!(
                        "initializer must be a scalar Number literal, not a comma list; got {other:?}"
                    ));
                }
            }
        }
        other => {
            return Err(format!("expected VariableDeclaration as first comma term, got {other:?}"));
        }
    }

    // Second term: `$b` as a standalone trailing expression, NOT folded
    // into $a's initializer.
    let second = elements.get(1).ok_or_else(|| "expected second comma term".to_string())?;
    match &second.kind {
        NodeKind::Variable { sigil, name, .. } => {
            assert_eq!(sigil, "$");
            assert_eq!(name, "b");
        }
        other => return Err(format!("expected trailing $b variable, got {other:?}")),
    }

    Ok(())
}

#[test]
fn parenthesized_rhs_keeps_list_as_initializer() -> Result<(), String> {
    // Guard: `my $a = (1, $b);` — the parens make `(1, $b)` a single
    // primary term, so it stays as $a's initializer.
    let source = "my $a = (1, $b);";
    assert_clean_parse(source);

    let ast = parse_program(source)?;
    let statements = program_statements(&ast)?;
    let top = statements.first().ok_or_else(|| "expected one top-level statement".to_string())?;

    let decl = match &top.kind {
        NodeKind::VariableDeclaration { .. } => top,
        NodeKind::ExpressionStatement { expression } => expression.as_ref(),
        other => return Err(format!("expected VariableDeclaration, got {other:?}")),
    };

    match &decl.kind {
        NodeKind::VariableDeclaration { declarator, initializer, .. } => {
            assert_eq!(declarator, "my");
            let init = initializer
                .as_ref()
                .ok_or_else(|| "expected $a to have an initializer".to_string())?;
            match &init.kind {
                NodeKind::ArrayLiteral { elements } => {
                    assert_eq!(
                        elements.len(),
                        2,
                        "parenthesized (1, $b) initializer must keep both elements; got {elements:?}"
                    );
                }
                other => {
                    return Err(format!(
                        "parenthesized RHS must stay a list initializer; got {other:?}"
                    ));
                }
            }
        }
        other => return Err(format!("expected VariableDeclaration, got {other:?}")),
    }

    Ok(())
}

// -----------------------------------------------------------------------------
// P1 regression caught by review (factory-droid + codex threads on PR #3908):
// switching the declaration-initializer RHS from parse_expression (comma
// precedence) to parse_assignment (assignment precedence) fixed #3748, but
// broke the low-precedence WORD operators (`or`/`and`) that commonly follow
// a `my`-in-condition idiom -- `if (my $x = foo() or die)`,
// `while (my $x = next_row() or last)`.
//
// Before #3748's fix, parse_variable_declaration's initializer parse used
// parse_expression, which internally descends through comma AND word-operator
// precedence (see parse_comma / parse_word_or_expr in
// engine/parser/expressions/precedence.rs) -- so it silently absorbed a
// trailing `or die` into $x's initializer itself: `my $x = (foo() or die)`.
// That's WRONG per perlop (`or`/`and` are lower precedence than `=`, so they
// must bind the whole assignment, not just its RHS) but it didn't error.
//
// After #3748's fix correctly stopped the initializer at assignment
// precedence, parse_condition_declaration (engine/parser/control_flow.rs)
// had no step that continued parsing a trailing `or`/`and` with the whole
// declaration as the left operand, so the trailing word operator was left
// unconsumed and the `)` expectation failed with a hard parse ERROR.
//
// Ground truth (perl 5.42.2, `-MO=Deparse,-p`):
//
//   $ perl -MO=Deparse,-p -e 'if (my $x = foo() or die) {}'
//   if (((my $x = foo()) or die)) { ... }
//
//   $ perl -MO=Deparse,-p -e 'while (my $x = next_row() or last) {}'
//   while (((my $x = next_row()) or (last))) { ... }
//
//   $ perl -MO=Deparse,-p -e 'if (my $y = g() and $y > 0) {}'
//   if (((my $y = g()) and ($y > 0))) { ... }
//
// Fixed in parse_condition_declaration by applying parse_word_or_expr (the
// same word-operator continuation the ordinary expression path already uses)
// to the whole declaration + comma-continuation, after the comma
// continuation and after the `&&`/`||`/ternary continuation -- matching
// perlop's `, ` > `and` > `or`/`xor` precedence ladder.
// -----------------------------------------------------------------------------

#[test]
fn if_condition_declaration_or_die_binds_whole_assignment() -> Result<(), String> {
    let source = "if (my $x = foo() or die) {}";
    assert_clean_parse(source);

    let ast = parse_program(source)?;
    let statements = program_statements(&ast)?;
    let top = statements.first().ok_or_else(|| "expected one top-level statement".to_string())?;

    let condition = match &top.kind {
        NodeKind::If { condition, .. } => condition.as_ref(),
        other => return Err(format!("expected If statement, got {other:?}")),
    };

    match &condition.kind {
        NodeKind::Binary { op, left, right } => {
            assert_eq!(op, "or", "condition must be an `or` binary node, got op {op:?}");
            match &left.kind {
                NodeKind::VariableDeclaration { declarator, initializer, .. } => {
                    assert_eq!(
                        declarator, "my",
                        "the `or`'s left operand must be the WHOLE `my $x = foo()` \
                         declaration, not just foo()"
                    );
                    let init = initializer
                        .as_ref()
                        .ok_or_else(|| "expected $x to have an initializer".to_string())?;
                    if matches!(&init.kind, NodeKind::Binary { op, .. } if op == "or") {
                        return Err("or die must NOT be folded into $x's initializer".to_string());
                    }
                }
                other => {
                    return Err(format!(
                        "expected the `or`'s left operand to be the my-declaration, got {other:?}"
                    ));
                }
            }
            let right_sexp = right.to_sexp();
            assert!(
                right_sexp.contains("die"),
                "expected `die` as the `or`'s right operand, got {right_sexp}"
            );
        }
        other => return Err(format!("expected a Binary `or` condition node, got {other:?}")),
    }

    Ok(())
}

#[test]
fn while_condition_declaration_or_last_binds_whole_assignment() -> Result<(), String> {
    let source = "while (my $x = next_row() or last) {}";
    assert_clean_parse(source);

    let ast = parse_program(source)?;
    let statements = program_statements(&ast)?;
    let top = statements.first().ok_or_else(|| "expected one top-level statement".to_string())?;

    let condition = match &top.kind {
        NodeKind::While { condition, .. } => condition.as_ref(),
        other => return Err(format!("expected While statement, got {other:?}")),
    };

    match &condition.kind {
        NodeKind::Binary { op, left, .. } => {
            assert_eq!(op, "or", "condition must be an `or` binary node, got op {op:?}");
            match &left.kind {
                NodeKind::VariableDeclaration { declarator, .. } => {
                    assert_eq!(
                        declarator, "my",
                        "the `or`'s left operand must be the WHOLE `my $x = next_row()` \
                         declaration"
                    );
                }
                other => {
                    return Err(format!(
                        "expected the `or`'s left operand to be the my-declaration, got {other:?}"
                    ));
                }
            }
        }
        other => return Err(format!("expected a Binary `or` condition node, got {other:?}")),
    }

    Ok(())
}

#[test]
fn if_condition_declaration_and_binds_whole_assignment() -> Result<(), String> {
    let source = "if (my $y = g() and $y > 0) {}";
    assert_clean_parse(source);

    let ast = parse_program(source)?;
    let statements = program_statements(&ast)?;
    let top = statements.first().ok_or_else(|| "expected one top-level statement".to_string())?;

    let condition = match &top.kind {
        NodeKind::If { condition, .. } => condition.as_ref(),
        other => return Err(format!("expected If statement, got {other:?}")),
    };

    match &condition.kind {
        NodeKind::Binary { op, left, right } => {
            assert_eq!(op, "and", "condition must be an `and` binary node, got op {op:?}");
            match &left.kind {
                NodeKind::VariableDeclaration { declarator, .. } => {
                    assert_eq!(
                        declarator, "my",
                        "the `and`'s left operand must be the WHOLE `my $y = g()` declaration"
                    );
                }
                other => {
                    return Err(format!(
                        "expected the `and`'s left operand to be the my-declaration, got {other:?}"
                    ));
                }
            }
            match &right.kind {
                NodeKind::Binary { op, .. } => {
                    assert_eq!(op, ">", "right operand must be `$y > 0`, got op {op:?}");
                }
                other => return Err(format!("expected `$y > 0` as right operand, got {other:?}")),
            }
        }
        other => return Err(format!("expected a Binary `and` condition node, got {other:?}")),
    }

    Ok(())
}

// -----------------------------------------------------------------------------
// Sibling follow-up (factory-droid thread PRRT_kwDOSid81M6QKmjZ on PR #3908):
// parse_condition_declaration (if/elsif/while/unless) got the parse_word_or_expr
// continuation above, but parse_c_style_or_implicit_foreach's `my`-in-init
// path (engine/parser/control_flow.rs) -- the C-style for-loop's ONE
// remaining unpaired collect_comma_fat_arrow_continuation call site -- did
// not, so the identical bug was still live there:
// `for (my $i = 0 or die; $i < 10; $i++) {}` failed with
// "expected expression, found 'or'".
//
// Ground truth (perl 5.42.2, `-MO=Deparse,-p`):
//
//   $ perl -MO=Deparse,-p -e 'for (my $i = 0 or die; $i < 10; $i++) {}'
//   for (((my $i = 0) or die); ($i < 10); (++$i)) { ... }
//
// Fixed the same way: parse_word_or_expr is now applied to the for-init
// declaration after its comma continuation.
// -----------------------------------------------------------------------------

#[test]
fn for_loop_init_declaration_or_die_binds_whole_assignment() -> Result<(), String> {
    let source = "for (my $i = 0 or die; $i < 10; $i++) {}";
    assert_clean_parse(source);

    let ast = parse_program(source)?;
    let statements = program_statements(&ast)?;
    let top = statements.first().ok_or_else(|| "expected one top-level statement".to_string())?;

    let init = match &top.kind {
        NodeKind::For { init, .. } => {
            init.as_ref().ok_or_else(|| "expected a for-loop init clause".to_string())?
        }
        other => return Err(format!("expected For statement, got {other:?}")),
    };

    match &init.kind {
        NodeKind::Binary { op, left, right } => {
            assert_eq!(op, "or", "init clause must be an `or` binary node, got op {op:?}");
            match &left.kind {
                NodeKind::VariableDeclaration { declarator, initializer, .. } => {
                    assert_eq!(
                        declarator, "my",
                        "the `or`'s left operand must be the WHOLE `my $i = 0` declaration, \
                         not just 0"
                    );
                    let decl_init = initializer
                        .as_ref()
                        .ok_or_else(|| "expected $i to have an initializer".to_string())?;
                    if matches!(&decl_init.kind, NodeKind::Binary { op, .. } if op == "or") {
                        return Err("or die must NOT be folded into $i's initializer".to_string());
                    }
                }
                other => {
                    return Err(format!(
                        "expected the `or`'s left operand to be the my-declaration, got {other:?}"
                    ));
                }
            }
            let right_sexp = right.to_sexp();
            assert!(
                right_sexp.contains("die"),
                "expected `die` as the `or`'s right operand, got {right_sexp}"
            );
        }
        other => return Err(format!("expected a Binary `or` init clause, got {other:?}")),
    }

    Ok(())
}

#[test]
fn for_loop_init_declaration_without_or_still_parses_plainly() -> Result<(), String> {
    // Guard: a for-init with no trailing word operator must NOT gain an
    // extra Binary wrapper from the new parse_word_or_expr continuation --
    // no over-consumption.
    let source = "for (my $i = 0; $i < 10; $i++) {}";
    assert_clean_parse(source);

    let ast = parse_program(source)?;
    let statements = program_statements(&ast)?;
    let top = statements.first().ok_or_else(|| "expected one top-level statement".to_string())?;

    let init = match &top.kind {
        NodeKind::For { init, .. } => {
            init.as_ref().ok_or_else(|| "expected a for-loop init clause".to_string())?
        }
        other => return Err(format!("expected For statement, got {other:?}")),
    };

    match &init.kind {
        NodeKind::VariableDeclaration { declarator, initializer, .. } => {
            assert_eq!(declarator, "my");
            let decl_init = initializer
                .as_ref()
                .ok_or_else(|| "expected $i to have an initializer".to_string())?;
            match &decl_init.kind {
                NodeKind::Number { value } => assert_eq!(value, "0"),
                other => return Err(format!("expected initializer 0, got {other:?}")),
            }
        }
        other => {
            return Err(format!(
                "for-init without a trailing word operator must stay a plain \
                 VariableDeclaration, got {other:?}"
            ));
        }
    }

    Ok(())
}
