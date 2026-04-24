//! Tests for phase-1 `SymbolRef` extraction.

use perl_ast::{Node, NodeKind, SourceLocation};
use perl_symbol::surface::{SymbolRefKind, extract_symbol_refs};
use perl_symbol::VarKind;

fn loc(start: usize, end: usize) -> SourceLocation {
    SourceLocation { start, end }
}

#[test]
fn test_variable_reference_is_extracted() {
    let var = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "count".to_string() },
        loc(10, 16),
    );
    let stmt = Node::new(NodeKind::ExpressionStatement { expression: Box::new(var) }, loc(10, 16));
    let program = Node::new(NodeKind::Program { statements: vec![stmt] }, loc(0, 17));

    let refs = extract_symbol_refs(&program);

    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].kind, SymbolRefKind::Variable(VarKind::Scalar));
    assert_eq!(refs[0].name, "count");
    assert_eq!(refs[0].full_span, (10, 16));
    assert_eq!(refs[0].anchor_span, (10, 16));
}

#[test]
fn test_subroutine_call_reference_is_extracted() {
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
}

#[test]
fn test_package_qualified_references_are_extracted() {
    let qualified_call = Node::new(
        NodeKind::FunctionCall {
            name: "Util::trim".to_string(),
            args: vec![Node::new(
                NodeKind::Variable { sigil: "$".to_string(), name: "Pkg::value".to_string() },
                loc(12, 23),
            )],
        },
        loc(0, 24),
    );
    let qualified_glob = Node::new(
        NodeKind::Typeglob { name: "Other::Thing".to_string() },
        loc(25, 37),
    );
    let program = Node::new(
        NodeKind::Program { statements: vec![qualified_call, qualified_glob] },
        loc(0, 37),
    );

    let refs = extract_symbol_refs(&program);

    assert_eq!(refs.len(), 3);
    assert_eq!(refs[0].kind, SymbolRefKind::QualifiedSymbol);
    assert_eq!(refs[0].name, "Util::trim");
    assert_eq!(refs[1].kind, SymbolRefKind::QualifiedSymbol);
    assert_eq!(refs[1].name, "Pkg::value");
    assert_eq!(refs[2].kind, SymbolRefKind::QualifiedSymbol);
    assert_eq!(refs[2].name, "Other::Thing");
}
