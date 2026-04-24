//! Representative declaration-bank tests used by workspace parity checks.
//!
//! These fixtures intentionally use real parser input (not hand-built AST nodes)
//! so `extract_symbol_decls()` behavior is locked against concrete Perl snippets.

use anyhow::Result;
use perl_parser_core::Parser;
use perl_symbol::SymbolKind;
use perl_symbol::extract_symbol_decls;

struct SurfaceCase {
    label: &'static str,
    src: &'static str,
    expected_names: &'static [&'static str],
}

fn parse(src: &str) -> Result<perl_parser_core::Node> {
    let mut parser = Parser::new(src);
    Ok(parser.parse()?)
}

#[test]
fn surface_bank_covers_representative_workspace_parity_constructs() -> Result<()> {
    let cases = vec![
        SurfaceCase {
            label: "package + sub",
            src: "package My::Pkg; sub run { 1 }",
            expected_names: &["My::Pkg", "run"],
        },
        SurfaceCase {
            label: "method in class",
            src: "class My::Class { method tick () { 1 } }",
            expected_names: &["My::Class", "tick"],
        },
        SurfaceCase { label: "my scalar", src: "my $count = 1;", expected_names: &["count"] },
        SurfaceCase { label: "our hash", src: "our %CONFIG = ();", expected_names: &["CONFIG"] },
        SurfaceCase {
            label: "array list declaration",
            src: "my ($x, @vals, %opts) = (1);",
            expected_names: &["x", "vals", "opts"],
        },
        SurfaceCase {
            label: "use constant",
            src: "package C; use constant PI => 3.14;",
            expected_names: &["C", "PI"],
        },
        SurfaceCase {
            label: "Const::Fast",
            src: "use Const::Fast; const my $MAX => 3;",
            expected_names: &["MAX"],
        },
        SurfaceCase {
            label: "Readonly",
            src: "use Readonly; Readonly my $NAME => 'n';",
            expected_names: &["NAME"],
        },
    ];

    for case in cases {
        let ast = parse(case.src)?;
        let decls = extract_symbol_decls(&ast, Some("main"));
        for expected in case.expected_names {
            assert!(
                decls.iter().any(|decl| decl.name == *expected),
                "case '{}' expected '{}' in {:?}",
                case.label,
                expected,
                decls.iter().map(|decl| (&decl.name, &decl.kind)).collect::<Vec<_>>()
            );
        }
    }

    Ok(())
}

#[test]
fn surface_bank_tracks_constant_kind_for_constant_sources() -> Result<()> {
    let ast = parse(
        "package K; use constant RATE => 1; use Const::Fast; const my $FLAG => 1; use Readonly; Readonly my $NAME => 'x';",
    )?;
    let decls = extract_symbol_decls(&ast, Some("main"));

    for name in ["RATE", "FLAG", "NAME"] {
        let decl = decls
            .iter()
            .find(|decl| decl.name == name)
            .ok_or_else(|| anyhow::anyhow!("missing constant declaration for {name}"))?;
        assert_eq!(decl.kind, SymbolKind::Constant, "{name} should be Constant kind");
    }

    Ok(())
}
