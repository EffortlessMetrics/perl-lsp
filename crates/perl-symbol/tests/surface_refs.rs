//! Tests for phase-1 `SymbolRef` extraction.

use perl_ast::{Node, NodeKind, SourceLocation};
use perl_symbol::surface::extract_symbol_refs;
use perl_symbol::{SymbolKind, VarKind};

fn loc(start: usize, end: usize) -> SourceLocation {
    SourceLocation { start, end }
}

#[test]
fn variable_usage_is_projected_as_symbol_ref() {
    let usage = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "count".to_string() },
        loc(5, 11),
    );
    let stmt = Node::new(NodeKind::ExpressionStatement { expression: Box::new(usage) }, loc(5, 11));
    let program = Node::new(NodeKind::Program { statements: vec![stmt] }, loc(0, 11));

    let refs = extract_symbol_refs(&program);

    assert_eq!(refs.len(), 1);
    let symbol_ref = &refs[0];
    assert_eq!(symbol_ref.kind, SymbolKind::Variable(VarKind::Scalar));
    assert_eq!(symbol_ref.name, "count");
    assert_eq!(symbol_ref.qualified_name, "count");
    assert_eq!(symbol_ref.full_span, (5, 11));
    assert_eq!(symbol_ref.anchor_span, Some((5, 11)));
}

#[test]
fn declaration_binding_variable_is_not_projected_as_reference() {
    let binding = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "decl_only".to_string() },
        loc(3, 13),
    );
    let decl = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(binding),
            attributes: vec![],
            initializer: None,
        },
        loc(0, 13),
    );
    let program = Node::new(NodeKind::Program { statements: vec![decl] }, loc(0, 13));

    let refs = extract_symbol_refs(&program);
    assert!(refs.is_empty());
}

#[test]
fn subroutine_call_and_qualified_refs_are_projected() {
    let pkg_ident =
        Node::new(NodeKind::Identifier { name: "Vendor::Shared".to_string() }, loc(0, 14));
    let call = Node::new(
        NodeKind::FunctionCall {
            name: "Vendor::Tools::run".to_string(),
            args: vec![Node::new(
                NodeKind::Variable { sigil: "$".to_string(), name: "Vendor::state".to_string() },
                loc(30, 43),
            )],
        },
        loc(15, 44),
    );
    let program = Node::new(
        NodeKind::Program {
            statements: vec![
                Node::new(
                    NodeKind::ExpressionStatement { expression: Box::new(pkg_ident) },
                    loc(0, 14),
                ),
                Node::new(
                    NodeKind::ExpressionStatement { expression: Box::new(call) },
                    loc(15, 44),
                ),
            ],
        },
        loc(0, 44),
    );

    let refs = extract_symbol_refs(&program);

    assert_eq!(refs.len(), 3);
    assert_eq!(refs[0].kind, SymbolKind::Package);
    assert_eq!(refs[0].qualified_name, "Vendor::Shared");
    assert_eq!(refs[0].name, "Shared");

    assert_eq!(refs[1].kind, SymbolKind::Subroutine);
    assert_eq!(refs[1].qualified_name, "Vendor::Tools::run");
    assert_eq!(refs[1].name, "run");

    assert_eq!(refs[2].kind, SymbolKind::Variable(VarKind::Scalar));
    assert_eq!(refs[2].qualified_name, "Vendor::state");
    assert_eq!(refs[2].name, "state");
}
