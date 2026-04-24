use std::error::Error;

use perl_parser_core::Parser;
use perl_symbol::surface::extract_symbol_decls;
use perl_symbol::{SymbolKind, VarKind};
use perl_workspace::workspace::workspace_index::{normalize_var, WorkspaceIndex};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    name: String,
    qualified_name: Option<String>,
    container: Option<String>,
    kind: SymbolKind,
    declarator: Option<String>,
}

fn parse(source: &str) -> Result<perl_parser_core::Node, Box<dyn Error>> {
    let mut parser = Parser::new(source);
    parser.parse().map_err(|err| format!("parse failed: {err}").into())
}

fn surface_rows(source: &str) -> Result<Vec<Row>, Box<dyn Error>> {
    let ast = parse(source)?;
    let rows = extract_symbol_decls(&ast, None)
        .into_iter()
        .map(|decl| Row {
            name: decl.name,
            qualified_name: Some(decl.qualified_name),
            container: decl.container,
            kind: decl.kind,
            declarator: decl.declarator,
        })
        .collect::<Vec<_>>();
    Ok(rows)
}

fn workspace_rows(source: &str) -> Result<Vec<Row>, Box<dyn Error>> {
    let index = WorkspaceIndex::new();
    index
        .index_file_str("file:///workspace-parity.pl", source)
        .map_err(|err| format!("index failed: {err}"))?;
    let rows = index
        .search_symbols("")
        .into_iter()
        .map(|sym| {
            let name = match sym.kind {
                SymbolKind::Variable(_) => normalize_var(&sym.name).1.to_string(),
                _ => sym.name,
            };
            Row {
                name,
                qualified_name: sym.qualified_name,
                container: sym.container_name,
                kind: sym.kind,
                declarator: None,
            }
        })
        .collect::<Vec<_>>();
    Ok(rows)
}

#[test]
fn parity_bank_surfaces_matches_and_known_divergences() -> Result<(), Box<dyn Error>> {
    let core_match_source = "package Demo::Pkg;\nsub run { 1 }\n";
    let surface = surface_rows(core_match_source)?;
    let workspace = workspace_rows(core_match_source)?;

    for row in [
        Row {
            name: "Demo::Pkg".to_string(),
            qualified_name: Some("Demo::Pkg".to_string()),
            container: None,
            kind: SymbolKind::Package,
            declarator: None,
        },
        Row {
            name: "run".to_string(),
            qualified_name: Some("Demo::Pkg::run".to_string()),
            container: Some("Demo::Pkg".to_string()),
            kind: SymbolKind::Subroutine,
            declarator: None,
        },
    ] {
        assert!(surface.contains(&row), "surface missing expected parity row: {:?}", row);
        assert!(workspace.contains(&row), "workspace missing expected parity row: {:?}", row);
    }

    let class_source = "class Demo::Thing { method ping () { 1 } }\n";
    let surface = surface_rows(class_source)?;
    let workspace = workspace_rows(class_source)?;

    let class_row = Row {
        name: "Demo::Thing".to_string(),
        qualified_name: Some("Demo::Thing".to_string()),
        container: None,
        kind: SymbolKind::Class,
        declarator: None,
    };
    assert!(surface.contains(&class_row), "surface should index class declaration");
    assert!(workspace.contains(&class_row), "workspace should index class declaration");

    let method_row = Row {
        name: "ping".to_string(),
        qualified_name: Some("Demo::Thing::ping".to_string()),
        container: Some("Demo::Thing".to_string()),
        kind: SymbolKind::Method,
        declarator: None,
    };
    assert!(surface.contains(&method_row), "surface should index class method declaration");
    assert!(
        !workspace.contains(&method_row),
        "workspace should expose current divergence for class method declarations"
    );

    let divergence_source = "package Demo::Vars;\nmy $s;\nmy @a;\nmy %h;\nour $o;\nuse constant LIMIT => 10;\nuse Const::Fast;\nconst my $FAST => 1;\nuse Readonly;\nReadonly my $RO => 2;\n";
    let surface = surface_rows(divergence_source)?;
    let workspace = workspace_rows(divergence_source)?;

    for row in [
        Row {
            name: "s".to_string(),
            qualified_name: Some("Demo::Vars::s".to_string()),
            container: Some("Demo::Vars".to_string()),
            kind: SymbolKind::Variable(VarKind::Scalar),
            declarator: Some("my".to_string()),
        },
        Row {
            name: "a".to_string(),
            qualified_name: Some("Demo::Vars::a".to_string()),
            container: Some("Demo::Vars".to_string()),
            kind: SymbolKind::Variable(VarKind::Array),
            declarator: Some("my".to_string()),
        },
        Row {
            name: "h".to_string(),
            qualified_name: Some("Demo::Vars::h".to_string()),
            container: Some("Demo::Vars".to_string()),
            kind: SymbolKind::Variable(VarKind::Hash),
            declarator: Some("my".to_string()),
        },
        Row {
            name: "o".to_string(),
            qualified_name: Some("Demo::Vars::o".to_string()),
            container: Some("Demo::Vars".to_string()),
            kind: SymbolKind::Variable(VarKind::Scalar),
            declarator: Some("our".to_string()),
        },
        Row {
            name: "LIMIT".to_string(),
            qualified_name: Some("Demo::Vars::LIMIT".to_string()),
            container: Some("Demo::Vars".to_string()),
            kind: SymbolKind::Constant,
            declarator: None,
        },
        Row {
            name: "FAST".to_string(),
            qualified_name: Some("Demo::Vars::FAST".to_string()),
            container: Some("Demo::Vars".to_string()),
            kind: SymbolKind::Constant,
            declarator: Some("const".to_string()),
        },
        Row {
            name: "RO".to_string(),
            qualified_name: Some("Demo::Vars::RO".to_string()),
            container: Some("Demo::Vars".to_string()),
            kind: SymbolKind::Constant,
            declarator: Some("Readonly".to_string()),
        },
    ] {
        assert!(surface.contains(&row), "surface missing expected divergence row: {:?}", row);
        assert!(
            !workspace.contains(&row),
            "workspace unexpectedly matched known divergence row: {:?}",
            row
        );
    }

    Ok(())
}
