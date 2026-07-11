//! Regression tests for issue #3728 follow-up: unparenthesized `my`/`our`/
//! `state` declarations inside control-flow condition/init positions
//! (`if`, `elsif`, `while`, and the C-style `for` init clause) must also
//! declare ONLY the first variable, matching real Perl semantics.
//!
//! The original #3728 fix removed the over-declaring fold from
//! `parse_variable_declaration` and taught the top-level statement
//! dispatch to absorb the resulting trailing comma term(s). That left the
//! *other* callers of `parse_variable_declaration` — `if`/`elsif`/`while`
//! conditions and the C-style `for` init clause — with a comma dangling
//! after the (now single-variable) declaration, which produced a parse
//! ERROR for constructs real Perl accepts, e.g.:
//!
//!   perl -MO=Deparse,-p -e 'if (my $a, $b) { print $a }'
//!   => if ((my($a), $b)) { print $a; }
//!
//! (The comma operator is valid inside a parenthesized condition; only
//! `$a` is declared, `$b` is evaluated for its own sake / as the branch's
//! effective truth value.)

mod cpan_test_helpers;

use cpan_test_helpers::assert_clean_parse;
use perl_parser_core::hir::{HirKind, lower_ast};
use perl_parser_core::{Node, NodeKind, Parser};

fn parse_program(source: &str) -> Result<Node, String> {
    let mut parser = Parser::new(source);
    parser.parse().map_err(|error| error.to_string())
}

fn first_statement(ast: &Node) -> Result<&Node, String> {
    match &ast.kind {
        NodeKind::Program { statements } => {
            statements.first().ok_or_else(|| "expected at least one statement".to_string())
        }
        other => Err(format!("expected Program node, got {other:?}")),
    }
}

fn variable_name(variable: &Node) -> Result<String, String> {
    match &variable.kind {
        NodeKind::Variable { sigil, name } => Ok(format!("{sigil}{name}")),
        other => Err(format!("expected Variable, got {other:?}")),
    }
}

/// Assert `condition` is `ArrayLiteral[VariableDeclaration(declarator, first_name, None), rest...]`
/// and return the rest of the comma-list elements (everything after the declaration).
fn assert_declares_only_first<'a>(
    condition: &'a Node,
    declarator: &str,
    first_name: &str,
) -> Result<&'a [Node], String> {
    let elements = match &condition.kind {
        NodeKind::ArrayLiteral { elements } => elements,
        other => return Err(format!("expected ArrayLiteral condition, got {other:?}")),
    };
    let first = elements.first().ok_or_else(|| "expected at least one element".to_string())?;
    match &first.kind {
        NodeKind::VariableDeclaration { declarator: d, variable, initializer, .. } => {
            if d != declarator {
                return Err(format!("expected declarator {declarator}, got {d}"));
            }
            if variable_name(variable)? != first_name {
                return Err(format!(
                    "expected first declared variable {first_name}, got {:?}",
                    variable.kind
                ));
            }
            if initializer.is_some() {
                return Err("expected no initializer on the first declared variable".to_string());
            }
        }
        other => return Err(format!("expected VariableDeclaration first element, got {other:?}")),
    }
    Ok(&elements[1..])
}

/// Count `HirKind::VariableDecl` items across the whole program and total
/// bound variables (paired with declarator).
fn declared_var_counts(ast: &Node) -> Vec<(String, usize)> {
    let hir = lower_ast(ast);
    hir.items
        .iter()
        .filter_map(|item| match &item.kind {
            HirKind::VariableDecl(declaration) => {
                Some((declaration.declarator.clone(), declaration.variables.len()))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn if_condition_declares_only_first_variable() -> Result<(), String> {
    let source = "if (my $a, $b) { $a = 1; }";
    assert_clean_parse(source);

    let ast = parse_program(source)?;
    let stmt = first_statement(&ast)?;
    let condition = match &stmt.kind {
        NodeKind::If { condition, .. } => condition,
        other => return Err(format!("expected If, got {other:?}")),
    };

    let rest = assert_declares_only_first(condition, "my", "$a")?;
    assert_eq!(rest.len(), 1, "expected exactly one trailing comma term ($b)");
    assert_eq!(variable_name(&rest[0])?, "$b");

    assert_eq!(
        declared_var_counts(&ast),
        vec![("my".to_string(), 1)],
        "only $a should be recorded as a declared lexical"
    );
    Ok(())
}

#[test]
fn elsif_condition_declares_only_first_variable() -> Result<(), String> {
    let source = "if (0) { } elsif (my $p, $q) { $p = 1; }";
    assert_clean_parse(source);

    let ast = parse_program(source)?;
    let stmt = first_statement(&ast)?;
    let elsif_branches = match &stmt.kind {
        NodeKind::If { elsif_branches, .. } => elsif_branches,
        other => return Err(format!("expected If, got {other:?}")),
    };
    let (elsif_condition, _) =
        elsif_branches.first().ok_or_else(|| "expected one elsif branch".to_string())?;

    let rest = assert_declares_only_first(elsif_condition, "my", "$p")?;
    assert_eq!(rest.len(), 1);
    assert_eq!(variable_name(&rest[0])?, "$q");

    assert_eq!(declared_var_counts(&ast), vec![("my".to_string(), 1)]);
    Ok(())
}

#[test]
fn while_condition_declares_only_first_variable() -> Result<(), String> {
    let source = "while (my $x, $y) { last; }";
    assert_clean_parse(source);

    let ast = parse_program(source)?;
    let stmt = first_statement(&ast)?;
    let condition = match &stmt.kind {
        NodeKind::While { condition, .. } => condition,
        other => return Err(format!("expected While, got {other:?}")),
    };

    let rest = assert_declares_only_first(condition, "my", "$x")?;
    assert_eq!(rest.len(), 1);
    assert_eq!(variable_name(&rest[0])?, "$y");

    assert_eq!(declared_var_counts(&ast), vec![("my".to_string(), 1)]);
    Ok(())
}

#[test]
fn c_style_for_init_declares_only_first_variable() -> Result<(), String> {
    let source = "for (my $i, $j; $i < 10; $i++) { }";
    assert_clean_parse(source);

    let ast = parse_program(source)?;
    let stmt = first_statement(&ast)?;
    let init = match &stmt.kind {
        NodeKind::For { init, .. } => {
            init.as_deref().ok_or_else(|| "expected a for-init clause".to_string())?
        }
        other => return Err(format!("expected For, got {other:?}")),
    };

    let rest = assert_declares_only_first(init, "my", "$i")?;
    assert_eq!(rest.len(), 1);
    assert_eq!(variable_name(&rest[0])?, "$j");

    assert_eq!(declared_var_counts(&ast), vec![("my".to_string(), 1)]);
    Ok(())
}

#[test]
fn our_state_local_declare_only_first_variable_in_conditions() -> Result<(), String> {
    for (declarator, source) in
        [("our", "if (our $a, $b) { $a = 1; }"), ("state", "while (state $a, $b) { last; }")]
    {
        assert_clean_parse(source);
        let ast = parse_program(source)?;
        assert_eq!(
            declared_var_counts(&ast),
            vec![(declarator.to_string(), 1)],
            "source {source:?} should declare only its first variable"
        );
    }
    Ok(())
}

/// Guard: a PARENTHESIZED declaration list inside a condition is a
/// completely different grammar form and MUST still declare every variable
/// — that is correct Perl (`my ($a, $b) = ...`).
#[test]
fn parenthesized_declaration_in_condition_still_declares_both() -> Result<(), String> {
    let source = "if (my ($a, $b) = (1, 2)) { $a = $b; }";
    assert_clean_parse(source);

    let ast = parse_program(&source)?;
    let stmt = first_statement(&ast)?;
    let condition = match &stmt.kind {
        NodeKind::If { condition, .. } => condition,
        other => return Err(format!("expected If, got {other:?}")),
    };
    match &condition.kind {
        NodeKind::VariableListDeclaration { declarator, variables, initializer, .. } => {
            assert_eq!(declarator, "my");
            assert_eq!(variables.len(), 2);
            assert!(initializer.is_some());
        }
        other => return Err(format!("expected VariableListDeclaration, got {other:?}")),
    }
    assert_eq!(declared_var_counts(&ast), vec![("my".to_string(), 2)]);
    Ok(())
}

/// Guard: the pre-existing "Pattern D" support (issue #2750) — a
/// declaration with no comma, followed directly by a binary operator
/// inside the condition — must keep working exactly as before.
#[test]
fn declaration_followed_by_binary_operator_in_condition_still_works() -> Result<(), String> {
    let source = "if (our $can_haz_xs && $ok) { }";
    assert_clean_parse(source);

    let ast = parse_program(&source)?;
    let stmt = first_statement(&ast)?;
    let condition = match &stmt.kind {
        NodeKind::If { condition, .. } => condition,
        other => return Err(format!("expected If, got {other:?}")),
    };
    match &condition.kind {
        NodeKind::Binary { op, left, .. } => {
            assert_eq!(op, "&&");
            match &left.kind {
                NodeKind::VariableDeclaration { declarator, .. } => {
                    assert_eq!(declarator, "our");
                }
                other => return Err(format!("expected VariableDeclaration, got {other:?}")),
            }
        }
        other => return Err(format!("expected Binary &&, got {other:?}")),
    }
    Ok(())
}
