//! Representative construct bank for workspace parity coverage.
//!
//! The direct extractor-vs-index comparison lives in
//! `perl-workspace-index/tests/symbol_surface_parity_tests.rs`.
//! This file keeps the `perl-symbol` side of the same snippets readable.

use perl_parser_core::Parser;
use perl_symbol::surface::extract_symbol_decls;
use perl_symbol::{SymbolKind, VarKind};

#[test]
fn representative_construct_bank_extracts_expected_surface_symbols()
-> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
package Bank::Pkg;
sub perform { return 1; }
class Bank::Worker {
    method process ($self) { return 1; }
}
my $scalar = 1;
our @items = (1, 2);
my %lookup = (a => 1);
use constant PI => 3.14159;
use Const::Fast;
const my $FAST_FLAG => 1;
use Readonly;
Readonly my $READ_FLAG => 1;
"#;

    let mut parser = Parser::new(source);
    let ast = parser.parse()?;
    let decls = extract_symbol_decls(&ast, None);

    assert!(decls.iter().any(|d| d.kind == SymbolKind::Package && d.name == "Bank::Pkg"));
    assert!(decls.iter().any(|d| d.kind == SymbolKind::Subroutine && d.name == "perform"));
    assert!(decls.iter().any(|d| d.kind == SymbolKind::Class && d.name == "Bank::Worker"));
    assert!(decls.iter().any(|d| d.kind == SymbolKind::Method && d.name == "process"));
    assert!(
        decls.iter().any(|d| d.kind == SymbolKind::Variable(VarKind::Scalar) && d.name == "scalar")
    );
    assert!(
        decls.iter().any(|d| d.kind == SymbolKind::Variable(VarKind::Array) && d.name == "items")
    );
    assert!(
        decls.iter().any(|d| d.kind == SymbolKind::Variable(VarKind::Hash) && d.name == "lookup")
    );
    assert!(decls.iter().any(|d| d.kind == SymbolKind::Constant && d.name == "PI"));
    assert!(decls.iter().any(|d| d.kind == SymbolKind::Constant
        && d.name == "FAST_FLAG"
        && d.declarator.as_deref() == Some("const")));
    assert!(decls.iter().any(|d| d.kind == SymbolKind::Constant
        && d.name == "READ_FLAG"
        && d.declarator.as_deref() == Some("Readonly")));

    Ok(())
}
