//! Tests for textDocument/implementation request

mod support;
use support::lsp_harness::LspHarness;

#[test]

fn test_implementation_find_subclasses() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test.pl";
    harness.open(
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
    harness.open(
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
    harness.open(
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
    harness.open(
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
        response.is_null()
            || (response.is_array() && response.as_array().is_some_and(|arr| arr.is_empty())),
        "Should return null or empty array for no implementations"
    );

    Ok(())
}

// --- New tests with real assertions (Fix B: correct enclosing package; Fix C: correct range) ---

/// When cursor is on `sub speak` inside `package Animal`, results must include
/// `Dog::speak` and `Cat::speak` overriders but NOT `main`.
///
/// Tests Fix B: `extract_implementation_target()` uses the actual enclosing package
/// (from `current_package_at`) instead of the hardcoded `"main"` package name.
/// Implementations are now searched for `Animal::speak` finding Dog and Cat overriders.
#[test]
fn test_implementation_correct_package_not_main() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_impl.pl";
    // Line 0: ""
    // Line 1: "package Animal;"
    // Line 2: "sub new { bless {}, shift }"
    // Line 3: "sub speak { die 'Abstract' }"
    // Line 4: ""
    // Line 5: "package Dog;"
    // Line 6: "use parent 'Animal';"
    // Line 7: "sub speak { 'Woof' }"
    // Line 8: ""
    // Line 9: "package Cat;"
    // Line 10: "use parent 'Animal';"
    // Line 11: "sub speak { 'Meow' }"
    harness.open(
        doc_uri,
        "\npackage Animal;\nsub new { bless {}, shift }\nsub speak { die 'Abstract' }\n\npackage Dog;\nuse parent 'Animal';\nsub speak { 'Woof' }\n\npackage Cat;\nuse parent 'Animal';\nsub speak { 'Meow' }\n",
    )?;

    // Position (3, 4) is on "speak" in "sub speak { die 'Abstract' }" inside Animal package
    // (line 3, character 4 = 's' of "speak")
    let response = harness.implementation(doc_uri, 3, 4)?;

    // Must return non-empty results — Dog and Cat both override speak
    let locations = response.as_array().ok_or("Expected array from implementation, got null")?;
    assert!(
        !locations.is_empty(),
        "Expected implementations of Animal::speak (Dog::speak, Cat::speak), got empty array"
    );

    // Verify none of the results point to a 'main' package context.
    // All results should be in this same document (single-file scenario).
    for loc in locations {
        let target_uri = loc["targetUri"].as_str().ok_or("Missing targetUri")?;
        assert_eq!(
            target_uri, doc_uri,
            "Implementation result should point to {doc_uri}, got {target_uri}"
        );
        // Result line should be line 7 (Dog::speak) or line 11 (Cat::speak), NOT line 3 (Animal::speak itself)
        let target_line =
            loc["targetRange"]["start"]["line"].as_u64().ok_or("Missing target line")?;
        assert!(
            target_line != 3,
            "Implementation should find overriders, not the base method at line 3"
        );
    }

    Ok(())
}

/// When `find_inheriting_packages_recursive` finds a `use parent 'Animal'` statement,
/// the result range must point to the enclosing `package Dog;` line, NOT to the
/// `use parent` line itself.
///
/// Tests Fix C: `target_range` in `find_inheriting_packages_recursive` uses the tracked
/// enclosing package node's range instead of the `use parent` statement's range.
#[test]
fn test_implementation_location_points_to_package_not_use_parent()
-> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_range.pl";
    // Line 0: ""
    // Line 1: "package Base;"
    // Line 2: "sub new { bless {}, shift }"
    // Line 3: ""
    // Line 4: "package Derived;"        <-- result should point HERE
    // Line 5: "use parent 'Base';"      <-- result CURRENTLY points here (wrong)
    // Line 6: "sub extra { }"
    harness.open(
        doc_uri,
        "\npackage Base;\nsub new { bless {}, shift }\n\npackage Derived;\nuse parent 'Base';\nsub extra { }\n",
    )?;

    // Cursor on "package Base;" — find all packages that inherit from Base
    // Position (1, 8) is on "Base" in "package Base;"
    let response = harness.implementation(doc_uri, 1, 8)?;

    let locations = response.as_array().ok_or("Expected array from implementation, got null")?;
    assert!(
        !locations.is_empty(),
        "Expected Derived to show up as implementation of Base, got empty"
    );

    // The result should point to "package Derived;" at line 4
    // NOT to "use parent 'Base';" at line 5
    let target_line = locations[0]["targetRange"]["start"]["line"]
        .as_u64()
        .ok_or("Missing targetRange.start.line")?;
    assert_eq!(
        target_line, 4,
        "Implementation result should point to 'package Derived;' at line 4, got line {target_line}"
    );

    Ok(())
}

/// With a base class and three subclasses all overriding the same method,
/// the implementation results must contain exactly 3 entries.
///
/// This validates both the counting logic and that the method lookup across
/// subclasses is exhaustive (Fix B+C).
#[test]
fn test_implementation_multiple_overriders_all_returned() -> Result<(), Box<dyn std::error::Error>>
{
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_multi.pl";
    // Line 0: ""
    // Line 1: "package Shape;"
    // Line 2: "sub new { bless {}, shift }"
    // Line 3: "sub area { 0 }"
    // Line 4: ""
    // Line 5: "package Circle;"
    // Line 6: "use parent 'Shape';"
    // Line 7: "sub area { 3.14 }"
    // Line 8: ""
    // Line 9: "package Square;"
    // Line 10: "use parent 'Shape';"
    // Line 11: "sub area { 4 }"
    // Line 12: ""
    // Line 13: "package Triangle;"
    // Line 14: "use parent 'Shape';"
    // Line 15: "sub area { 6 }"
    harness.open(
        doc_uri,
        "\npackage Shape;\nsub new { bless {}, shift }\nsub area { 0 }\n\npackage Circle;\nuse parent 'Shape';\nsub area { 3.14 }\n\npackage Square;\nuse parent 'Shape';\nsub area { 4 }\n\npackage Triangle;\nuse parent 'Shape';\nsub area { 6 }\n",
    )?;

    // Cursor on "sub area { 0 }" in Shape package — line 3, character 4
    let response = harness.implementation(doc_uri, 3, 4)?;

    let locations = response.as_array().ok_or("Expected array from implementation, got null")?;

    assert_eq!(
        locations.len(),
        3,
        "Expected exactly 3 implementations of Shape::area (Circle, Square, Triangle), got {}",
        locations.len()
    );

    Ok(())
}

/// Package-block method implementations are discoverable from a parent class
/// lookup path using linear inheritance declarations.
#[test]
fn test_implementation_finds_block_package_methods() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_block_pkg.pl";
    harness.open(
        doc_uri,
        "\npackage Base;\nsub speak { 'Base' }\n\npackage Derived;\nuse parent 'Base';\npackage Derived {\n    sub speak { 'Block' }\n}\n",
    )?;

    // Cursor on `Base::speak`
    let response = harness.implementation(doc_uri, 2, 4)?;
    let locations = response.as_array().ok_or("Expected array from implementation, got null")?;
    assert!(
        !locations.is_empty(),
        "Expected at least one implementation for Base::speak in package block form",
    );

    let mut saw_block_method = false;
    for loc in locations {
        let target_uri = loc["targetUri"].as_str().ok_or("Missing targetUri")?;
        assert_eq!(target_uri, doc_uri);
        let target_line = loc["targetRange"]["start"]["line"].as_u64().ok_or("Missing line")?;
        if target_line == 7 {
            saw_block_method = true;
        }
    }

    assert!(
        saw_block_method,
        "Expected implementation location to include package-block method at line 7"
    );

    Ok(())
}

#[test]
fn test_implementation_finds_qualified_override_definition()
-> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///qualified_impl.pl";
    harness.open(
        doc_uri,
        "
package Base;
sub process { 'base' }

package Derived;
use parent 'Base';
sub Derived::process { 'derived' }
",
    )?;

    let response = harness.implementation(doc_uri, 2, 4)?;
    let locations = response.as_array().ok_or("Expected array from implementation, got null")?;
    assert!(
        locations.iter().any(|loc| loc["targetRange"]["start"]["line"].as_u64() == Some(6)),
        "expected implementation results to include qualified override at line 6, got {locations:?}"
    );

    Ok(())
}
