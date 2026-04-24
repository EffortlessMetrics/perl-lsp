use std::error::Error;

use perl_parser_core::Parser;
use perl_symbol::surface::{extract_symbol_decls, SymbolDecl};
use perl_symbol::SymbolKind;
use perl_workspace::workspace::workspace_index::{normalize_var, WorkspaceIndex};

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeclRow {
    name: String,
    qualified_name: Option<String>,
    container: Option<String>,
    kind: SymbolKind,
    declarator: Option<String>,
}

#[derive(Debug)]
struct Case<'a> {
    id: &'a str,
    source: &'a str,
    expect_matches: Vec<DeclRow>,
    expect_divergences: Vec<DeclRow>,
}

fn parse_program(source: &str) -> Result<perl_parser_core::Node, Box<dyn Error>> {
    let mut parser = Parser::new(source);
    parser.parse().map_err(|err| format!("parse failed: {err}").into())
}

fn from_surface(decl: &SymbolDecl) -> DeclRow {
    DeclRow {
        name: decl.name.clone(),
        qualified_name: Some(decl.qualified_name.clone()),
        container: decl.container.clone(),
        kind: decl.kind.clone(),
        declarator: decl.declarator.clone(),
    }
}

fn from_workspace(symbol: &perl_workspace::workspace::workspace_index::WorkspaceSymbol) -> DeclRow {
    let normalized_name = match symbol.kind {
        SymbolKind::Variable(_) => {
            let (_, bare_name) = normalize_var(&symbol.name);
            bare_name.to_string()
        }
        _ => symbol.name.clone(),
    };

    DeclRow {
        name: normalized_name,
        qualified_name: symbol.qualified_name.clone(),
        container: symbol.container_name.clone(),
        kind: symbol.kind.clone(),
        declarator: None,
    }
}

fn collect_surface_rows(source: &str) -> Result<Vec<DeclRow>, Box<dyn Error>> {
    let ast = parse_program(source)?;
    Ok(extract_symbol_decls(&ast, None).iter().map(from_surface).collect::<Vec<_>>())
}

fn collect_workspace_rows(source: &str) -> Result<Vec<DeclRow>, Box<dyn Error>> {
    let index = WorkspaceIndex::new();
    index
        .index_file_str("file:///parity.pl", source)
        .map_err(|err| format!("index failed: {err}"))?;
    Ok(index.search_symbols("").iter().map(from_workspace).collect::<Vec<_>>())
}

#[test]
fn surface_workspace_parity_bank() -> Result<(), Box<dyn Error>> {
    let cases = vec![
        Case {
            id: "package_and_sub",
            source: "package Demo::Pkg;\nsub run { return 1; }\n",
            expect_matches: vec![
                DeclRow {
                    name: "Demo::Pkg".to_string(),
                    qualified_name: Some("Demo::Pkg".to_string()),
                    container: None,
                    kind: SymbolKind::Package,
                    declarator: None,
                },
                DeclRow {
                    name: "run".to_string(),
                    qualified_name: Some("Demo::Pkg::run".to_string()),
                    container: Some("Demo::Pkg".to_string()),
                    kind: SymbolKind::Subroutine,
                    declarator: None,
                },
            ],
            expect_divergences: vec![],
        },
        Case {
            id: "class_and_method",
            source: "class Demo::Thing { method ping () { return 1; } }\n",
            expect_matches: vec![DeclRow {
                name: "Demo::Thing".to_string(),
                qualified_name: Some("Demo::Thing".to_string()),
                container: None,
                kind: SymbolKind::Class,
                declarator: None,
            }],
            expect_divergences: vec![DeclRow {
                name: "ping".to_string(),
                qualified_name: Some("Demo::Thing::ping".to_string()),
                container: Some("Demo::Thing".to_string()),
                kind: SymbolKind::Method,
                declarator: None,
            }],
        },
        Case {
            id: "my_scalar",
            source: "package Vars;\nmy $count = 1;\n",
            expect_matches: vec![],
            expect_divergences: vec![DeclRow {
                name: "count".to_string(),
                qualified_name: Some("Vars::count".to_string()),
                container: Some("Vars".to_string()),
                kind: SymbolKind::Variable(perl_symbol::VarKind::Scalar),
                declarator: Some("my".to_string()),
            }],
        },
        Case {
            id: "array_and_hash",
            source: "package Vars;\nmy @items;\nmy %lookup;\n",
            expect_matches: vec![],
            expect_divergences: vec![
                DeclRow {
                    name: "items".to_string(),
                    qualified_name: Some("Vars::items".to_string()),
                    container: Some("Vars".to_string()),
                    kind: SymbolKind::Variable(perl_symbol::VarKind::Array),
                    declarator: Some("my".to_string()),
                },
                DeclRow {
                    name: "lookup".to_string(),
                    qualified_name: Some("Vars::lookup".to_string()),
                    container: Some("Vars".to_string()),
                    kind: SymbolKind::Variable(perl_symbol::VarKind::Hash),
                    declarator: Some("my".to_string()),
                },
            ],
        },
        Case {
            id: "our_vs_my",
            source: "package Vars;\nour $shared = 1;\nmy $local = 2;\n",
            expect_matches: vec![],
            expect_divergences: vec![
                DeclRow {
                    name: "shared".to_string(),
                    qualified_name: Some("Vars::shared".to_string()),
                    container: Some("Vars".to_string()),
                    kind: SymbolKind::Variable(perl_symbol::VarKind::Scalar),
                    declarator: Some("our".to_string()),
                },
                DeclRow {
                    name: "local".to_string(),
                    qualified_name: Some("Vars::local".to_string()),
                    container: Some("Vars".to_string()),
                    kind: SymbolKind::Variable(perl_symbol::VarKind::Scalar),
                    declarator: Some("my".to_string()),
                },
            ],
        },
        Case {
            id: "use_constant",
            source: "package Demo::Const;\nuse constant LIMIT => 10;\n",
            expect_matches: vec![],
            expect_divergences: vec![DeclRow {
                name: "LIMIT".to_string(),
                qualified_name: Some("Demo::Const::LIMIT".to_string()),
                container: Some("Demo::Const".to_string()),
                kind: SymbolKind::Constant,
                declarator: None,
            }],
        },
        Case {
            id: "const_fast",
            source: "package Demo::Fast;\nuse Const::Fast;\nconst my $FAST => 1;\n",
            expect_matches: vec![],
            expect_divergences: vec![DeclRow {
                name: "FAST".to_string(),
                qualified_name: Some("Demo::Fast::FAST".to_string()),
                container: Some("Demo::Fast".to_string()),
                kind: SymbolKind::Constant,
                declarator: Some("const".to_string()),
            }],
        },
        Case {
            id: "readonly",
            source: "package Demo::RO;\nuse Readonly;\nReadonly my $LIMIT => 3;\n",
            expect_matches: vec![],
            expect_divergences: vec![DeclRow {
                name: "LIMIT".to_string(),
                qualified_name: Some("Demo::RO::LIMIT".to_string()),
                container: Some("Demo::RO".to_string()),
                kind: SymbolKind::Constant,
                declarator: Some("Readonly".to_string()),
            }],
        },
    ];

    for case in &cases {
        let surface = collect_surface_rows(case.source)?;
        let workspace = collect_workspace_rows(case.source)?;

        for expected_match in &case.expect_matches {
            assert!(
                surface.contains(expected_match),
                "case={} expected matched row missing in surface: {:?}; surface={:?}",
                case.id,
                expected_match,
                surface
            );
            assert!(
                workspace.contains(expected_match),
                "case={} expected matched row missing in workspace: {:?}; workspace={:?}",
                case.id,
                expected_match,
                workspace
            );
        }

        for expected_divergence in &case.expect_divergences {
            assert!(
                surface.contains(expected_divergence),
                "case={} expected divergence row missing in surface: {:?}; surface={:?}",
                case.id,
                expected_divergence,
                surface
            );
            assert!(
                !workspace.contains(expected_divergence),
                "case={} divergence should remain unresolved in workspace for row {:?}; workspace={:?}",
                case.id,
                expected_divergence,
                workspace
            );
        }
    }

    Ok(())
}
