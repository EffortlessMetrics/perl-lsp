//! Mutation-proof boundary tests for unparenthesized declaration lists.
//!
//! A comma after a lexical variable begins a declaration list only when the
//! following token is another variable.  The parser must retain every binding
//! in the shared list-declaration AST shape without reclassifying initializer
//! argument commas.

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

fn variable_names(variables: &[Node]) -> Result<Vec<String>, String> {
    variables.iter().map(variable_name).collect()
}

fn variable_name(variable: &Node) -> Result<String, String> {
    match &variable.kind {
        NodeKind::Variable { sigil, name } => Ok(format!("{sigil}{name}")),
        NodeKind::VariableWithAttributes { variable, .. } => variable_name(variable),
        other => Err(format!("expected declared variable, got {other:?}")),
    }
}

#[test]
fn unparenthesized_my_list_records_all_bindings_and_initializer() -> Result<(), String> {
    let source = "my $exit, $exit_arg = values();";
    assert_clean_parse(source);

    let ast = parse_program(source)?;
    let program_statements = match &ast.kind {
        NodeKind::Program { statements } => statements,
        other => return Err(format!("expected Program node, got {other:?}")),
    };
    let declaration =
        program_statements.first().ok_or_else(|| "expected declaration statement".to_string())?;

    match &declaration.kind {
        NodeKind::VariableListDeclaration { declarator, variables, initializer, .. } => {
            assert_eq!(declarator, "my");
            assert_eq!(variable_names(variables)?, ["$exit", "$exit_arg"]);
            assert!(initializer.is_some(), "expected list initializer");
        }
        other => return Err(format!("expected VariableListDeclaration, got {other:?}")),
    }

    let hir = lower_ast(&ast);
    let list_declaration = hir.items.iter().find_map(|item| match &item.kind {
        HirKind::VariableDecl(declaration) if declaration.is_list => Some(declaration),
        _ => None,
    });
    let list_declaration =
        list_declaration.ok_or_else(|| "expected list declaration HIR item".to_string())?;
    assert_eq!(list_declaration.declarator, "my");
    assert_eq!(list_declaration.variables.len(), 2);
    assert!(list_declaration.has_initializer);
    Ok(())
}

#[test]
fn our_and_state_support_unparenthesized_declaration_lists() -> Result<(), String> {
    let source = "our $package_name, $package_value; state $cached_name, $cached_value;";
    assert_clean_parse(source);

    let ast = parse_program(source)?;
    let declarations = match &ast.kind {
        NodeKind::Program { statements } => statements,
        other => return Err(format!("expected Program node, got {other:?}")),
    };
    let mut declarators = Vec::new();
    for declaration in declarations {
        match &declaration.kind {
            NodeKind::VariableListDeclaration { declarator, variables, .. } => {
                declarators.push((declarator.clone(), variable_names(variables)?));
            }
            other => return Err(format!("expected VariableListDeclaration, got {other:?}")),
        }
    }

    assert_eq!(
        declarators,
        vec![
            ("our".to_string(), vec!["$package_name".to_string(), "$package_value".to_string()]),
            ("state".to_string(), vec!["$cached_name".to_string(), "$cached_value".to_string()]),
        ]
    );

    let hir = lower_ast(&ast);
    let hir_declarators = hir
        .items
        .iter()
        .filter_map(|item| match &item.kind {
            HirKind::VariableDecl(declaration) if declaration.is_list => {
                Some((declaration.declarator.clone(), declaration.variables.len()))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(hir_declarators, vec![("our".to_string(), 2), ("state".to_string(), 2)]);
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
fn declaration_list_preserves_direct_subscripts_for_every_variable() -> Result<(), String> {
    let source = "my $first[0], $second{key};";
    assert_clean_parse(source);

    let declarations = statements(source)?;
    let declaration =
        declarations.first().ok_or_else(|| "expected declaration statement".to_string())?;
    let variables = match &declaration.kind {
        NodeKind::VariableListDeclaration { variables, .. } => variables,
        other => return Err(format!("expected VariableListDeclaration, got {other:?}")),
    };

    assert_eq!(variables.len(), 2);
    assert!(matches!(&variables[0].kind, NodeKind::Binary { op, .. } if op == "[]"));
    assert!(matches!(&variables[1].kind, NodeKind::Binary { op, .. } if op == "{}"));
    Ok(())
}

#[test]
fn attributed_and_hash_declarations_remain_list_bindings() -> Result<(), String> {
    let source = "my $first :Attr, %second;";
    assert_clean_parse(source);

    let ast = parse_program(source)?;
    let declarations = match &ast.kind {
        NodeKind::Program { statements } => statements,
        other => return Err(format!("expected Program node, got {other:?}")),
    };
    let declaration =
        declarations.first().ok_or_else(|| "expected declaration statement".to_string())?;
    let variables = match &declaration.kind {
        NodeKind::VariableListDeclaration { variables, .. } => variables,
        other => return Err(format!("expected VariableListDeclaration, got {other:?}")),
    };

    assert_eq!(variable_names(variables)?, ["$first", "%second"]);
    assert!(matches!(
        &variables[0].kind,
        NodeKind::VariableWithAttributes { attributes, .. } if attributes == &["Attr"]
    ));

    let hir = lower_ast(&ast);
    let list_declaration = hir.items.iter().find_map(|item| match &item.kind {
        HirKind::VariableDecl(declaration) if declaration.is_list => Some(declaration),
        _ => None,
    });
    let list_declaration =
        list_declaration.ok_or_else(|| "expected list declaration HIR item".to_string())?;
    assert_eq!(list_declaration.variables.len(), 2);
    Ok(())
}
