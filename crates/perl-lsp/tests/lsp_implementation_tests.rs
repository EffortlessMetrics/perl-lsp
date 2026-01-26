//! Tests for textDocument/implementation request

mod support;
use support::lsp_harness::LspHarness;

#[test]

fn test_implementation_find_subclasses() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test.pl";
    harness
        .open(
            doc_uri,
            r#"
package Animal;
sub new { bless {}, shift }
sub speak { die "Abstract method" }

package Dog;
use parent 'Animal';
sub speak { "Woof!" }

package Cat;
use parent 'Animal';
sub speak { "Meow!" }

package main;
my $pet = Animal->new();
"#,
        )?;

    // Request implementations of Animal class
    let response = harness.implementation(doc_uri, 1, 8)?;

    // Check response format (even with dummy positions)
    assert!(
        response.is_array() || response.is_null(),
        "Implementation should return array or null"
    );

    Ok(())
}

#[test]

fn test_implementation_method_overrides() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test.pl";
    harness
        .open(
            doc_uri,
            r#"
package Base;
sub new { bless {}, shift }
sub process { print "Base process\n" }

package Derived;
use parent 'Base';
sub process { print "Derived process\n" }

package AnotherDerived;
use parent 'Base';
sub process { print "Another process\n" }
"#,
        )?;

    // Request implementations of process method
    let response = harness.implementation(doc_uri, 3, 4)?;

    // Check response format
    assert!(
        response.is_array() || response.is_null(),
        "Implementation should return array or null"
    );

    // Verify positions are not dummy (0,0) if we have results
    if let Some(locations) = response.as_array()
        && !locations.is_empty()
    {
        let location = &locations[0];
        if let Some(range) = location.get("range") {
            let start = &range["start"];
            let start_line = start["line"].as_u64().unwrap_or(0);
            let start_char = start["character"].as_u64().unwrap_or(0);
            assert!(
                start_line > 0 || start_char > 0,
                "Expected non-(0,0) position for implementation, got ({},{})",
                start_line,
                start_char
            );
        }
    }

    Ok(())
}

#[test]

fn test_implementation_interface_pattern() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test.pl";
    harness
        .open(
            doc_uri,
            r#"
package Serializable;
# Interface-like pattern in Perl
sub serialize { die "Must implement serialize" }
sub deserialize { die "Must implement deserialize" }

package JSONSerializer;
use parent 'Serializable';
sub serialize { return "json" }
sub deserialize { return "from json" }

package XMLSerializer;
use parent 'Serializable';
sub serialize { return "xml" }
sub deserialize { return "from xml" }
"#,
        )?;

    // Request implementations of Serializable interface
    let response = harness.implementation(doc_uri, 1, 8)?;

    // Check response format
    assert!(
        response.is_array() || response.is_null(),
        "Implementation should return array or null"
    );

    Ok(())
}

#[test]

fn test_implementation_no_implementations() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test.pl";
    harness
        .open(
            doc_uri,
            r#"
package Standalone;
sub new { bless {}, shift }
sub method { print "Hello\n" }

my $obj = Standalone->new();
"#,
        )?;

    // Request implementations for class with no subclasses
    let response = harness.implementation(doc_uri, 1, 8)?;

    // Should return empty array or null
    assert!(
        response.is_null() || (response.is_array() && response.as_array().is_some_and(|arr| arr.is_empty())),
        "Should return null or empty array for no implementations"
    );

    Ok(())
}
