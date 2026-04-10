//! Tests for native Perl 5.38 class `:isa(Parent)` inheritance syntax.
//! Issue #3540: Add semantic support for native Perl 5.38 class inheritance.

mod cpan_test_helpers;
use cpan_test_helpers::*;
use perl_parser_core::{NodeKind, Parser};
use perl_tdd_support::must;

/// Helper: parse source and find the first Class node, returning its parents.
fn class_parents(source: &str) -> Vec<String> {
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    find_class_parents(&ast)
}

fn find_class_parents(node: &perl_parser_core::Node) -> Vec<String> {
    match &node.kind {
        NodeKind::Class { parents, .. } => parents.clone(),
        _ => {
            for child in node.children() {
                let found = find_class_parents(child);
                if !found.is_empty() {
                    return found;
                }
            }
            vec![]
        }
    }
}

// ── Parser tests: clean parse ─────────────────────────────────────────────────

#[test]
fn class_without_isa_parses_cleanly() {
    assert_clean_parse(
        r#"
class Point {
    field $x :param = 0;
    field $y :param = 0;
}
"#,
    );
}

#[test]
fn class_with_single_isa_parses_cleanly() {
    assert_clean_parse(
        r#"
class Point3D :isa(Point) {
    field $z :param = 0;
}
"#,
    );
}

#[test]
fn class_with_multiple_isa_parses_cleanly() {
    assert_clean_parse(
        r#"
class Shape3D :isa(Shape) :isa(Printable) {
    field $z :param = 0;
}
"#,
    );
}

#[test]
fn class_with_qualified_isa_parses_cleanly() {
    assert_clean_parse(
        r#"
class MyApp::Point3D :isa(MyApp::Point) {
    field $z :param = 0;
}
"#,
    );
}

// ── Parser tests: isa parent extraction ──────────────────────────────────────

#[test]
fn class_without_isa_has_no_parents() {
    let parents = class_parents(r#"class Point { }"#);
    assert!(parents.is_empty(), "expected no parents, got {:?}", parents);
}

#[test]
fn class_with_isa_has_correct_parent() {
    let parents = class_parents(r#"class Point3D :isa(Point) { }"#);
    assert_eq!(parents, vec!["Point"], "expected parent 'Point', got {:?}", parents);
}

#[test]
fn class_with_multiple_isa_has_all_parents() {
    let parents = class_parents(r#"class Shape3D :isa(Shape) :isa(Printable) { }"#);
    assert!(parents.contains(&"Shape".to_string()), "expected 'Shape' in {:?}", parents);
    assert!(parents.contains(&"Printable".to_string()), "expected 'Printable' in {:?}", parents);
}

#[test]
fn class_with_qualified_isa_has_qualified_parent() {
    let parents = class_parents(r#"class MyApp::Point3D :isa(MyApp::Point) { }"#);
    assert_eq!(parents, vec!["MyApp::Point"], "expected qualified parent, got {:?}", parents);
}
