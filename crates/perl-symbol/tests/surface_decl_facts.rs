use perl_semantic_facts::FileId;
use perl_symbol::surface::{SymbolDecl, symbol_decls_to_facts};
use perl_symbol::{SymbolKind, VarKind};

#[test]
fn symbol_decl_adapter_snapshot_is_deterministic() -> Result<(), serde_json::Error> {
    let decls = vec![
        SymbolDecl {
            kind: SymbolKind::Package,
            name: "Foo".to_string(),
            qualified_name: "Foo".to_string(),
            full_span: (0, 13),
            anchor_span: Some((8, 11)),
            container: None,
            declarator: None,
        },
        SymbolDecl {
            kind: SymbolKind::Class,
            name: "Thing".to_string(),
            qualified_name: "Foo::Thing".to_string(),
            full_span: (14, 40),
            anchor_span: None,
            container: Some("Foo".to_string()),
            declarator: None,
        },
        SymbolDecl {
            kind: SymbolKind::Subroutine,
            name: "run".to_string(),
            qualified_name: "Foo::run".to_string(),
            full_span: (41, 70),
            anchor_span: Some((45, 48)),
            container: Some("Foo".to_string()),
            declarator: None,
        },
        SymbolDecl {
            kind: SymbolKind::Method,
            name: "tick".to_string(),
            qualified_name: "Foo::tick".to_string(),
            full_span: (71, 100),
            anchor_span: None,
            container: Some("Foo".to_string()),
            declarator: None,
        },
        SymbolDecl {
            kind: SymbolKind::Variable(VarKind::Scalar),
            name: "counter".to_string(),
            qualified_name: "Foo::counter".to_string(),
            full_span: (101, 120),
            anchor_span: Some((105, 113)),
            container: Some("Foo".to_string()),
            declarator: Some("my".to_string()),
        },
        SymbolDecl {
            kind: SymbolKind::Constant,
            name: "MAX".to_string(),
            qualified_name: "Foo::MAX".to_string(),
            full_span: (121, 140),
            anchor_span: Some((125, 128)),
            container: Some("Foo".to_string()),
            declarator: None,
        },
        SymbolDecl {
            kind: SymbolKind::Label,
            name: "RETRY".to_string(),
            qualified_name: "RETRY".to_string(),
            full_span: (141, 160),
            anchor_span: None,
            container: Some("Foo".to_string()),
            declarator: None,
        },
        SymbolDecl {
            kind: SymbolKind::Format,
            name: "STDOUT".to_string(),
            qualified_name: "Foo::STDOUT".to_string(),
            full_span: (161, 190),
            anchor_span: None,
            container: Some("Foo".to_string()),
            declarator: None,
        },
    ];

    let first = symbol_decls_to_facts(&decls, FileId(77));
    let second = symbol_decls_to_facts(&decls, FileId(77));

    assert_eq!(first, second);
    assert!(first.unsupported.is_empty());

    let json = serde_json::to_string_pretty(&first.entities)?;
    assert_eq!(json, serde_json::to_string_pretty(&second.entities)?);
    Ok(())
}

#[test]
fn symbol_decl_adapter_reports_unsupported_symbol_kinds() {
    let decl = SymbolDecl {
        kind: SymbolKind::Import,
        name: "imported".to_string(),
        qualified_name: "Foo::imported".to_string(),
        full_span: (10, 20),
        anchor_span: Some((12, 20)),
        container: Some("Foo".to_string()),
        declarator: None,
    };

    let facts = symbol_decls_to_facts(&[decl], FileId(1));
    assert!(facts.entities.is_empty());
    assert_eq!(facts.unsupported.len(), 1);
}
