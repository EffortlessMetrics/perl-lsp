//! Tests for phase-1 `SymbolRef` extraction.

use perl_ast::{Node, NodeKind, SourceLocation};
use perl_symbol::VarKind;
use perl_symbol::surface::{SymbolRefKind, extract_symbol_refs};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn loc(start: usize, end: usize) -> SourceLocation {
    SourceLocation { start, end }
}

#[test]
fn variable_reference_is_extracted() -> Result<()> {
    let var = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "count".to_string() },
        loc(0, 6),
    );
    let expr_stmt =
        Node::new(NodeKind::ExpressionStatement { expression: Box::new(var) }, loc(0, 7));
    let program = Node::new(NodeKind::Program { statements: vec![expr_stmt] }, loc(0, 7));

    let refs = extract_symbol_refs(&program);
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].kind, SymbolRefKind::Variable(VarKind::Scalar));
    assert_eq!(refs[0].name, "count");
    assert_eq!(refs[0].qualified_name, "count");
    assert_eq!(refs[0].sigil.as_deref(), Some("$"));
    assert_eq!(refs[0].package_qualifier, None);
    Ok(())
}

#[test]
fn declaration_target_is_not_treated_as_reference() -> Result<()> {
    let decl_var =
        Node::new(NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() }, loc(3, 5));
    let init_var =
        Node::new(NodeKind::Variable { sigil: "$".to_string(), name: "y".to_string() }, loc(8, 10));
    let decl = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(decl_var),
            attributes: vec![],
            initializer: Some(Box::new(init_var)),
        },
        loc(0, 10),
    );
    let program = Node::new(NodeKind::Program { statements: vec![decl] }, loc(0, 10));

    let refs = extract_symbol_refs(&program);
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].name, "y");
    Ok(())
}

#[test]
fn subroutine_call_reference_is_extracted() -> Result<()> {
    let call = Node::new(
        NodeKind::FunctionCall {
            name: "greet".to_string(),
            args: vec![Node::new(NodeKind::Number { value: "1".to_string() }, loc(6, 7))],
        },
        loc(0, 8),
    );
    let program = Node::new(NodeKind::Program { statements: vec![call] }, loc(0, 8));

    let refs = extract_symbol_refs(&program);
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].kind, SymbolRefKind::SubroutineCall);
    assert_eq!(refs[0].name, "greet");
    assert_eq!(refs[0].qualified_name, "greet");
    assert_eq!(refs[0].package_qualifier, None);
    Ok(())
}

#[test]
fn package_qualified_references_are_projected() -> Result<()> {
    let call = Node::new(
        NodeKind::FunctionCall { name: "My::Pkg::run".to_string(), args: vec![] },
        loc(0, 12),
    );
    let var = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "My::Pkg::VALUE".to_string() },
        loc(13, 28),
    );
    let program = Node::new(NodeKind::Program { statements: vec![call, var] }, loc(0, 28));

    let refs = extract_symbol_refs(&program);
    assert_eq!(refs.len(), 2);

    assert_eq!(refs[0].kind, SymbolRefKind::SubroutineCall);
    assert_eq!(refs[0].name, "run");
    assert_eq!(refs[0].qualified_name, "My::Pkg::run");
    assert_eq!(refs[0].package_qualifier.as_deref(), Some("My::Pkg"));

    assert_eq!(refs[1].kind, SymbolRefKind::Variable(VarKind::Scalar));
    assert_eq!(refs[1].name, "VALUE");
    assert_eq!(refs[1].qualified_name, "My::Pkg::VALUE");
    assert_eq!(refs[1].package_qualifier.as_deref(), Some("My::Pkg"));
    Ok(())
}
