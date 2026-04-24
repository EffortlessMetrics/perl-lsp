//! Parity bank between `perl_symbol::surface::extract_symbol_decls()` and
//! `perl_workspace::workspace::workspace_index` declaration extraction.
//!
//! These tests lock down where both extractors agree today and where they still
//! diverge before consolidating walkers.

use perl_parser_core::Parser;
use perl_symbol::SymbolKind;
use perl_symbol::surface::{SymbolDecl, extract_symbol_decls};
use perl_workspace::workspace::workspace_index::{WorkspaceIndex, WorkspaceSymbol};
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CanonicalDecl {
    kind: SymbolKind,
    name: String,
    qualified_name: String,
    container: Option<String>,
    declarator: Option<String>,
}

fn parse_surface(source: &str) -> Result<Vec<CanonicalDecl>, Box<dyn std::error::Error>> {
    let mut parser = Parser::new(source);
    let ast = parser.parse()?;

    Ok(extract_symbol_decls(&ast, None).into_iter().map(from_surface).collect())
}

fn parse_workspace(source: &str) -> Result<Vec<CanonicalDecl>, Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = Url::parse("file:///tmp/symbol_surface_parity.pm")?;
    index.index_file(uri.clone(), source.to_string())?;

    Ok(index.file_symbols(uri.as_str()).into_iter().map(from_workspace).collect())
}

fn from_surface(decl: SymbolDecl) -> CanonicalDecl {
    CanonicalDecl {
        kind: decl.kind,
        name: decl.name,
        qualified_name: decl.qualified_name,
        container: decl.container,
        declarator: decl.declarator,
    }
}

fn from_workspace(symbol: WorkspaceSymbol) -> CanonicalDecl {
    let (normalized_name, normalized_qualified) = normalize_workspace_name(&symbol);

    CanonicalDecl {
        kind: symbol.kind,
        name: normalized_name.clone(),
        qualified_name: normalized_qualified.unwrap_or(normalized_name),
        container: symbol.container_name,
        declarator: None,
    }
}

fn normalize_workspace_name(symbol: &WorkspaceSymbol) -> (String, Option<String>) {
    let raw_name = symbol.name.clone();
    let normalized_name = raw_name
        .strip_prefix('$')
        .or_else(|| raw_name.strip_prefix('@'))
        .or_else(|| raw_name.strip_prefix('%'))
        .map(str::to_owned)
        .unwrap_or(raw_name);

    let normalized_qualified = symbol.qualified_name.clone().map(|qualified| {
        let without_sigil =
            qualified.replace("::$", "::").replace("::@", "::").replace("::%", "::");
        without_sigil
    });

    (normalized_name, normalized_qualified)
}

fn find_decl<'a>(
    decls: &'a [CanonicalDecl],
    kind: SymbolKind,
    name: &str,
) -> Option<&'a CanonicalDecl> {
    decls.iter().find(|decl| decl.kind == kind && decl.name == name)
}

#[test]
fn parity_bank_matches_for_core_declarations() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
package Bank::Core;
sub run { return 1; }
class Bank::Worker {
    method process ($self) { return 1; }
}
"#;

    let surface = parse_surface(source)?;
    let workspace = parse_workspace(source)?;

    let package_surface = find_decl(&surface, SymbolKind::Package, "Bank::Core")
        .ok_or_else(|| "surface package missing".to_string())?;
    let package_workspace = find_decl(&workspace, SymbolKind::Package, "Bank::Core")
        .ok_or_else(|| "workspace package missing".to_string())?;
    assert_eq!(package_surface.qualified_name, package_workspace.qualified_name);
    assert_eq!(package_surface.container, package_workspace.container);

    let sub_surface = find_decl(&surface, SymbolKind::Subroutine, "run")
        .ok_or_else(|| "surface sub missing".to_string())?;
    let sub_workspace = find_decl(&workspace, SymbolKind::Subroutine, "run")
        .ok_or_else(|| "workspace sub missing".to_string())?;
    assert_eq!(sub_surface.qualified_name, sub_workspace.qualified_name);
    assert_eq!(sub_surface.container, sub_workspace.container);

    let class_surface = find_decl(&surface, SymbolKind::Class, "Bank::Worker")
        .ok_or_else(|| "surface class missing".to_string())?;
    let class_workspace = find_decl(&workspace, SymbolKind::Class, "Bank::Worker")
        .ok_or_else(|| "workspace class missing".to_string())?;
    // Known divergence: surface re-qualifies an already-qualified class name when
    // declared under an active package context; workspace preserves the class name.
    assert_eq!(class_surface.qualified_name, "Bank::Core::Bank::Worker");
    assert_eq!(class_workspace.qualified_name, "Bank::Worker");
    assert_eq!(class_surface.container, Some("Bank::Core".to_string()));
    assert_eq!(class_workspace.container, None);

    let method_surface = find_decl(&surface, SymbolKind::Method, "process")
        .ok_or_else(|| "surface method missing".to_string())?;
    assert_eq!(method_surface.qualified_name, "Bank::Worker::process");
    assert_eq!(method_surface.container, Some("Bank::Worker".to_string()));
    assert!(
        find_decl(&workspace, SymbolKind::Method, "process").is_none(),
        "workspace unexpectedly indexes class body methods; update parity test if traversal changes"
    );

    Ok(())
}

#[test]
fn parity_bank_matches_variable_shape_but_not_declarator() -> Result<(), Box<dyn std::error::Error>>
{
    let source = r#"
package Bank::Vars;
my $scalar = 1;
our @items = (1, 2);
my %lookup = (a => 1);
"#;

    let surface = parse_surface(source)?;
    let workspace = parse_workspace(source)?;

    for (kind, name, expected_declarator) in [
        (SymbolKind::Variable(perl_symbol::VarKind::Scalar), "scalar", "my"),
        (SymbolKind::Variable(perl_symbol::VarKind::Array), "items", "our"),
        (SymbolKind::Variable(perl_symbol::VarKind::Hash), "lookup", "my"),
    ] {
        let s = find_decl(&surface, kind, name)
            .ok_or_else(|| format!("surface variable missing: {name}"))?;
        let w = find_decl(&workspace, kind, name)
            .ok_or_else(|| format!("workspace variable missing: {name}"))?;

        // Known divergence: workspace symbols currently keep variable
        // `qualified_name` empty, while surface synthesizes package-qualified names.
        assert!(
            w.qualified_name == name,
            "workspace variable qualified_name unexpectedly changed for {name}"
        );
        assert!(
            s.qualified_name.ends_with(name),
            "surface qualified_name should end with variable name for {name}"
        );
        assert_eq!(s.container, w.container, "container mismatch for {name}");
        assert_eq!(s.declarator.as_deref(), Some(expected_declarator));
        assert_eq!(
            w.declarator, None,
            "workspace unexpectedly captured declarator for {name}; update parity test if this changed"
        );
    }

    Ok(())
}

#[test]
fn parity_bank_explicit_divergences_for_constant_wrappers() -> Result<(), Box<dyn std::error::Error>>
{
    let use_constant = r#"
package Bank::Const;
use constant PI => 3.14159;
"#;
    let surface_use_constant = parse_surface(use_constant)?;
    let workspace_use_constant = parse_workspace(use_constant)?;

    let surface_pi = find_decl(&surface_use_constant, SymbolKind::Constant, "PI")
        .ok_or_else(|| "surface use constant PI missing".to_string())?;
    let workspace_pi = find_decl(&workspace_use_constant, SymbolKind::Subroutine, "PI")
        .ok_or_else(|| "workspace use constant PI missing".to_string())?;
    assert_eq!(surface_pi.qualified_name, workspace_pi.qualified_name);
    assert_eq!(surface_pi.container, workspace_pi.container);

    let const_fast = r#"
package Bank::Fast;
use Const::Fast;
const my $FAST_FLAG => 1;
"#;
    let surface_const_fast = parse_surface(const_fast)?;
    let workspace_const_fast = parse_workspace(const_fast)?;

    let const_fast_surface = find_decl(&surface_const_fast, SymbolKind::Constant, "FAST_FLAG")
        .ok_or_else(|| "surface Const::Fast symbol missing".to_string())?;
    assert_eq!(const_fast_surface.declarator.as_deref(), Some("const"));
    assert!(
        find_decl(&workspace_const_fast, SymbolKind::Constant, "FAST_FLAG").is_none(),
        "workspace unexpectedly emits Const::Fast constants; update parity test if support lands"
    );

    let readonly = r#"
package Bank::ReadOnly;
use Readonly;
Readonly my $READ_FLAG => 1;
"#;
    let surface_readonly = parse_surface(readonly)?;
    let workspace_readonly = parse_workspace(readonly)?;

    let readonly_surface = find_decl(&surface_readonly, SymbolKind::Constant, "READ_FLAG")
        .ok_or_else(|| "surface Readonly symbol missing".to_string())?;
    assert_eq!(readonly_surface.declarator.as_deref(), Some("Readonly"));
    assert!(
        find_decl(&workspace_readonly, SymbolKind::Constant, "READ_FLAG").is_none(),
        "workspace unexpectedly emits Readonly constants; update parity test if support lands"
    );

    Ok(())
}
