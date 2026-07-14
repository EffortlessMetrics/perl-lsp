//! Regression test for issue #3728: unparenthesized `my`/`our`/`state`
//! declaration lists must declare ONLY the first variable, matching real
//! Perl semantics.
//!
//! Ground truth (perl 5.42.2, `-MO=Deparse,-p`):
//!
//!   $ perl -MO=Deparse,-p -e 'my $a, $b, $c = 1;'
//!   (my($a), $b, ($c = 1));
//!
//! Only `$a` is a declared lexical; `$b` is an untouched package global and
//! `$c = 1` is an ordinary assignment to a package global. perlsub:
//! "If more than one value is listed, the list must be placed in
//! parentheses." A regression introduced by #3627 folded every
//! comma-separated variable into one `NodeKind::VariableListDeclaration`,
//! reporting `num_declared_vars=3` for this source.

mod cpan_test_helpers;

use cpan_test_helpers::assert_clean_parse;
use perl_parser_core::hir::{HirKind, lower_ast};
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

/// Count of `HirKind::VariableDecl` items produced across the whole program,
/// paired with (declarator, number of variables actually bound).
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
fn unparenthesized_my_comma_list_declares_only_first_variable() -> Result<(), String> {
    // Real Perl: `(my($a), $b, ($c = 1));` — only `$a` is a lexical.
    let source = "my $a, $b, $c = 1;";
    assert_clean_parse(source);

    let ast = parse_program(source)?;
    let counts = declared_var_counts(&ast);

    // Exactly one declaration should be recorded (for `my`), and it should
    // bind exactly one variable ($a). Before the fix, this reported a single
    // `my` VariableDecl with 3 bound variables ($a, $b, $c) — the over-fold.
    assert_eq!(
        counts,
        vec![("my".to_string(), 1)],
        "expected only $a to be declared as a lexical; got {counts:?}"
    );

    // $b must not be recorded as a declared binding anywhere in the program.
    let hir = lower_ast(&ast);
    let all_declared_names: Vec<String> = hir
        .items
        .iter()
        .filter_map(|item| match &item.kind {
            HirKind::VariableDecl(declaration) => Some(
                declaration
                    .variables
                    .iter()
                    .map(|binding| format!("{}{}", binding.sigil, binding.name))
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .flatten()
        .collect();
    assert_eq!(
        all_declared_names,
        vec!["$a".to_string()],
        "only $a should be a declared lexical binding; got {all_declared_names:?}"
    );

    Ok(())
}

#[test]
fn unparenthesized_our_and_state_comma_lists_declare_only_first_variable() -> Result<(), String> {
    let source = "our $x, $y; state $p, $q;";
    assert_clean_parse(source);

    let ast = parse_program(source)?;
    let counts = declared_var_counts(&ast);

    assert_eq!(
        counts,
        vec![("our".to_string(), 1), ("state".to_string(), 1)],
        "expected our/state to each declare only their first variable; got {counts:?}"
    );

    Ok(())
}

#[test]
fn parenthesized_my_list_still_declares_both_variables() -> Result<(), String> {
    // Guard: `my ($a, $b) = (1, 2);` is CORRECT Perl and MUST still declare
    // both variables. Parenthesized lists are unaffected by the fix for the
    // unparenthesized case.
    let source = "my ($a, $b) = (1, 2);";
    assert_clean_parse(source);

    let ast = parse_program(source)?;
    let statements = program_statements(&ast)?;
    let declaration =
        statements.first().ok_or_else(|| "expected declaration statement".to_string())?;

    match &declaration.kind {
        NodeKind::VariableListDeclaration { declarator, variables, initializer, .. } => {
            assert_eq!(declarator, "my");
            assert_eq!(variables.len(), 2, "parenthesized my list must still declare both vars");
            assert!(initializer.is_some());
        }
        other => return Err(format!("expected VariableListDeclaration, got {other:?}")),
    }

    let counts = declared_var_counts(&ast);
    assert_eq!(
        counts,
        vec![("my".to_string(), 2)],
        "parenthesized declaration must record both bound variables; got {counts:?}"
    );

    Ok(())
}
