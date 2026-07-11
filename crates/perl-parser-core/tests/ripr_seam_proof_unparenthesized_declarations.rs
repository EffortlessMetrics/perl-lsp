//! Mutation-proof boundary tests for unparenthesized declaration lists.
//!
//! Real Perl declares ONLY the first variable in an unparenthesized `my` /
//! `our` / `state` declaration list. Verified live on `perl 5.42.2`:
//!
//!   $ perl -MO=Deparse,-p -e 'my $a, $b, $c = 1;'
//!   (my($a), $b, ($c = 1));
//!
//! perlsub ("Private Variables via my()"): "If more than one value is
//! listed, the list must be placed in parentheses" and "`my $foo, $bar = 1;`
//! has the same effect as `my $foo; $bar = 1;`". Perl also emits
//! "Parentheses missing around \"my\" list" (perldiag).
//!
//! A comma immediately following the declared variable therefore does NOT
//! extend the declaration — it starts the surrounding comma expression,
//! which the parser represents the same way it represents any other
//! statement-level comma list (`print $a, $b, $c;`): an `ArrayLiteral` of
//! the individual terms, wrapped in an `ExpressionStatement`. Only the
//! first element of that list is a `VariableDeclaration`; the rest are
//! ordinary expression terms.
//!
//! Parenthesized lists (`my ($a, $b) = ...`) are a DIFFERENT grammar form
//! and are unaffected: they still fold every variable into one
//! `NodeKind::VariableListDeclaration`, which is correct Perl.
//!
//! This file previously (incorrectly, per issue #3728) asserted the
//! over-declaring behavior introduced by #3627 — every comma-separated
//! variable folded into the declaration. These assertions are corrected to
//! match real Perl semantics.

mod cpan_test_helpers;

use cpan_test_helpers::assert_clean_parse;
use perl_parser_core::hir::{HirKind, lower_ast};
use perl_parser_core::{Node, NodeKind, Parser};

fn parse_program(source: &str) -> Result<Node, String> {
    let mut parser = Parser::new(source);
    parser.parse().map_err(|error| error.to_string())
}

fn statements(source: &str) -> Result<Vec<Node>, String> {
    let ast = parse_program(source)?;
    match ast.kind {
        NodeKind::Program { statements } => Ok(statements),
        other => Err(format!("expected Program node, got {other:?}")),
    }
}

fn variable_name(variable: &Node) -> Result<String, String> {
    match &variable.kind {
        NodeKind::Variable { sigil, name } => Ok(format!("{sigil}{name}")),
        NodeKind::VariableWithAttributes { variable, .. } => variable_name(variable),
        other => Err(format!("expected declared variable, got {other:?}")),
    }
}

/// Unwrap the `ArrayLiteral` elements built by the statement-level comma
/// continuation for `declarator $first, <rest...>;`.
fn comma_list_elements(declaration: &Node) -> Result<&[Node], String> {
    match &declaration.kind {
        NodeKind::ExpressionStatement { expression } => match &expression.kind {
            NodeKind::ArrayLiteral { elements } => Ok(elements),
            other => Err(format!("expected ArrayLiteral, got {other:?}")),
        },
        other => Err(format!("expected ExpressionStatement, got {other:?}")),
    }
}

#[test]
fn unparenthesized_my_list_declares_only_first_variable() -> Result<(), String> {
    let source = "my $exit, $exit_arg = values();";
    assert_clean_parse(source);

    let ast = parse_program(source)?;
    let program_statements = match &ast.kind {
        NodeKind::Program { statements } => statements,
        other => return Err(format!("expected Program node, got {other:?}")),
    };
    let declaration =
        program_statements.first().ok_or_else(|| "expected declaration statement".to_string())?;

    let elements = comma_list_elements(declaration)?;
    assert_eq!(elements.len(), 2, "expected [my($exit), $exit_arg = values()]");

    // Only `$exit` is declared; it has no initializer of its own (`= values()`
    // belongs to `$exit_arg`, a separate assignment term).
    match &elements[0].kind {
        NodeKind::VariableDeclaration { declarator, variable, initializer, .. } => {
            assert_eq!(declarator, "my");
            assert_eq!(variable_name(variable)?, "$exit");
            assert!(initializer.is_none(), "$exit itself is not initialized");
        }
        other => return Err(format!("expected VariableDeclaration for $exit, got {other:?}")),
    }

    // `$exit_arg = values()` is an ordinary assignment, NOT a declaration.
    match &elements[1].kind {
        NodeKind::Assignment { lhs, .. } => {
            assert_eq!(variable_name(lhs)?, "$exit_arg");
        }
        other => return Err(format!("expected Assignment for $exit_arg, got {other:?}")),
    }

    // HIR: exactly one `my` declaration, binding only `$exit`.
    let hir = lower_ast(&ast);
    let my_decls = hir
        .items
        .iter()
        .filter_map(|item| match &item.kind {
            HirKind::VariableDecl(declaration) if declaration.declarator == "my" => {
                Some(declaration)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(my_decls.len(), 1, "expected exactly one my-declaration");
    assert!(!my_decls[0].is_list, "unparenthesized single-variable declaration is not a list");
    assert_eq!(my_decls[0].variables.len(), 1);
    assert_eq!(
        format!("{}{}", my_decls[0].variables[0].sigil, my_decls[0].variables[0].name),
        "$exit"
    );
    Ok(())
}

#[test]
fn our_and_state_declare_only_first_variable() -> Result<(), String> {
    let source = "our $package_name, $package_value; state $cached_name, $cached_value;";
    assert_clean_parse(source);

    let ast = parse_program(source)?;
    let declarations = match &ast.kind {
        NodeKind::Program { statements } => statements,
        other => return Err(format!("expected Program node, got {other:?}")),
    };
    assert_eq!(declarations.len(), 2);

    let mut declared = Vec::new();
    let mut bare = Vec::new();
    for declaration in declarations {
        let elements = comma_list_elements(declaration)?;
        assert_eq!(elements.len(), 2, "expected [declarator $first, $second]");

        match &elements[0].kind {
            NodeKind::VariableDeclaration { declarator, variable, .. } => {
                declared.push((declarator.clone(), variable_name(variable)?));
            }
            other => return Err(format!("expected VariableDeclaration, got {other:?}")),
        }
        bare.push(variable_name(&elements[1])?);
    }

    assert_eq!(
        declared,
        vec![
            ("our".to_string(), "$package_name".to_string()),
            ("state".to_string(), "$cached_name".to_string()),
        ]
    );
    // The second variable in each list is an ordinary (undeclared) reference.
    assert_eq!(bare, vec!["$package_value".to_string(), "$cached_value".to_string()]);

    let hir = lower_ast(&ast);
    let hir_declarators = hir
        .items
        .iter()
        .filter_map(|item| match &item.kind {
            HirKind::VariableDecl(declaration) => {
                Some((declaration.declarator.clone(), declaration.variables.len()))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(hir_declarators, vec![("our".to_string(), 1), ("state".to_string(), 1)]);
    Ok(())
}

#[test]
fn initializer_argument_commas_remain_a_single_declaration() -> Result<(), String> {
    let source = "my $only = values($first, $second);";
    assert_clean_parse(source);

    let declarations = statements(source)?;
    let declaration =
        declarations.first().ok_or_else(|| "expected declaration statement".to_string())?;
    match &declaration.kind {
        NodeKind::VariableDeclaration { declarator, initializer, .. } => {
            assert_eq!(declarator, "my");
            assert!(initializer.is_some(), "expected initializer");
        }
        other => return Err(format!("expected VariableDeclaration, got {other:?}")),
    }
    Ok(())
}

#[test]
fn unparenthesized_declaration_with_subscripted_first_term_declares_only_that_term()
-> Result<(), String> {
    let source = "my $first[0], $second{key};";
    assert_clean_parse(source);

    let declarations = statements(source)?;
    let declaration =
        declarations.first().ok_or_else(|| "expected declaration statement".to_string())?;
    let elements = comma_list_elements(declaration)?;
    assert_eq!(elements.len(), 2, "expected [my $first[0], $second{{key}}]");

    match &elements[0].kind {
        NodeKind::VariableDeclaration { declarator, variable, .. } => {
            assert_eq!(declarator, "my");
            assert!(
                matches!(&variable.kind, NodeKind::Binary { op, .. } if op == "[]"),
                "expected subscripted declaration target, got {:?}",
                variable.kind
            );
        }
        other => return Err(format!("expected VariableDeclaration, got {other:?}")),
    }
    assert!(matches!(&elements[1].kind, NodeKind::Binary { op, .. } if op == "{}"));
    Ok(())
}

#[test]
fn attributed_first_variable_declares_only_that_variable_with_its_attribute() -> Result<(), String>
{
    let source = "my $first :Attr, %second;";
    assert_clean_parse(source);

    let ast = parse_program(source)?;
    let declarations = match &ast.kind {
        NodeKind::Program { statements } => statements,
        other => return Err(format!("expected Program node, got {other:?}")),
    };
    let declaration =
        declarations.first().ok_or_else(|| "expected declaration statement".to_string())?;
    let elements = comma_list_elements(declaration)?;
    assert_eq!(elements.len(), 2, "expected [my $first :Attr, %second]");

    match &elements[0].kind {
        NodeKind::VariableDeclaration { declarator, variable, attributes, .. } => {
            assert_eq!(declarator, "my");
            assert_eq!(variable_name(variable)?, "$first");
            assert_eq!(attributes, &["Attr".to_string()]);
        }
        other => return Err(format!("expected VariableDeclaration, got {other:?}")),
    }
    // `%second` is a bare (undeclared) reference, not part of the declaration.
    assert_eq!(variable_name(&elements[1])?, "%second");

    let hir = lower_ast(&ast);
    let my_decl = hir.items.iter().find_map(|item| match &item.kind {
        HirKind::VariableDecl(declaration) if declaration.declarator == "my" => Some(declaration),
        _ => None,
    });
    let my_decl = my_decl.ok_or_else(|| "expected my-declaration HIR item".to_string())?;
    assert_eq!(my_decl.variables.len(), 1);
    assert_eq!(my_decl.attribute_count, 1);
    Ok(())
}
