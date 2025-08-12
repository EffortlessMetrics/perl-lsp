use serde_json::json;

mod support;
use support::lsp_client::LspClient;

#[test]
fn prepare_and_subtypes() {
    let bin = env!("CARGO_BIN_EXE_perl-lsp");
    let mut client = LspClient::spawn(bin);
    let uri = "file:///isa.pl";
    let source = "package Base; package Child; use parent 'Base'; package GrandChild; use parent 'Child'; 1;\n";

    client.did_open(uri, "perl", source);

    // Prepare type hierarchy at "Base"
    let base_col = source.find("Base").unwrap();
    let prep_response = client.request(
        "textDocument/prepareTypeHierarchy",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": 0, "character": base_col}
        }),
    );

    let items = prep_response["result"]
        .as_array()
        .expect("prepareTypeHierarchy should return an array");

    assert!(!items.is_empty(), "Should prepare type hierarchy item");
    let base_item = &items[0];
    assert_eq!(base_item["name"], "Base", "Should find Base class");

    // Get subtypes of Base
    let subtypes_response = client.request(
        "typeHierarchy/subtypes",
        json!({
            "item": base_item
        }),
    );

    let subtypes = subtypes_response["result"]
        .as_array()
        .expect("subtypes should return an array");

    assert_eq!(subtypes.len(), 1, "Base should have one direct subtype");
    assert_eq!(subtypes[0]["name"], "Child", "Subtype should be Child");

    // Get supertypes of Child
    let child_col = source.find("Child").unwrap();
    let child_prep = client.request(
        "textDocument/prepareTypeHierarchy",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": 0, "character": child_col}
        }),
    );

    let child_items = child_prep["result"]
        .as_array()
        .expect("prepareTypeHierarchy should return an array");
    let child_item = &child_items[0];

    let supertypes_response = client.request(
        "typeHierarchy/supertypes",
        json!({
            "item": child_item
        }),
    );

    let supertypes = supertypes_response["result"]
        .as_array()
        .expect("supertypes should return an array");

    assert_eq!(
        supertypes.len(),
        1,
        "Child should have one direct supertype"
    );
    assert_eq!(supertypes[0]["name"], "Base", "Supertype should be Base");

    client.shutdown();
}

#[test]
fn multiple_inheritance() {
    let bin = env!("CARGO_BIN_EXE_perl-lsp");
    let mut client = LspClient::spawn(bin);
    let uri = "file:///multi.pl";
    let source = r#"
package Mixin1;
package Mixin2;
package Combined;
use parent qw(Mixin1 Mixin2);
1;
"#;

    client.did_open(uri, "perl", source);

    // Find position of "Combined"
    let col = source.find("Combined").unwrap();
    let line = source[..col].matches('\n').count();
    let char_pos = col - source[..col].rfind('\n').map(|p| p + 1).unwrap_or(0);

    let prep_response = client.request(
        "textDocument/prepareTypeHierarchy",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": char_pos}
        }),
    );

    let items = prep_response["result"]
        .as_array()
        .expect("prepareTypeHierarchy should return an array");

    assert!(!items.is_empty(), "Should prepare type hierarchy item");
    let combined_item = &items[0];

    // Get supertypes - should have both Mixin1 and Mixin2
    let supertypes_response = client.request(
        "typeHierarchy/supertypes",
        json!({
            "item": combined_item
        }),
    );

    let supertypes = supertypes_response["result"]
        .as_array()
        .expect("supertypes should return an array");

    assert_eq!(supertypes.len(), 2, "Combined should have two supertypes");

    let names: Vec<String> = supertypes
        .iter()
        .map(|item| item["name"].as_str().unwrap().to_string())
        .collect();

    assert!(
        names.contains(&"Mixin1".to_string()),
        "Should have Mixin1 as parent"
    );
    assert!(
        names.contains(&"Mixin2".to_string()),
        "Should have Mixin2 as parent"
    );

    client.shutdown();
}

#[test]
fn isa_array_inheritance() {
    let bin = env!("CARGO_BIN_EXE_perl-lsp");
    let mut client = LspClient::spawn(bin);
    let uri = "file:///isa.pl";
    let source = r#"
package Parent1;
package Parent2;
package Child;
our @ISA = ('Parent1', 'Parent2');
1;
"#;

    client.did_open(uri, "perl", source);

    // Find position of "Child"
    let col = source.find("Child").unwrap();
    let line = source[..col].matches('\n').count();
    let char_pos = col - source[..col].rfind('\n').map(|p| p + 1).unwrap_or(0);

    let prep_response = client.request(
        "textDocument/prepareTypeHierarchy",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": char_pos}
        }),
    );

    let items = prep_response["result"]
        .as_array()
        .expect("prepareTypeHierarchy should return an array");

    assert!(!items.is_empty(), "Should prepare type hierarchy item");
    let child_item = &items[0];

    // Get supertypes - should have both Parent1 and Parent2
    let supertypes_response = client.request(
        "typeHierarchy/supertypes",
        json!({
            "item": child_item
        }),
    );

    let supertypes = supertypes_response["result"]
        .as_array()
        .expect("supertypes should return an array");

    assert_eq!(
        supertypes.len(),
        2,
        "Child should have two supertypes via @ISA"
    );

    let names: Vec<String> = supertypes
        .iter()
        .map(|item| item["name"].as_str().unwrap().to_string())
        .collect();

    assert!(
        names.contains(&"Parent1".to_string()),
        "Should have Parent1 in @ISA"
    );
    assert!(
        names.contains(&"Parent2".to_string()),
        "Should have Parent2 in @ISA"
    );

    client.shutdown();
}

#[test]
fn type_hierarchy_ignores_string_literals() {
    let bin = env!("CARGO_BIN_EXE_perl-lsp");
    let mut client = LspClient::spawn(bin);
    let uri = "file:///string.pl";
    let source = r#"
package Base;
sub new { bless {}, shift }
sub test {
    my $msg = 'Base';  # This string literal should not be treated as a class
    print "Using Base class\n";  # Neither should this
}
1;
"#;

    client.did_open(uri, "perl", source);

    // Try to get type hierarchy on the string literal 'Base'
    let string_col = source.find("'Base'").unwrap();
    let line = source[..string_col].matches('\n').count();
    let char_pos = (string_col + 1) - source[..string_col].rfind('\n').map(|p| p + 1).unwrap_or(0); // +1 to be inside the string

    let prep_response = client.request(
        "textDocument/prepareTypeHierarchy",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": char_pos}
        }),
    );

    // Should return empty or null for string literals
    let result = &prep_response["result"];
    if let Some(items) = result.as_array() {
        assert!(
            items.is_empty(),
            "String literals should not have type hierarchy"
        );
    } else {
        assert!(
            result.is_null(),
            "String literals should return null for type hierarchy"
        );
    }

    // Now test that the actual package Base works
    let package_col = source.find("package Base").unwrap() + 8; // Position on "Base"
    let prep_response2 = client.request(
        "textDocument/prepareTypeHierarchy",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": 1, "character": package_col}
        }),
    );

    let items = prep_response2["result"]
        .as_array()
        .expect("Package should have type hierarchy");
    assert!(!items.is_empty(), "Package Base should be found");
    assert_eq!(items[0]["name"], "Base", "Should find the Base package");

    client.shutdown();
}
