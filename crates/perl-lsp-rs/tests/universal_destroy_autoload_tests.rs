//! Tests for DESTROY and AUTOLOAD method recognition
//!
//! Validates that:
//! - DESTROY and AUTOLOAD appear in completion lists with correct documentation
//! - DESTROY does NOT fall back to a fabricated `UNIVERSAL::DESTROY` goto-def
//!   target — per perldoc.perl.org/UNIVERSAL, only `isa`/`can`/`DOES`/`VERSION`
//!   are real subs in `package UNIVERSAL`. DESTROY (GC destructor hook) and
//!   AUTOLOAD (failed-method-lookup hook) are interpreter special-method
//!   hooks (perlobj) with no corresponding `UNIVERSAL::` sub to navigate to.
//! - `isa` (a real UNIVERSAL method) still resolves via the UNIVERSAL::
//!   goto-def fallback, for contrast.

mod support;

use serde_json::{Value, json};
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Helper to extract completion items from response
fn extract_completions(response: &Value) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let items = response
        .get("items")
        .and_then(|v| v.as_array())
        .ok_or("Expected completion items array")?;

    let labels: Vec<String> = items
        .iter()
        .filter_map(|item| item.get("label").and_then(|l| l.as_str()).map(String::from))
        .collect();

    Ok(labels)
}

/// Helper to find a completion item by label
fn find_completion_item<'a>(
    response: &'a Value,
    label: &str,
) -> Result<Option<&'a Value>, Box<dyn std::error::Error>> {
    let items = response
        .get("items")
        .and_then(|v| v.as_array())
        .ok_or("Expected completion items array")?;

    Ok(items.iter().find(|item| {
        item.get("label").and_then(|l| l.as_str()).map(|l| l == label).unwrap_or(false)
    }))
}

// ---------------------------------------------------------------------------
// Test 1: DESTROY appears in completion list with documentation
// ---------------------------------------------------------------------------

#[test]
fn completion_offers_destroy_method() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // Open a test file that triggers method completion
    harness.open(
        "file:///test.pl",
        r#"#!/usr/bin/perl
use strict;
use warnings;

my $obj = {};
$obj->
"#,
    )?;

    harness.barrier();

    // Request completion at the end of "$obj->"
    // Line 5 (0-indexed), character 6 (after "->")
    let response = harness.request(
        "textDocument/completion",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 5, "character": 6 }
        }),
    )?;

    let completions = extract_completions(&response)?;

    // Verify DESTROY is in the completion list
    assert!(
        completions.contains(&"DESTROY".to_string()),
        "DESTROY should appear in completion list, got: {:?}",
        completions
    );

    // Verify AUTOLOAD is in the completion list
    assert!(
        completions.contains(&"AUTOLOAD".to_string()),
        "AUTOLOAD should appear in completion list, got: {:?}",
        completions
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Test 2: DESTROY completion has correct documentation
// ---------------------------------------------------------------------------

#[test]
fn completion_destroy_has_documentation() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    harness.open(
        "file:///test.pl",
        r#"#!/usr/bin/perl
use strict;
use warnings;

my $obj = {};
$obj->
"#,
    )?;

    harness.barrier();

    let response = harness.request(
        "textDocument/completion",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 5, "character": 6 }
        }),
    )?;

    let destroy_item = find_completion_item(&response, "DESTROY")?
        .ok_or("DESTROY not found in completion items")?;

    let doc =
        destroy_item.get("documentation").ok_or("DESTROY completion item missing documentation")?;

    // Verify documentation describes the GC/last-reference-released hook.
    let doc_str = doc
        .get("value")
        .and_then(Value::as_str)
        .ok_or("Documentation should include markdown value")?;

    assert!(
        doc_str.contains("released") || doc_str.contains("garbage collected"),
        "DESTROY documentation should describe the last-reference-released/GC hook, got: {}",
        doc_str
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Test 3: AUTOLOAD completion has correct documentation
// ---------------------------------------------------------------------------

#[test]
fn completion_autoload_has_documentation() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    harness.open(
        "file:///test.pl",
        r#"#!/usr/bin/perl
use strict;
use warnings;

my $obj = {};
$obj->
"#,
    )?;

    harness.barrier();

    let response = harness.request(
        "textDocument/completion",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 5, "character": 6 }
        }),
    )?;

    let autoload_item = find_completion_item(&response, "AUTOLOAD")?
        .ok_or("AUTOLOAD not found in completion items")?;

    let doc = autoload_item
        .get("documentation")
        .ok_or("AUTOLOAD completion item missing documentation")?;

    let doc_str = doc
        .get("value")
        .and_then(Value::as_str)
        .ok_or("Documentation should include markdown value")?;

    assert!(
        doc_str.contains("AUTOLOAD")
            || doc_str.contains("Automatic")
            || doc_str.contains("dispatcher"),
        "AUTOLOAD documentation should mention method dispatch, got: {}",
        doc_str
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Test 4: DESTROY goto-definition does NOT fabricate a UNIVERSAL:: target,
// while a real UNIVERSAL method (isa) still does.
//
// perldoc.perl.org/UNIVERSAL lists exactly four real subs shipped in
// `package UNIVERSAL`: isa, can, DOES, VERSION. DESTROY (GC destructor hook)
// and AUTOLOAD (failed-method-lookup hook) are interpreter special-method
// hooks (perlobj) — there is no `UNIVERSAL::DESTROY` sub to navigate to,
// even when a workspace happens to define one under `package UNIVERSAL`
// (that definition isn't reachable through MyPackage's inheritance chain
// unless MyPackage actually lists UNIVERSAL in @ISA/use parent, which it
// does not here). Goto-definition must not claim otherwise.
// ---------------------------------------------------------------------------

#[test]
fn goto_definition_on_destroy_does_not_resolve_to_universal() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // A workspace file that defines `package UNIVERSAL` with both a real
    // UNIVERSAL method (isa) and a special hook (DESTROY). MyPackage below
    // does not explicitly inherit from UNIVERSAL (it is the implicit,
    // universal ancestor only insofar as the interpreter consults it for
    // real method dispatch — the LSP's inheritance-chain walker does not
    // treat it as an explicit @ISA entry).
    harness.open(
        "file:///lib/UNIVERSAL.pm",
        r#"package UNIVERSAL;
use strict;
use warnings;

sub isa {
    my ($self, $class) = @_;
}

sub DESTROY {
    my ($self) = @_;
    # Cleanup code
}

1;
"#,
    )?;

    harness.open(
        "file:///test.pl",
        r#"#!/usr/bin/perl
use strict;
use warnings;

package MyPackage;

sub method {
    my ($self) = @_;
}

package main;

my $obj = bless {}, 'MyPackage';
$obj->DESTROY;
$obj->isa('MyPackage');
"#,
    )?;

    harness.barrier();

    // "DESTROY" on `$obj->DESTROY;` (line 13, 0-indexed).
    let destroy_response = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 13, "character": 10 }  // On "DESTROY"
        }),
    )?;
    let destroy_locations =
        destroy_response.as_array().ok_or("Expected array result for definition")?;
    assert!(
        destroy_locations.is_empty(),
        "DESTROY must NOT resolve to a fabricated UNIVERSAL:: target \
         (no UNIVERSAL::DESTROY sub is reachable from MyPackage), got: {:?}",
        destroy_locations
    );

    // "isa" on `$obj->isa('MyPackage');` (line 14, 0-indexed) — a real
    // UNIVERSAL method, so the UNIVERSAL:: fallback is expected to fire.
    let isa_response = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 14, "character": 7 }  // On "isa"
        }),
    )?;
    let isa_locations = isa_response.as_array().ok_or("Expected array result for definition")?;
    assert!(
        !isa_locations.is_empty(),
        "isa should resolve via the UNIVERSAL:: fallback (isa is a real UNIVERSAL method)"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Test 5: DESTROY appears in both regular and special contexts
// ---------------------------------------------------------------------------

#[test]
fn destroy_recognized_in_various_contexts() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    harness.open(
        "file:///test.pl",
        r#"#!/usr/bin/perl
use strict;
use warnings;

package MyClass;

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub DESTROY {
    my ($self) = @_;
    print "Destroying object\n";
}

package main;

my $obj = MyClass->new();
$obj->DESTROY;
"#,
    )?;

    harness.barrier();

    // Test completion context
    let response = harness.request(
        "textDocument/completion",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 19, "character": 6 }  // After "$obj->"
        }),
    )?;

    let completions = extract_completions(&response)?;
    assert!(
        completions.contains(&"DESTROY".to_string()),
        "DESTROY should be in completion after $obj->"
    );

    Ok(())
}
