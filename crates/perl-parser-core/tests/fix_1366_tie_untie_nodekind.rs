//! Issue #1366: `tie`/`untie` must produce explicit `NodeKind::Tie` / `Untie`
//! nodes rather than falling back to a generic `FunctionCall`.
//!
//! The corpus-wide NodeKind coverage gate already proves these kinds appear,
//! but these focused regression tests pin the intent: each surface form of
//! `tie`/`untie` resolves to its dedicated AST node.

mod cpan_test_helpers;
use cpan_test_helpers::*;

use perl_parser_core::Node;

/// Collect every `kind_name()` present in the AST.
fn collect_kinds(node: &Node, out: &mut Vec<&'static str>) {
    out.push(node.kind.kind_name());
    for child in node.children() {
        collect_kinds(child, out);
    }
}

fn kinds(source: &str) -> Vec<&'static str> {
    let ast = parse(source);
    let mut out = Vec::new();
    collect_kinds(&ast, &mut out);
    out
}

#[test]
fn test_tie_hash_produces_tie_nodekind() {
    let source = r#"tie %hash, 'Tie::StdHash';"#;
    assert_clean_parse(source);
    let ks = kinds(source);
    assert!(ks.contains(&"Tie"), "expected explicit Tie NodeKind, got: {ks:?}");
}

#[test]
fn test_tie_with_constructor_args_produces_tie_nodekind() {
    let source = r#"tie %hash, 'MyTie::Class', $file, @args;"#;
    assert_clean_parse(source);
    let ks = kinds(source);
    assert!(ks.contains(&"Tie"), "expected explicit Tie NodeKind, got: {ks:?}");
}

#[test]
fn test_tie_scalar_and_array_produce_tie_nodekind() {
    for source in [r#"tie $scalar, 'Tie::Scalar';"#, r#"tie @array, 'Tie::Array';"#] {
        assert_clean_parse(source);
        let ks = kinds(source);
        assert!(ks.contains(&"Tie"), "expected explicit Tie NodeKind for `{source}`, got: {ks:?}");
    }
}

#[test]
fn test_untie_produces_untie_nodekind() {
    let source = r#"untie %hash;"#;
    assert_clean_parse(source);
    let ks = kinds(source);
    assert!(ks.contains(&"Untie"), "expected explicit Untie NodeKind, got: {ks:?}");
}
