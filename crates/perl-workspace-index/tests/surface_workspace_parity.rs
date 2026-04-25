//! Parity bank comparing `perl_symbol::extract_symbol_decls()` with
//! `perl_workspace` declaration extraction.
//!
//! Goal: make matches and divergences explicit before consolidating extraction.

use anyhow::{Result, anyhow};
use perl_parser_core::Parser;
use perl_symbol::{SymbolDecl, SymbolKind, extract_symbol_decls};
use perl_workspace::workspace::workspace_index::{WorkspaceIndex, WorkspaceSymbol};
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeclView {
    name: String,
    qualified_name: Option<String>,
    container: Option<String>,
    kind: SymbolKind,
    declarator: Option<String>,
}

struct ParityCase {
    label: &'static str,
    src: &'static str,
    symbol_name: &'static str,
    expect_core_match: bool,
    actionable_divergence: Option<&'static str>,
}

fn parse(src: &str) -> Result<perl_parser_core::Node> {
    let mut parser = Parser::new(src);
    Ok(parser.parse()?)
}

fn surface_views(src: &str) -> Result<Vec<DeclView>> {
    let ast = parse(src)?;
    let decls = extract_symbol_decls(&ast, Some("main"));
    Ok(decls.iter().map(from_surface).collect())
}

fn workspace_views(src: &str) -> Result<Vec<DeclView>> {
    let index = WorkspaceIndex::new();
    let uri = Url::parse("file:///parity_case.pl")?;
    index.index_file(uri, src.to_string()).map_err(|err| anyhow!("index_file failed: {err}"))?;

    Ok(index.all_symbols().iter().map(from_workspace).collect())
}

fn from_surface(decl: &SymbolDecl) -> DeclView {
    DeclView {
        name: decl.name.clone(),
        qualified_name: Some(decl.qualified_name.clone()),
        container: decl.container.clone(),
        kind: decl.kind,
        declarator: decl.declarator.clone(),
    }
}

fn from_workspace(symbol: &WorkspaceSymbol) -> DeclView {
    DeclView {
        // normalize sigil-prefixed variable names for direct comparison
        name: symbol.name.trim_start_matches(['$', '@', '%']).to_string(),
        qualified_name: symbol.qualified_name.clone().or_else(|| {
            symbol.container_name.as_ref().map(|container| format!("{container}::{}", symbol.name))
        }),
        container: symbol.container_name.clone(),
        kind: symbol.kind,
        declarator: None,
    }
}

fn find<'a>(decls: &'a [DeclView], name: &str) -> Option<&'a DeclView> {
    decls.iter().find(|decl| decl.name == name)
}

#[test]
fn surface_workspace_parity_bank() -> Result<()> {
    let cases = vec![
        ParityCase {
            label: "package declaration",
            src: "package My::Pkg; 1;",
            symbol_name: "My::Pkg",
            expect_core_match: false,
            actionable_divergence: Some(
                "surface records implicit `main` as package container; workspace leaves container empty",
            ),
        },
        ParityCase {
            label: "subroutine in package",
            src: "package My::Pkg; sub run { 1 }",
            symbol_name: "run",
            expect_core_match: true,
            actionable_divergence: None,
        },
        ParityCase {
            label: "method declaration",
            src: "class Clock { method tick () { 1 } }",
            symbol_name: "tick",
            expect_core_match: false,
            actionable_divergence: Some(
                "workspace index currently misses methods declared inside `class { ... }` bodies",
            ),
        },
        ParityCase {
            label: "my variable declaration",
            src: "my $count = 1;",
            symbol_name: "count",
            expect_core_match: false,
            actionable_divergence: Some(
                "workspace index does not preserve variable declarator (`my` vs `our`)",
            ),
        },
        ParityCase {
            label: "our variable declaration",
            src: "our $VERSION = '1.0';",
            symbol_name: "VERSION",
            expect_core_match: false,
            actionable_divergence: Some(
                "workspace index does not preserve variable declarator (`my` vs `our`)",
            ),
        },
        ParityCase {
            label: "use constant",
            src: "package C; use constant PI => 3.14;",
            symbol_name: "PI",
            expect_core_match: false,
            actionable_divergence: Some("workspace index emits `use constant` as Subroutine kind"),
        },
        ParityCase {
            label: "Const::Fast wrapper",
            src: "use Const::Fast; const my $MAX => 3;",
            symbol_name: "MAX",
            expect_core_match: false,
            actionable_divergence: Some(
                "workspace index extracts Const::Fast as Variable, not Constant",
            ),
        },
        ParityCase {
            label: "Readonly wrapper",
            src: "use Readonly; Readonly my $NAME => 'x';",
            symbol_name: "NAME",
            expect_core_match: false,
            actionable_divergence: Some(
                "workspace index extracts Readonly as Variable, not Constant",
            ),
        },
        ParityCase {
            label: "class declaration",
            src: "class Worker { }",
            symbol_name: "Worker",
            expect_core_match: false,
            actionable_divergence: Some(
                "class container/qualification differs: surface seeds `main`, workspace stores top-level class",
            ),
        },
    ];

    for case in cases {
        let surface = surface_views(case.src)?;
        let workspace = workspace_views(case.src)?;

        // compare one representative declaration per case: first non-main symbol
        let lhs = find(&surface, case.symbol_name)
            .ok_or_else(|| anyhow!("{}: missing surface {}", case.label, case.symbol_name))?;
        let rhs =
            if case.actionable_divergence.is_some() {
                find(&workspace, case.symbol_name)
            } else {
                Some(find(&workspace, case.symbol_name).ok_or_else(|| {
                    anyhow!("{}: missing workspace {}", case.label, case.symbol_name)
                })?)
            };

        if rhs.is_none() {
            assert!(
                case.actionable_divergence.is_some(),
                "{}: missing workspace {} without divergence annotation",
                case.label,
                case.symbol_name
            );
            continue;
        }
        let rhs = match rhs {
            Some(r) => r,
            None => continue,
        };

        let core_fields_match = lhs.name == rhs.name
            && lhs.qualified_name == rhs.qualified_name
            && lhs.container == rhs.container
            && lhs.kind == rhs.kind;

        assert_eq!(
            core_fields_match, case.expect_core_match,
            "{}: core parity mismatch. surface={lhs:?} workspace={rhs:?}",
            case.label
        );

        if let Some(note) = case.actionable_divergence {
            assert!(
                !core_fields_match || lhs.declarator != rhs.declarator,
                "{}: expected divergence ({note}) but declarations looked equivalent",
                case.label
            );
        }
    }

    Ok(())
}

#[test]
fn surface_workspace_divergence_details_are_actionable() -> Result<()> {
    let surface = surface_views("our $VERSION = '1.0';")?;
    let workspace = workspace_views("our $VERSION = '1.0';")?;

    let s = find(&surface, "VERSION").ok_or_else(|| anyhow!("surface VERSION missing"))?;
    let w = find(&workspace, "VERSION").ok_or_else(|| anyhow!("workspace VERSION missing"))?;
    assert_eq!(s.declarator.as_deref(), Some("our"));
    assert_eq!(w.declarator, None);

    let surface_const = surface_views("package C; use constant PI => 3.14;")?;
    let workspace_const = workspace_views("package C; use constant PI => 3.14;")?;
    let s_const = find(&surface_const, "PI").ok_or_else(|| anyhow!("surface PI missing"))?;
    let w_const = find(&workspace_const, "PI").ok_or_else(|| anyhow!("workspace PI missing"))?;

    assert_eq!(s_const.kind, SymbolKind::Constant);
    assert_eq!(w_const.kind, SymbolKind::Subroutine);

    let const_fast_surface = surface_views("use Const::Fast; const my $MAX => 3;")?;
    let const_fast_workspace = workspace_views("use Const::Fast; const my $MAX => 3;")?;
    let s_fast = find(&const_fast_surface, "MAX").ok_or_else(|| anyhow!("surface MAX missing"))?;
    let w_fast =
        find(&const_fast_workspace, "MAX").ok_or_else(|| anyhow!("workspace MAX missing"))?;
    assert_eq!(s_fast.kind, SymbolKind::Constant);
    assert_eq!(w_fast.kind, SymbolKind::Variable(perl_symbol::VarKind::Scalar));

    let readonly_surface = surface_views("use Readonly; Readonly my $NAME => 'x';")?;
    let readonly_workspace = workspace_views("use Readonly; Readonly my $NAME => 'x';")?;
    let s_readonly =
        find(&readonly_surface, "NAME").ok_or_else(|| anyhow!("surface NAME missing"))?;
    let w_readonly =
        find(&readonly_workspace, "NAME").ok_or_else(|| anyhow!("workspace NAME missing"))?;
    assert_eq!(s_readonly.kind, SymbolKind::Constant);
    assert_eq!(w_readonly.kind, SymbolKind::Variable(perl_symbol::VarKind::Scalar));

    Ok(())
}
