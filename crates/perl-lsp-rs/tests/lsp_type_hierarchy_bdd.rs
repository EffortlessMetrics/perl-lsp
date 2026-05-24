//! BDD-style workflow tests for textDocument/prepareTypeHierarchy +
//! typeHierarchy/subtypes + typeHierarchy/supertypes.
//!
//! The provider has stable assertion-driven e2e coverage in
//! `lsp_type_hierarchy_e2e.rs`, but no narrative Given/When/Then scenario
//! that reads as a user workflow. These tests wrap the two most common
//! interactions - exploring subtypes from a base class and tracing
//! supertypes via @ISA - under the existing BDD scenario logger.

mod support;

use serde_json::{Value, json};
use support::lsp_client::LspClient;
use support::ux_bdd::UxScenario;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn find_line_col(source: &str, needle: &str) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    let byte_offset = source.find(needle).ok_or_else(|| format!("needle {needle:?} not found"))?;
    let line = source[..byte_offset].matches('\n').count() as u32;
    let line_start = source[..byte_offset].rfind('\n').map(|p| p + 1).unwrap_or(0);
    let col = (byte_offset - line_start) as u32;
    Ok((line, col))
}

fn names_of(items: &[Value]) -> Vec<String> {
    items.iter().filter_map(|item| item["name"].as_str().map(String::from)).collect()
}

#[test]
fn bdd_user_explores_subtypes_from_a_base_class() -> TestResult {
    let scenario = UxScenario::new("User explores subtypes from a base class");

    scenario.given("a file with Base, Child, and GrandChild forming a single-inheritance chain");
    let source = "package Base; package Child; use parent 'Base'; package GrandChild; use parent 'Child'; 1;\n";
    let uri = "file:///hierarchy.pl";

    let bin = env!("CARGO_BIN_EXE_perl-lsp");
    let mut client = LspClient::spawn(bin)?;
    client.did_open(uri, "perl", source)?;

    scenario.when("the user requests prepareTypeHierarchy on the Base package declaration");
    let (line, character) = find_line_col(source, "Base")?;
    let prep = client.request(
        "textDocument/prepareTypeHierarchy",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character },
        }),
    )?;

    let items =
        prep["result"].as_array().ok_or("prepareTypeHierarchy did not return an array result")?;
    assert!(!items.is_empty(), "prepareTypeHierarchy should resolve Base to an item");
    let base_item = &items[0];
    assert_eq!(base_item["name"], "Base", "prepared item should name Base");

    scenario.when("the user then requests typeHierarchy/subtypes on that item");
    let subtypes = client.request("typeHierarchy/subtypes", json!({ "item": base_item }))?;
    let subtypes_arr = subtypes["result"].as_array().ok_or("subtypes did not return an array")?;

    scenario.then("Child is reported as a direct subtype");
    let names = names_of(subtypes_arr);
    assert!(
        names.iter().any(|n| n == "Child"),
        "Child should appear as a direct subtype of Base, got: {names:?}",
    );

    client.shutdown()?;
    Ok(())
}

#[test]
fn bdd_user_traces_supertypes_via_at_isa_multiple_inheritance() -> TestResult {
    let scenario = UxScenario::new("User traces supertypes via @ISA multiple inheritance");

    scenario.given("a file with Parent1, Parent2 and a Child whose @ISA names both parents");
    let source = "
package Parent1;
package Parent2;
package Child;
our @ISA = ('Parent1', 'Parent2');
1;
";
    let uri = "file:///isa.pl";

    let bin = env!("CARGO_BIN_EXE_perl-lsp");
    let mut client = LspClient::spawn(bin)?;
    client.did_open(uri, "perl", source)?;

    scenario.when("the user requests prepareTypeHierarchy on the Child package declaration");
    let (line, character) = find_line_col(source, "Child")?;
    let prep = client.request(
        "textDocument/prepareTypeHierarchy",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character },
        }),
    )?;

    let items =
        prep["result"].as_array().ok_or("prepareTypeHierarchy did not return an array result")?;
    assert!(!items.is_empty(), "prepareTypeHierarchy should resolve Child to an item");
    let child_item = &items[0];

    scenario.when("the user then requests typeHierarchy/supertypes on that item");
    let supertypes = client.request("typeHierarchy/supertypes", json!({ "item": child_item }))?;
    let supertypes_arr =
        supertypes["result"].as_array().ok_or("supertypes did not return an array")?;

    scenario.then("both Parent1 and Parent2 appear as direct supertypes");
    let names = names_of(supertypes_arr);
    assert!(
        names.iter().any(|n| n == "Parent1"),
        "Parent1 should appear as a supertype, got: {names:?}",
    );
    assert!(
        names.iter().any(|n| n == "Parent2"),
        "Parent2 should appear as a supertype, got: {names:?}",
    );

    client.shutdown()?;
    Ok(())
}
