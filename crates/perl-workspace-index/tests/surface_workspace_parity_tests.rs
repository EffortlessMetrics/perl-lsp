//! Parity and divergence bank between `perl-symbol` surface extraction and
//! `perl-workspace-index` declaration extraction.

use perl_parser_core::Parser;
use perl_symbol::surface::extract_symbol_decls;
use perl_symbol::{SymbolKind, VarKind};
use perl_workspace::workspace::workspace_index::WorkspaceIndex;
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeclRow {
    name: String,
    qualified_name: String,
    container: Option<String>,
    kind: SymbolKind,
    declarator: Option<String>,
}

fn sort_rows(rows: &mut [DeclRow]) {
    rows.sort_by(|left, right| {
        (
            left.container.clone(),
            left.qualified_name.clone(),
            format!("{:?}", left.kind),
            left.declarator.clone(),
        )
            .cmp(&(
                right.container.clone(),
                right.qualified_name.clone(),
                format!("{:?}", right.kind),
                right.declarator.clone(),
            ))
    });
}

fn parse_surface_rows(source: &str) -> Result<Vec<DeclRow>, Box<dyn std::error::Error>> {
    let mut parser = Parser::new(source);
    let ast = parser.parse()?;
    let mut rows = extract_symbol_decls(&ast, None)
        .into_iter()
        .map(|decl| DeclRow {
            name: decl.name,
            qualified_name: decl.qualified_name,
            container: decl.container,
            kind: decl.kind,
            declarator: decl.declarator,
        })
        .collect::<Vec<_>>();
    sort_rows(&mut rows);
    Ok(rows)
}

fn parse_workspace_rows(source: &str) -> Result<Vec<DeclRow>, Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = Url::parse("file:///parity/sample.pm")?;
    index.index_file(uri.clone(), source.to_string())?;

    let mut rows = index
        .file_symbols(uri.as_str())
        .into_iter()
        .map(|symbol| {
            let (name, qualified_name, kind) = match symbol.kind {
                SymbolKind::Variable(kind) => {
                    let sigil = match kind {
                        VarKind::Scalar => '$',
                        VarKind::Array => '@',
                        VarKind::Hash => '%',
                    };
                    let bare_name = symbol.name.trim_start_matches(sigil).to_string();
                    let qualified = symbol.container_name.as_ref().map_or_else(
                        || bare_name.clone(),
                        |container| format!("{container}::{bare_name}"),
                    );
                    (bare_name, qualified, SymbolKind::Variable(kind))
                }
                other => {
                    let qualified = symbol.qualified_name.clone().unwrap_or_else(|| {
                        symbol.container_name.as_ref().map_or_else(
                            || symbol.name.clone(),
                            |container| format!("{container}::{}", symbol.name),
                        )
                    });
                    (symbol.name, qualified, other)
                }
            };

            DeclRow {
                name,
                qualified_name,
                container: symbol.container_name,
                kind,
                declarator: None,
            }
        })
        .collect::<Vec<_>>();
    sort_rows(&mut rows);
    Ok(rows)
}

fn has_row(rows: &[DeclRow], needle: &DeclRow) -> bool {
    rows.iter().any(|row| row == needle)
}

#[test]
fn surface_and_workspace_match_for_core_named_declarations()
-> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
package Alpha;
sub run { }
"#;

    let surface_rows = parse_surface_rows(source)?;
    let workspace_rows = parse_workspace_rows(source)?;

    let expected_rows = vec![
        DeclRow {
            name: "Alpha".to_string(),
            qualified_name: "Alpha".to_string(),
            container: None,
            kind: SymbolKind::Package,
            declarator: None,
        },
        DeclRow {
            name: "run".to_string(),
            qualified_name: "Alpha::run".to_string(),
            container: Some("Alpha".to_string()),
            kind: SymbolKind::Subroutine,
            declarator: None,
        },
    ];

    for expected in expected_rows {
        assert!(has_row(&surface_rows, &expected), "surface missing expected row: {expected:?}");
        assert!(
            has_row(&workspace_rows, &expected),
            "workspace-index missing expected row: {expected:?}"
        );
    }

    Ok(())
}

#[test]
fn surface_and_workspace_divergence_bank_is_explicit_and_actionable()
-> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
package Alpha;
class App::Model {
    method save ($self) { }
}
my $lex = 1;
our @exports = qw(run);
my %cfg = ();
use constant PI => 3.14;
use Const::Fast;
const my $CF_RATE => 7;
use Readonly;
Readonly my $RO_LIMIT => 10;
"#;

    let surface_rows = parse_surface_rows(source)?;
    let workspace_rows = parse_workspace_rows(source)?;

    let my_lex_surface = DeclRow {
        name: "lex".to_string(),
        qualified_name: "Alpha::lex".to_string(),
        container: Some("Alpha".to_string()),
        kind: SymbolKind::Variable(VarKind::Scalar),
        declarator: Some("my".to_string()),
    };
    assert!(has_row(&surface_rows, &my_lex_surface));
    assert!(
        !has_row(&workspace_rows, &my_lex_surface),
        "workspace-index does not preserve variable declarator yet (expected divergence: add declarator to workspace symbol model)"
    );

    let our_exports_surface = DeclRow {
        name: "exports".to_string(),
        qualified_name: "Alpha::exports".to_string(),
        container: Some("Alpha".to_string()),
        kind: SymbolKind::Variable(VarKind::Array),
        declarator: Some("our".to_string()),
    };
    assert!(has_row(&surface_rows, &our_exports_surface));
    assert!(
        !has_row(&workspace_rows, &our_exports_surface),
        "workspace-index does not distinguish `our` vs `my` declarators yet"
    );

    let hash_surface = DeclRow {
        name: "cfg".to_string(),
        qualified_name: "Alpha::cfg".to_string(),
        container: Some("Alpha".to_string()),
        kind: SymbolKind::Variable(VarKind::Hash),
        declarator: Some("my".to_string()),
    };
    assert!(has_row(&surface_rows, &hash_surface));
    assert!(
        !has_row(&workspace_rows, &hash_surface),
        "workspace-index does not preserve variable declarator for hash declarations"
    );

    let use_constant_surface = DeclRow {
        name: "PI".to_string(),
        qualified_name: "Alpha::PI".to_string(),
        container: Some("Alpha".to_string()),
        kind: SymbolKind::Constant,
        declarator: None,
    };
    assert!(has_row(&surface_rows, &use_constant_surface));
    assert!(
        !has_row(&workspace_rows, &use_constant_surface),
        "workspace-index indexes `use constant` as Subroutine; align SymbolKind to Constant for parity"
    );

    let const_fast_surface = DeclRow {
        name: "CF_RATE".to_string(),
        qualified_name: "Alpha::CF_RATE".to_string(),
        container: Some("Alpha".to_string()),
        kind: SymbolKind::Constant,
        declarator: Some("const".to_string()),
    };
    assert!(has_row(&surface_rows, &const_fast_surface));
    assert!(
        !has_row(&workspace_rows, &const_fast_surface),
        "workspace-index currently misses Const::Fast constant extraction; add const-call handling"
    );

    let readonly_surface = DeclRow {
        name: "RO_LIMIT".to_string(),
        qualified_name: "Alpha::RO_LIMIT".to_string(),
        container: Some("Alpha".to_string()),
        kind: SymbolKind::Constant,
        declarator: Some("Readonly".to_string()),
    };
    assert!(has_row(&surface_rows, &readonly_surface));
    assert!(
        !has_row(&workspace_rows, &readonly_surface),
        "workspace-index currently misses Readonly constant extraction; add Readonly-call handling"
    );

    let class_surface = DeclRow {
        name: "App::Model".to_string(),
        qualified_name: "Alpha::App::Model".to_string(),
        container: Some("Alpha".to_string()),
        kind: SymbolKind::Class,
        declarator: None,
    };
    assert!(has_row(&surface_rows, &class_surface));
    assert!(
        !has_row(&workspace_rows, &class_surface),
        "workspace-index class declaration uses unqualified class name and drops package container; align class qualification semantics"
    );

    let method_surface = DeclRow {
        name: "save".to_string(),
        qualified_name: "App::Model::save".to_string(),
        container: Some("App::Model".to_string()),
        kind: SymbolKind::Method,
        declarator: None,
    };
    assert!(has_row(&surface_rows, &method_surface));
    assert!(
        !has_row(&workspace_rows, &method_surface),
        "workspace-index does not walk class bodies when indexing methods; descend into `NodeKind::Class` body for parity"
    );

    Ok(())
}
