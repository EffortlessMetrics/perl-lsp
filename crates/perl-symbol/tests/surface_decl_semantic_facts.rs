use perl_semantic_facts::{EdgeKind, EntityKind, FileId, ScopeId};
use perl_symbol::surface::{SymbolDecl, symbol_decls_to_semantic_facts};
use perl_symbol::{SymbolKind, VarKind};

#[test]
fn adapter_maps_supported_decl_kinds_and_is_deterministic() -> Result<(), serde_json::Error> {
    let decls = vec![
        SymbolDecl { kind: SymbolKind::Package, name: "Foo".into(), qualified_name: "Foo".into(), full_span: (0, 11), anchor_span: Some((8, 11)), container: None, declarator: None },
        SymbolDecl { kind: SymbolKind::Class, name: "Thing".into(), qualified_name: "Foo::Thing".into(), full_span: (12, 30), anchor_span: None, container: Some("Foo".into()), declarator: None },
        SymbolDecl { kind: SymbolKind::Subroutine, name: "run".into(), qualified_name: "Foo::run".into(), full_span: (31, 50), anchor_span: Some((35, 38)), container: Some("Foo".into()), declarator: None },
        SymbolDecl { kind: SymbolKind::Method, name: "new".into(), qualified_name: "Foo::Thing::new".into(), full_span: (51, 70), anchor_span: None, container: Some("Foo::Thing".into()), declarator: None },
        SymbolDecl { kind: SymbolKind::Variable(VarKind::Scalar), name: "x".into(), qualified_name: "Foo::x".into(), full_span: (71, 80), anchor_span: Some((74, 76)), container: Some("Foo".into()), declarator: Some("my".into()) },
        SymbolDecl { kind: SymbolKind::Constant, name: "PI".into(), qualified_name: "Foo::PI".into(), full_span: (81, 100), anchor_span: None, container: Some("Foo".into()), declarator: None },
        SymbolDecl { kind: SymbolKind::Label, name: "LOOP".into(), qualified_name: "LOOP".into(), full_span: (101, 110), anchor_span: None, container: Some("Foo".into()), declarator: None },
        SymbolDecl { kind: SymbolKind::Format, name: "STDOUT".into(), qualified_name: "Foo::STDOUT".into(), full_span: (111, 130), anchor_span: None, container: Some("Foo".into()), declarator: None },
    ];

    let first = symbol_decls_to_semantic_facts(&decls, FileId(7), Some(ScopeId(9)));
    let second = symbol_decls_to_semantic_facts(&decls, FileId(7), Some(ScopeId(9)));

    assert_eq!(first, second);
    assert!(first.unsupported.is_empty());
    assert_eq!(first.entities.len(), 8);
    assert!(first.edges.iter().all(|edge| edge.kind == EdgeKind::Defines));

    let kinds: Vec<EntityKind> = first.entities.iter().map(|e| e.kind).collect();
    assert_eq!(
        kinds,
        vec![
            EntityKind::Package,
            EntityKind::Class,
            EntityKind::Subroutine,
            EntityKind::Method,
            EntityKind::Variable,
            EntityKind::Constant,
            EntityKind::Label,
            EntityKind::Format,
        ]
    );

    let json = serde_json::to_string_pretty(&first.entities)?;
    assert_eq!(json, serde_json::to_string_pretty(&second.entities)?);
    Ok(())
}

#[test]
fn adapter_reports_unsupported_decl_kinds_explicitly() {
    let decls = vec![SymbolDecl {
        kind: SymbolKind::Role,
        name: "Roley".into(),
        qualified_name: "Foo::Roley".into(),
        full_span: (0, 10),
        anchor_span: None,
        container: Some("Foo".into()),
        declarator: None,
    }];

    let facts = symbol_decls_to_semantic_facts(&decls, FileId(1), None);

    assert!(facts.anchors.is_empty());
    assert!(facts.entities.is_empty());
    assert!(facts.edges.is_empty());
    assert_eq!(facts.unsupported.len(), 1);
    assert_eq!(facts.unsupported[0].symbol_name, "Foo::Roley");
    assert_eq!(facts.unsupported[0].symbol_kind, SymbolKind::Role);
}
