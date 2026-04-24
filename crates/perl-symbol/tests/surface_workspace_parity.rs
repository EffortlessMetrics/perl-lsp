//! Bank of declaration-shape fixtures used by the workspace parity suite.
//!
//! This file focuses on what `perl_symbol::surface::extract_symbol_decls` emits
//! for representative constructs that `perl-workspace-index` also indexes.

use perl_parser_core::Parser;
use perl_symbol::surface::extract_symbol_decls;
use perl_symbol::{SymbolKind, VarKind};

#[derive(Debug, Clone, PartialEq, Eq)]
struct SurfaceRow {
    name: String,
    qualified_name: String,
    container: Option<String>,
    kind: SymbolKind,
    declarator: Option<String>,
}

fn surface_rows(source: &str) -> Result<Vec<SurfaceRow>, Box<dyn std::error::Error>> {
    let mut parser = Parser::new(source);
    let ast = parser.parse()?;

    Ok(extract_symbol_decls(&ast, None)
        .into_iter()
        .map(|decl| SurfaceRow {
            name: decl.name,
            qualified_name: decl.qualified_name,
            container: decl.container,
            kind: decl.kind,
            declarator: decl.declarator,
        })
        .collect())
}

#[test]
fn parity_fixture_bank_covers_representative_constructs() -> Result<(), Box<dyn std::error::Error>>
{
    let source = r#"
package Alpha;
sub run { }

class App::Model {
    method save ($self) { }
}

my $lex = 1;
our @EXPORT = qw(run);
my %cfg = ();
use constant PI => 3.14;
use Const::Fast;
const my $CF_RATE => 7;
use Readonly;
Readonly my $RO_LIMIT => 10;
"#;

    let rows = surface_rows(source)?;

    assert!(rows.iter().any(|row| {
        row.name == "Alpha"
            && row.qualified_name == "Alpha"
            && row.container.is_none()
            && row.kind == SymbolKind::Package
    }));

    assert!(rows.iter().any(|row| {
        row.name == "run"
            && row.qualified_name == "Alpha::run"
            && row.container.as_deref() == Some("Alpha")
            && row.kind == SymbolKind::Subroutine
    }));

    assert!(rows.iter().any(|row| {
        row.name == "App::Model"
            && row.qualified_name == "Alpha::App::Model"
            && row.container.as_deref() == Some("Alpha")
            && row.kind == SymbolKind::Class
    }));

    assert!(rows.iter().any(|row| {
        row.name == "save"
            && row.qualified_name == "App::Model::save"
            && row.container.as_deref() == Some("App::Model")
            && row.kind == SymbolKind::Method
    }));

    assert!(rows.iter().any(|row| {
        row.name == "lex"
            && row.qualified_name == "Alpha::lex"
            && row.container.as_deref() == Some("Alpha")
            && row.kind == SymbolKind::Variable(VarKind::Scalar)
            && row.declarator.as_deref() == Some("my")
    }));

    assert!(rows.iter().any(|row| {
        row.name == "EXPORT"
            && row.qualified_name == "Alpha::EXPORT"
            && row.container.as_deref() == Some("Alpha")
            && row.kind == SymbolKind::Variable(VarKind::Array)
            && row.declarator.as_deref() == Some("our")
    }));

    assert!(rows.iter().any(|row| {
        row.name == "cfg"
            && row.qualified_name == "Alpha::cfg"
            && row.container.as_deref() == Some("Alpha")
            && row.kind == SymbolKind::Variable(VarKind::Hash)
            && row.declarator.as_deref() == Some("my")
    }));

    assert!(rows.iter().any(|row| {
        row.name == "PI"
            && row.qualified_name == "Alpha::PI"
            && row.container.as_deref() == Some("Alpha")
            && row.kind == SymbolKind::Constant
            && row.declarator.is_none()
    }));

    assert!(rows.iter().any(|row| {
        row.name == "CF_RATE"
            && row.qualified_name == "Alpha::CF_RATE"
            && row.container.as_deref() == Some("Alpha")
            && row.kind == SymbolKind::Constant
            && row.declarator.as_deref() == Some("const")
    }));

    assert!(rows.iter().any(|row| {
        row.name == "RO_LIMIT"
            && row.qualified_name == "Alpha::RO_LIMIT"
            && row.container.as_deref() == Some("Alpha")
            && row.kind == SymbolKind::Constant
            && row.declarator.as_deref() == Some("Readonly")
    }));

    Ok(())
}
