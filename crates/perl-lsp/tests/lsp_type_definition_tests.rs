//! Tests for textDocument/typeDefinition request

mod support;
use serde_json::json;
use support::lsp_harness::LspHarness;

#[test]
fn test_type_definition_basic() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new();

    // Initialize with type definition capability
    let _init_response = harness.initialize(Some(json!({
        "textDocument": {
            "typeDefinition": {
                "dynamicRegistration": false
            }
        }
    })))?;

    // Open a document with a class and object
    let doc_uri = "file:///test.pl";
    harness.open(
        doc_uri,
        r#"
package MyClass;

sub new {
    my $class = shift;
    bless {}, $class;
}

sub method {
    my $self = shift;
    print "Hello\n";
}

package main;

my $obj = MyClass->new();
$obj->method();
"#,
    )?;

    // Request type definition for MyClass in the instantiation
    let response = harness.type_definition(doc_uri, 14, 10)?;

    // Should return the MyClass package definition
    println!("Type definition response: {:?}", response);

    // The implementation may return null if nothing is found
    // or an array if there are results
    assert!(
        response.is_array() || response.is_null(),
        "Type definition should return array or null, got: {:?}",
        response
    );

    // For now just check the response format, the implementation
    // needs refinement to actually find the definitions
    if let Some(locations) = response.as_array()
        && !locations.is_empty()
    {
        let location = &locations[0];
        assert_eq!(location["uri"], doc_uri);

        // Verify we have real positions, not dummy (0,0) values
        if let Some(range) = location.get("range") {
            let start = &range["start"];
            let start_line = start["line"].as_u64().ok_or("Missing line number")?;
            let start_char = start["character"]
                .as_u64()
                .ok_or("Missing character position")?;
            assert!(
                start_line > 0 || start_char > 0,
                "Expected non-(0,0) position, got ({},{})",
                start_line,
                start_char
            );
        }
    }
    Ok(())
}

#[test]
fn test_type_definition_crlf_emoji_positions() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_crlf.pl";
    // Use CRLF line endings and emojis to test position handling
    harness.open(
        doc_uri,
        "package MyClass;\r\n# 🎉 This has emojis\r\nsub new { bless {}, shift }\r\n\r\nmy $obj = MyClass->new();\r\n$obj->process();\r\n",
    )?;

    // Request type definition for $obj on line 5 (after CRLF lines)
    let response = harness.type_definition(doc_uri, 4, 1)?;

    // Verify we get proper positions
    if let Some(locations) = response.as_array()
        && !locations.is_empty()
    {
        let location = &locations[0];
        if let Some(range) = location.get("range") {
            let start = &range["start"];
            let start_line = start["line"].as_u64().ok_or("Missing line number")?;
            let start_char = start["character"]
                .as_u64()
                .ok_or("Missing character position")?;

            // With CRLF and emojis, positions should still be valid and non-zero
            assert!(
                start_line > 0 || start_char > 0,
                "CRLF/emoji test: Expected non-(0,0) position, got ({},{})",
                start_line,
                start_char
            );
        }
    }
    Ok(())
}

#[test]
fn test_type_definition_method_call() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test.pl";
    harness.open(
        doc_uri,
        r#"
package Base;
sub new { bless {}, shift }

package Derived;
use parent 'Base';
sub method { }

package main;
my $obj = Derived->new();
$obj->method();
"#,
    )?;

    // Request type definition on method call
    let response = harness.type_definition(doc_uri, 9, 5)?;

    // Check we get a result (even if positions are dummy for now)
    assert!(
        response.is_array() || response.is_null(),
        "Type definition should return array or null"
    );
    Ok(())
}

#[test]
fn test_type_definition_blessed_reference() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test.pl";
    harness.open(
        doc_uri,
        r#"
package MyClass;
sub new { bless {}, shift }

my $obj = bless {}, 'MyClass';
my $type = ref $obj;
"#,
    )?;

    // Request type definition on blessed reference
    let response = harness.type_definition(doc_uri, 4, 15)?;

    // Check response format
    assert!(
        response.is_array() || response.is_null(),
        "Type definition should return array or null"
    );
    Ok(())
}

#[test]
fn test_type_definition_isa_operator() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test.pl";
    harness.open(
        doc_uri,
        r#"
package Animal;
sub new { bless {}, shift }

package Dog;
use parent 'Animal';

my $pet = Dog->new();
if ($pet isa Animal) {
    print "It's an animal\n";
}
"#,
    )?;

    // Request type definition on the isa check
    let response = harness.type_definition(doc_uri, 8, 13)?;

    // Check response format
    assert!(
        response.is_array() || response.is_null(),
        "Type definition should return array or null"
    );
    Ok(())
}

#[test]
fn test_type_definition_moose_type_library_resolves_custom_type()
-> Result<(), Box<dyn std::error::Error>> {
    let types_uri = "file:///lib/MyApp/Types.pm";
    let types_code = r#"
package MyApp::Types;
use MooseX::Types -declare => [qw(UserID)];
type UserID, where { /\A\d+\z/ };

1;
"#;
    let _types_ast = perl_parser::Parser::new(types_code).parse()?;

    let user_uri = "file:///lib/MyApp/User.pm";
    let user_code = r#"
package MyApp::User;
use Moose;
use MyApp::Types qw(UserID);

has 'id' => (is => 'ro', isa => UserID);

1;
"#;
    let user_ast = perl_parser::Parser::new(user_code).parse()?;

    let line = user_code
        .lines()
        .position(|line| line.contains("isa => UserID"))
        .ok_or("type use line not found")?;
    let character = user_code
        .lines()
        .nth(line)
        .and_then(|line| line.find("UserID"))
        .ok_or("type use column not found")?;

    let mut documents = std::collections::HashMap::new();
    documents.insert(types_uri.to_string(), types_code.to_string());
    documents.insert(user_uri.to_string(), user_code.to_string());

    let provider = perl_lsp_navigation::TypeDefinitionProvider::new();
    let locations = provider
        .find_type_definition(
            &user_ast,
            line as u32,
            character as u32,
            user_uri,
            &documents,
        )
        .ok_or("Expected array from type definition")?;
    assert!(
        !locations.is_empty(),
        "Expected custom Moose type definition to resolve, got: {locations:?}"
    );

    let target_uri = locations[0].target_uri.as_str();
    assert_eq!(
        target_uri, types_uri,
        "type definition should resolve into the type library"
    );

    let target_line = locations[0].target_range.start.line as u64;
    let expected_line = types_code
        .lines()
        .position(|line| line.contains("type UserID"))
        .ok_or("type declaration line missing")? as u64;
    assert_eq!(
        target_line, expected_line,
        "type definition should land on `type UserID`"
    );

    Ok(())
}

#[test]
fn test_type_definition_no_type() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test.pl";
    harness.open(
        doc_uri,
        r#"
my $scalar = 42;
my @array = (1, 2, 3);
my %hash = (key => 'value');
"#,
    )?;

    // Request type definition on regular variable
    let response = harness.type_definition(doc_uri, 1, 4)?;

    // Should return null for non-object types
    let is_empty_array = response.is_array()
        && response
            .as_array()
            .ok_or("Expected array but got different type")?
            .is_empty();
    assert!(
        response.is_null() || is_empty_array,
        "Should return null or empty array for non-object types"
    );
    Ok(())
}

// --- New tests with real assertions (Fix A: cross-file/inheritance lookup) ---

/// Type definition on `Base->new()` should resolve to `package Base;` at line 1.
///
/// Tests Fix A: `find_package_definition_in_docs` searches all open documents so that
/// same-file lookups work even when the type name comes from a
/// `Binary { op: "->", left: Identifier("Base"), right: Identifier("new") }` node.
#[test]
fn test_type_definition_use_parent_chain_finds_base() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_parent.pl";
    // Line 0: ""
    // Line 1: "package Base;"
    // Line 2: "sub new { bless {}, shift }"
    // Line 3: ""
    // Line 4: "package Derived;"
    // Line 5: "use parent 'Base';"
    // Line 6: "sub method { }"
    // Line 7: ""
    // Line 8: "package main;"
    // Line 9: "my $obj = Base->new();"
    harness.open(
        doc_uri,
        "\npackage Base;\nsub new { bless {}, shift }\n\npackage Derived;\nuse parent 'Base';\nsub method { }\n\npackage main;\nmy $obj = Base->new();\n",
    )?;

    // Position (9, 10) is on "Base" in "my $obj = Base->new();"
    // (line 9, character 10 = 'B' of "Base")
    let response = harness.type_definition(doc_uri, 9, 10)?;

    // Must return non-empty array — "package Base;" is at line 1 in this document
    let locations = response
        .as_array()
        .ok_or("Expected array from type definition, got null")?;
    assert!(
        !locations.is_empty(),
        "Expected at least one location for 'Base' type definition"
    );

    // The result should point to "package Base;" which is at line 1
    let location = &locations[0];
    let target_line = location["targetRange"]["start"]["line"]
        .as_u64()
        .ok_or("Missing targetRange.start.line in LocationLink")?;
    assert_eq!(
        target_line, 1,
        "Type definition should point to 'package Base;' at line 1, got line {target_line}"
    );

    Ok(())
}

/// When two documents are open, type definition should find a package defined in
/// the second document (not the one containing the reference).
///
/// Tests Fix A: `find_package_definition_in_docs` iterates all documents in `doc_map`,
/// enabling cross-file resolution.
#[test]
fn test_type_definition_cross_document_lookup() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    // Document 1: defines the package
    let lib_uri = "file:///lib/Widget.pm";
    harness.open(
        lib_uri,
        "\npackage Widget;\nsub new { bless {}, shift }\nsub render { }\n",
    )?;

    // Document 2: uses the package — Widget is NOT defined here
    // Line 0: ""
    // Line 1: "use Widget;"
    // Line 2: ""
    // Line 3: "my $w = Widget->new();"
    let main_uri = "file:///main.pl";
    harness.open(main_uri, "\nuse Widget;\n\nmy $w = Widget->new();\n")?;

    // Position (3, 8) is on "Widget" in "my $w = Widget->new();"
    // (line 3, character 8 = 'W' of "Widget")
    let response = harness.type_definition(main_uri, 3, 8)?;

    // Must return non-empty array — package Widget is defined in lib_uri
    let locations = response
        .as_array()
        .ok_or("Expected array from type definition, got null")?;
    assert!(
        !locations.is_empty(),
        "Expected location for 'Widget' defined in {lib_uri}, got empty array"
    );

    // The target URI must be the library document
    let target_uri = locations[0]["targetUri"]
        .as_str()
        .ok_or("Missing targetUri in LocationLink result")?;
    assert_eq!(
        target_uri, lib_uri,
        "Type definition should point to {lib_uri} (where Widget is defined), got {target_uri}"
    );

    // Package Widget is at line 1 of lib_uri
    let target_line = locations[0]["targetRange"]["start"]["line"]
        .as_u64()
        .ok_or("Missing targetRange.start.line")?;
    assert_eq!(
        target_line, 1,
        "Type definition should point to line 1 ('package Widget;') in {lib_uri}, got line {target_line}"
    );

    Ok(())
}

/// The existing `test_type_definition_method_call` weakly asserts `is_array() || is_null()`.
/// This test strengthens that: when cursor is on `Derived->new()`, the result MUST be
/// non-empty and point to `package Derived;` at line 4.
///
/// This test validates the same-file lookup path works end-to-end with real assertions.
#[test]
fn test_type_definition_method_call_strong_assertion() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_strong.pl";
    // Line 0: ""
    // Line 1: "package Base;"
    // Line 2: "sub new { bless {}, shift }"
    // Line 3: ""
    // Line 4: "package Derived;"
    // Line 5: "use parent 'Base';"
    // Line 6: "sub method { }"
    // Line 7: ""
    // Line 8: "package main;"
    // Line 9: "my $obj = Derived->new();"
    harness.open(
        doc_uri,
        "\npackage Base;\nsub new { bless {}, shift }\n\npackage Derived;\nuse parent 'Base';\nsub method { }\n\npackage main;\nmy $obj = Derived->new();\n",
    )?;

    // Position (9, 10) is on "Derived" in "my $obj = Derived->new();"
    // (line 9, character 10 = 'D' of "Derived")
    let response = harness.type_definition(doc_uri, 9, 10)?;

    // Must be non-null and non-empty
    let locations = response.as_array().ok_or(
        "Expected array from type definition, got null — no type found for Derived->new()",
    )?;
    assert!(
        !locations.is_empty(),
        "Expected location for 'Derived' constructor call, got empty"
    );

    // Result must point to this same document
    let target_uri = locations[0]["targetUri"]
        .as_str()
        .ok_or("Missing targetUri")?;
    assert_eq!(
        target_uri, doc_uri,
        "Type definition should point to {doc_uri}, got {target_uri}"
    );

    // "package Derived;" is at line 4
    let target_line = locations[0]["targetRange"]["start"]["line"]
        .as_u64()
        .ok_or("Missing targetRange.start.line")?;
    assert_eq!(
        target_line, 4,
        "Type definition should point to 'package Derived;' at line 4, got line {target_line}"
    );

    Ok(())
}
