use perl_semantic_facts::FileId;
use perl_symbol::{SymbolDecl, SymbolKind, VarKind, symbol_decls_to_semantic_facts};

#[test]
fn adapter_snapshot_for_symbol_decl_projection() -> Result<(), serde_json::Error> {
    let decls = vec![
        SymbolDecl {
            kind: SymbolKind::Package,
            name: "Foo".to_string(),
            qualified_name: "Foo".to_string(),
            full_span: (0, 15),
            anchor_span: Some((8, 11)),
            container: None,
            declarator: None,
        },
        SymbolDecl {
            kind: SymbolKind::Class,
            name: "Widget".to_string(),
            qualified_name: "Foo::Widget".to_string(),
            full_span: (16, 40),
            anchor_span: None,
            container: Some("Foo".to_string()),
            declarator: None,
        },
        SymbolDecl {
            kind: SymbolKind::Subroutine,
            name: "greet".to_string(),
            qualified_name: "Foo::greet".to_string(),
            full_span: (41, 70),
            anchor_span: Some((45, 50)),
            container: Some("Foo".to_string()),
            declarator: None,
        },
        SymbolDecl {
            kind: SymbolKind::Method,
            name: "wave".to_string(),
            qualified_name: "Foo::wave".to_string(),
            full_span: (71, 95),
            anchor_span: None,
            container: Some("Foo".to_string()),
            declarator: None,
        },
        SymbolDecl {
            kind: SymbolKind::Variable(VarKind::Scalar),
            name: "name".to_string(),
            qualified_name: "Foo::name".to_string(),
            full_span: (96, 114),
            anchor_span: Some((101, 106)),
            container: Some("Foo".to_string()),
            declarator: Some("our".to_string()),
        },
        SymbolDecl {
            kind: SymbolKind::Constant,
            name: "ANSWER".to_string(),
            qualified_name: "Foo::ANSWER".to_string(),
            full_span: (115, 145),
            anchor_span: None,
            container: Some("Foo".to_string()),
            declarator: None,
        },
        SymbolDecl {
            kind: SymbolKind::Format,
            name: "STDOUT".to_string(),
            qualified_name: "Foo::STDOUT".to_string(),
            full_span: (146, 180),
            anchor_span: None,
            container: Some("Foo".to_string()),
            declarator: None,
        },
        SymbolDecl {
            kind: SymbolKind::Label,
            name: "LOOP".to_string(),
            qualified_name: "LOOP".to_string(),
            full_span: (181, 220),
            anchor_span: None,
            container: Some("Foo".to_string()),
            declarator: None,
        },
    ];

    let facts = symbol_decls_to_semantic_facts(&decls, FileId(7));
    let json = serde_json::to_string_pretty(&facts)?;
    assert!(
        json.contains("\"defines_edges\""),
        "serialized JSON should contain 'defines_edges' key"
    );
    assert_eq!(facts.entities.len(), 8, "should have exactly 8 entities");
    assert_eq!(facts.defines_edges.len(), 7, "should have exactly 7 defines edges");
    assert!(facts.unsupported.is_empty(), "should have no unsupported declarations");
    Ok(())
}
