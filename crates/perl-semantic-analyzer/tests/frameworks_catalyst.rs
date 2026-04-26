//! Catalyst controller/action semantic extraction tests.

use perl_semantic_analyzer::{
    Parser,
    symbol::{Symbol, SymbolExtractor, SymbolKind, SymbolTable},
};
use perl_tdd_support::{must, must_some};

fn extract_symbols(code: &str) -> SymbolTable {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    SymbolExtractor::new_with_source(code).extract(&ast)
}

fn symbol<'a>(table: &'a SymbolTable, name: &str, kind: SymbolKind) -> Option<&'a Symbol> {
    table.symbols.get(name).and_then(|symbols| symbols.iter().find(|symbol| symbol.kind == kind))
}

#[test]
fn catalyst_controller_package_marks_action_metadata() {
    let code = r#"
package MyApp::Controller::Root;

sub index :Path :Args(0) { }
sub ping :Local { }
"#;

    let table = extract_symbols(code);

    let index = must_some(symbol(&table, "index", SymbolKind::Subroutine));
    assert!(
        index.attributes.iter().any(|attr| attr == "framework=Catalyst"),
        "expected Catalyst framework metadata on `index`"
    );
    assert!(
        index.attributes.iter().any(|attr| attr == "catalyst_controller=true"),
        "expected controller marker on `index`"
    );
    assert!(
        index.attributes.iter().any(|attr| attr == "catalyst_action=true"),
        "expected action marker on `index`"
    );
    assert!(
        index.attributes.iter().any(|attr| attr == "catalyst_action_kind=Path"),
        "expected `Path` action kind on `index`"
    );
    assert!(
        index.attributes.iter().any(|attr| attr.starts_with("catalyst_action_attributes=")),
        "expected synthesized action attribute summary on `index`"
    );
    let index_doc = must_some(index.documentation.as_deref());
    assert!(
        index_doc.contains("Catalyst action") && index_doc.contains("Args(0)"),
        "expected Catalyst action documentation on `index`, got: {index_doc}"
    );

    let ping = must_some(symbol(&table, "ping", SymbolKind::Subroutine));
    assert!(
        ping.attributes.iter().any(|attr| attr == "catalyst_action_kind=Local"),
        "expected `Local` action kind on `ping`"
    );
    let ping_doc = must_some(ping.documentation.as_deref());
    assert!(
        ping_doc.contains("Catalyst action") && ping_doc.contains("Local"),
        "expected Catalyst action documentation on `ping`, got: {ping_doc}"
    );
}

#[test]
fn catalyst_controller_via_parent_marks_global_action() {
    let code = r#"
package MyApp::Admin;
use parent 'Catalyst::Controller';

sub dashboard :Global { }
"#;

    let table = extract_symbols(code);
    let dashboard = must_some(symbol(&table, "dashboard", SymbolKind::Subroutine));
    assert!(
        dashboard.attributes.iter().any(|attr| attr == "framework=Catalyst"),
        "expected Catalyst framework metadata on `dashboard`"
    );
    assert!(
        dashboard.attributes.iter().any(|attr| attr == "catalyst_controller=true"),
        "expected controller marker on `dashboard`"
    );
    assert!(
        dashboard.attributes.iter().any(|attr| attr == "catalyst_action_kind=Global"),
        "expected `Global` action kind on `dashboard`"
    );
    let dashboard_doc = must_some(dashboard.documentation.as_deref());
    assert!(
        dashboard_doc.contains("Catalyst action") && dashboard_doc.contains("Global"),
        "expected Catalyst action documentation on `dashboard`, got: {dashboard_doc}"
    );
}

#[test]
fn non_controller_package_is_not_tagged_as_catalyst_action() {
    let code = r#"
package MyApp::Utility;

sub helper :Path :Args(0) { }
"#;

    let table = extract_symbols(code);
    let helper = must_some(symbol(&table, "helper", SymbolKind::Subroutine));
    assert!(
        !helper.attributes.iter().any(|attr| attr == "framework=Catalyst"),
        "did not expect Catalyst metadata in a non-controller package"
    );
    assert!(
        !helper.attributes.iter().any(|attr| attr == "catalyst_action=true"),
        "did not expect Catalyst action marker in a non-controller package"
    );
    assert!(
        helper.documentation.as_deref().is_none_or(|doc| !doc.contains("Catalyst action")),
        "did not expect Catalyst action documentation in a non-controller package"
    );
}
