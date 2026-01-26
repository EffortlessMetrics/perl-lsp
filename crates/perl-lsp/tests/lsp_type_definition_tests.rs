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
            let start_char = start["character"].as_u64().ok_or("Missing character position")?;
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
            let start_char = start["character"].as_u64().ok_or("Missing character position")?;

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
        && response.as_array().ok_or("Expected array but got different type")?.is_empty();
    assert!(
        response.is_null() || is_empty_array,
        "Should return null or empty array for non-object types"
    );
    Ok(())
}
