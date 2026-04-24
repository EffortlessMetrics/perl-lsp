//! Tests for phase-1 `SymbolRef` extraction.

use perl_ast::{Node, NodeKind, SourceLocation};
use perl_symbol::surface::extract_symbol_refs;
use perl_symbol::{SymbolKind, VarKind};

fn loc(start: usize, end: usize) -> SourceLocation {
    SourceLocation { start, end }
}

#[test]
fn test_variable_reference_is_extracted() {
    let var = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "count".to_string() },
        loc(0, 6),
    );
    let stmt = Node::new(NodeKind::ExpressionStatement { expression: Box::new(var) }, loc(0, 6));
    let program = Node::new(NodeKind::Program { statements: vec![stmt] }, loc(0, 6));

    let refs = extract_symbol_refs(&program, Some("MyPkg"));

    assert_eq!(refs.len(), 1);
    let r = &refs[0];
    assert_eq!(r.kind, SymbolKind::Variable(VarKind::Scalar));
    assert_eq!(r.name, "count");
    assert_eq!(r.qualified_name, "MyPkg::count");
    assert_eq!(r.referenced_package, None);
    assert_eq!(r.container.as_deref(), Some("MyPkg"));
    assert_eq!(r.span, (0, 6));
}

#[test]
fn test_subroutine_call_reference_is_extracted() {
    let call = Node::new(
        NodeKind::FunctionCall {
            name: "greet".to_string(),
            args: vec![Node::new(
                NodeKind::Variable { sigil: "$".to_string(), name: "name".to_string() },
                loc(7, 12),
            )],
        },
        loc(0, 13),
    );
    let stmt = Node::new(NodeKind::ExpressionStatement { expression: Box::new(call) }, loc(0, 13));
    let program = Node::new(NodeKind::Program { statements: vec![stmt] }, loc(0, 13));

    let refs = extract_symbol_refs(&program, Some("MyPkg"));

    assert_eq!(refs.len(), 2);
    let call_ref = &refs[0];
    assert_eq!(call_ref.kind, SymbolKind::Subroutine);
    assert_eq!(call_ref.name, "greet");
    assert_eq!(call_ref.qualified_name, "MyPkg::greet");
    assert_eq!(call_ref.referenced_package, None);

    let arg_ref = &refs[1];
    assert_eq!(arg_ref.kind, SymbolKind::Variable(VarKind::Scalar));
    assert_eq!(arg_ref.name, "name");
    assert_eq!(arg_ref.qualified_name, "MyPkg::name");
}

#[test]
fn test_explicit_package_qualified_refs_are_preserved() {
    let call = Node::new(
        NodeKind::FunctionCall {
            name: "Other::run".to_string(),
            args: vec![Node::new(
                NodeKind::Variable { sigil: "$".to_string(), name: "Other::VALUE".to_string() },
                loc(11, 24),
            )],
        },
        loc(0, 25),
    );
    let stmt = Node::new(NodeKind::ExpressionStatement { expression: Box::new(call) }, loc(0, 25));
    let program = Node::new(NodeKind::Program { statements: vec![stmt] }, loc(0, 25));

    let refs = extract_symbol_refs(&program, Some("Local"));

    assert_eq!(refs.len(), 2);

    let call_ref = &refs[0];
    assert_eq!(call_ref.kind, SymbolKind::Subroutine);
    assert_eq!(call_ref.name, "run");
    assert_eq!(call_ref.qualified_name, "Other::run");
    assert_eq!(call_ref.referenced_package.as_deref(), Some("Other"));
    assert_eq!(call_ref.container.as_deref(), Some("Local"));

    let var_ref = &refs[1];
    assert_eq!(var_ref.kind, SymbolKind::Variable(VarKind::Scalar));
    assert_eq!(var_ref.name, "VALUE");
    assert_eq!(var_ref.qualified_name, "Other::VALUE");
    assert_eq!(var_ref.referenced_package.as_deref(), Some("Other"));
}

#[test]
fn test_variable_declaration_sites_are_not_emitted_as_refs() {
    let declared_var = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "declared".to_string() },
        loc(3, 12),
    );
    let initializer_ref = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "source".to_string() },
        loc(15, 21),
    );
    let decl = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(declared_var),
            attributes: vec![],
            initializer: Some(Box::new(initializer_ref)),
        },
        loc(0, 21),
    );
    let program = Node::new(NodeKind::Program { statements: vec![decl] }, loc(0, 21));

    let refs = extract_symbol_refs(&program, None);

    assert_eq!(refs.len(), 1);
    let r = &refs[0];
    assert_eq!(r.name, "source");
    assert_eq!(r.kind, SymbolKind::Variable(VarKind::Scalar));
}
