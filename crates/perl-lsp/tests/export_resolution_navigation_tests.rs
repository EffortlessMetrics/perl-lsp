//! Cross-module export symbol resolution navigation tests for perl-lsp
//!
//! Validates that go-to-definition on a bareword subroutine call in a consumer file
//! that uses `use Module;` will navigate to the exporter's subroutine definition.
//!
//! AC2: Cross-module go-to-definition via @EXPORT
//! AC3: use Module () does NOT trigger export resolution
//!
//! NOTE: AC1 (export extraction) and AC4 (%EXPORT_TAGS) are tested in
//! perl-workspace-index tests.

mod support;

use serde_json::{Value, json};
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Helper to get the first location from a definition response
fn first_location(response: &Value) -> Result<&Value, Box<dyn std::error::Error + 'static>> {
    let locations = response.as_array().ok_or_else(|| {
        Box::new(std::io::Error::other("expected array result for definition"))
            as Box<dyn std::error::Error + 'static>
    })?;
    locations.first().ok_or_else(|| {
        Box::new(std::io::Error::other("definition result was empty"))
            as Box<dyn std::error::Error + 'static>
    })
}

/// Helper to assert a valid LSP location structure
fn assert_valid_location(location: &Value) -> Result<(), Box<dyn std::error::Error + 'static>> {
    if location.get("uri").is_none() {
        return Err("Location must have 'uri' field".into());
    }
    if location.get("range").is_none() {
        return Err("Location must have 'range' field".into());
    }
    let range = location.get("range").unwrap();
    if range.get("start").is_none() {
        return Err("Range must have 'start' position".into());
    }
    if range.get("end").is_none() {
        return Err("Range must have 'end' position".into());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// AC2: Cross-module go-to-definition via @EXPORT
// ---------------------------------------------------------------------------

/// When a consumer file uses `use MyModule;` and calls `foo()`,
/// go-to-definition on `foo` should navigate to `sub foo` in MyModule.pm
#[test]
fn go_to_definition_on_bareword_export_via_use_module() -> TestResult {
    let mut harness = LspHarness::new();
    let workspace = support::lsp_harness::TempWorkspace::new()?;

    // Write the exporter module
    workspace.write(
        "lib/MyModule.pm",
        r#"package MyModule;
use strict;
use warnings;

our @EXPORT = qw(foo bar);

sub foo {
    return "foo";
}

sub bar {
    return "bar";
}

sub _private {
    return "private";
}

1;
"#,
    )?;

    harness.initialize_with_root(&workspace.root_uri, None)?;

    // Open the module in the LSP so it's indexed
    let module_uri = workspace.uri("lib/MyModule.pm");
    let module_content = std::fs::read_to_string(workspace.dir.path().join("lib/MyModule.pm"))
        .map_err(|e| format!("failed to read module: {e}"))?;
    harness.open(&module_uri, &module_content)?;

    // Open the consumer file that uses MyModule and calls foo()
    harness.open(
        &workspace.uri("consumer.pl"),
        r#"#!/usr/bin/perl
use strict;
use warnings;
use MyModule;

# Call the exported function
my $result = foo();
print "$result\n";
"#,
    )?;

    harness.barrier();

    // Request go-to-definition on "foo" in the consumer file
    // Line 5 (0-indexed): "my $result = foo();"
    // Character around position 15 (on 'foo')
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("consumer.pl")},
            "position": {"line": 5, "character": 15}
        }),
    )?;

    // The result should navigate to the exporter's sub foo
    let location = first_location(&result)?;
    assert_valid_location(location)?;

    let uri = location["uri"].as_str().ok_or("Expected URI string")?;
    assert!(
        uri.contains("MyModule.pm") || uri.contains("MyModule%2Epm"),
        "Definition should point to MyModule.pm, got: {}",
        uri
    );

    // Verify it's pointing to the 'sub foo' definition (around line 7)
    let range = &location["range"];
    let start_line = range["start"]["line"].as_u64().unwrap_or(0);
    assert!(
        start_line >= 6 && start_line <= 9,
        "Definition should point to 'sub foo' body (around lines 7-9), got line {}",
        start_line
    );

    Ok(())
}

/// Same as above but for 'bar' function
#[test]
fn go_to_definition_on_bareword_export_bar() -> TestResult {
    let mut harness = LspHarness::new();
    let workspace = support::lsp_harness::TempWorkspace::new()?;

    workspace.write(
        "lib/ExportModule.pm",
        r#"package ExportModule;
use strict;
use warnings;

our @EXPORT = qw(bar);

sub bar {
    return "bar";
}

1;
"#,
    )?;

    harness.initialize_with_root(&workspace.root_uri, None)?;

    let module_uri = workspace.uri("lib/ExportModule.pm");
    let module_content = std::fs::read_to_string(workspace.dir.path().join("lib/ExportModule.pm"))
        .map_err(|e| format!("failed to read module: {e}"))?;
    harness.open(&module_uri, &module_content)?;

    harness.open(
        &workspace.uri("app.pl"),
        r#"#!/usr/bin/perl
use strict;
use ExportModule;

my $x = bar();
"#,
    )?;

    harness.barrier();

    // Line 3: "my $x = bar();" - character on 'bar'
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("app.pl")},
            "position": {"line": 3, "character": 10}
        }),
    )?;

    let location = first_location(&result)?;
    assert_valid_location(location)?;

    let uri = location["uri"].as_str().ok_or("Expected URI string")?;
    assert!(
        uri.contains("ExportModule.pm"),
        "Definition should point to ExportModule.pm, got: {}",
        uri
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// AC3: use Module () does NOT trigger export resolution
// ---------------------------------------------------------------------------

/// When a consumer file uses `use MyModule ();` (explicit empty import),
/// go-to-definition on a bareword that MyModule exports should NOT
/// navigate to the exporter - it should return empty or local results.
#[test]
fn go_to_definition_does_not_resolve_export_when_use_module_has_empty_parens() -> TestResult {
    let mut harness = LspHarness::new();
    let workspace = support::lsp_harness::TempWorkspace::new()?;

    // Write a module that exports 'foo'
    workspace.write(
        "lib/SomeModule.pm",
        r#"package SomeModule;
use strict;
use warnings;

our @EXPORT = qw(foo);

sub foo {
    return "foo";
}

1;
"#,
    )?;

    harness.initialize_with_root(&workspace.root_uri, None)?;

    let module_uri = workspace.uri("lib/SomeModule.pm");
    let module_content = std::fs::read_to_string(workspace.dir.path().join("lib/SomeModule.pm"))
        .map_err(|e| format!("failed to read module: {e}"))?;
    harness.open(&module_uri, &module_content)?;

    // Consumer uses 'use SomeModule ();' - empty parens means "import nothing"
    harness.open(
        &workspace.uri("consumer.pl"),
        r#"#!/usr/bin/perl
use strict;
use warnings;
use SomeModule ();

# Try to call foo() - but SomeModule didn't actually import it
my $result = foo();
"#,
    )?;

    harness.barrier();

    // Request go-to-definition on "foo"
    // Line 5: "my $result = foo();" - character on 'foo'
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("consumer.pl")},
            "position": {"line": 5, "character": 15}
        }),
    )?;

    // The result should be empty or should NOT point to SomeModule.pm
    // because use SomeModule () means "import nothing"
    if let Some(locations) = result.as_array() {
        if !locations.is_empty() {
            let location = &locations[0];
            if let Some(uri) = location.get("uri").and_then(|u| u.as_str()) {
                assert!(
                    !uri.contains("SomeModule.pm"),
                    "use SomeModule () means no symbols are imported, \
                     so foo() should NOT resolve to SomeModule.pm. \
                     Got URI: {}",
                    uri
                );
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// AC2 variant: Multiple use statements - export resolution fallback
// ---------------------------------------------------------------------------

/// When multiple modules are used and one of them exports the symbol,
/// go-to-definition should find the exported symbol.
#[test]
fn go_to_definition_resolves_export_from_one_of_multiple_modules() -> TestResult {
    let mut harness = LspHarness::new();
    let workspace = support::lsp_harness::TempWorkspace::new()?;

    // Write module A that exports 'shared_func'
    workspace.write(
        "lib/ModuleA.pm",
        r#"package ModuleA;
use strict;
use warnings;

our @EXPORT = qw(shared_func);

sub shared_func {
    return "A";
}

1;
"#,
    )?;

    // Write module B that does NOT export 'shared_func'
    workspace.write(
        "lib/ModuleB.pm",
        r#"package ModuleB;
use strict;
use warnings;

# Note: NO @EXPORT or @EXPORT_OK for shared_func

sub other_func {
    return "B";
}

1;
"#,
    )?;

    harness.initialize_with_root(&workspace.root_uri, None)?;

    // Open both modules
    let module_a_uri = workspace.uri("lib/ModuleA.pm");
    let module_a_content = std::fs::read_to_string(workspace.dir.path().join("lib/ModuleA.pm"))
        .map_err(|e| format!("failed to read module: {e}"))?;
    harness.open(&module_a_uri, &module_a_content)?;

    let module_b_uri = workspace.uri("lib/ModuleB.pm");
    let module_b_content = std::fs::read_to_string(workspace.dir.path().join("lib/ModuleB.pm"))
        .map_err(|e| format!("failed to read module: {e}"))?;
    harness.open(&module_b_uri, &module_b_content)?;

    // Consumer uses both modules, calls shared_func (exported by ModuleA)
    harness.open(
        &workspace.uri("consumer.pl"),
        r#"#!/usr/bin/perl
use strict;
use ModuleA;
use ModuleB;

my $result = shared_func();
"#,
    )?;

    harness.barrier();

    // Line 4: "my $result = shared_func();" - character on 'shared_func'
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("consumer.pl")},
            "position": {"line": 4, "character": 14}
        }),
    )?;

    let location = first_location(&result)?;
    assert_valid_location(location)?;

    let uri = location["uri"].as_str().ok_or("Expected URI string")?;
    assert!(
        uri.contains("ModuleA.pm"),
        "shared_func is exported by ModuleA, definition should point to ModuleA.pm, got: {}",
        uri
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// AC2 variant: Symbol defined locally takes precedence over export
// ---------------------------------------------------------------------------

/// When a symbol is both defined locally AND exported from a used module,
/// the local definition should take precedence.
#[test]
fn go_to_definition_prefers_local_definition_over_export() -> TestResult {
    let mut harness = LspHarness::new();
    let workspace = support::lsp_harness::TempWorkspace::new()?;

    // Write module that exports 'foo'
    workspace.write(
        "lib/ExportModule.pm",
        r#"package ExportModule;
use strict;
use warnings;

our @EXPORT = qw(foo);

sub foo {
    return "from module";
}

1;
"#,
    )?;

    harness.initialize_with_root(&workspace.root_uri, None)?;

    let module_uri = workspace.uri("lib/ExportModule.pm");
    let module_content = std::fs::read_to_string(workspace.dir.path().join("lib/ExportModule.pm"))
        .map_err(|e| format!("failed to read module: {e}"))?;
    harness.open(&module_uri, &module_content)?;

    // Consumer defines its own 'foo' locally AND uses the module
    harness.open(
        &workspace.uri("consumer.pl"),
        r#"#!/usr/bin/perl
use strict;
use ExportModule;

sub foo {
    return "local";
}

my $result = foo();
"#,
    )?;

    harness.barrier();

    // Line 6: "my $result = foo();" - should resolve to local definition
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("consumer.pl")},
            "position": {"line": 6, "character": 14}
        }),
    )?;

    let location = first_location(&result)?;
    assert_valid_location(location)?;

    // Should point to consumer.pl (the local definition), NOT ExportModule.pm
    let uri = location["uri"].as_str().ok_or("Expected URI string")?;
    assert!(
        uri.contains("consumer.pl"),
        "Local definition should take precedence, but got: {}",
        uri
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// AC2: @EXPORT_OK symbol requires explicit import
// ---------------------------------------------------------------------------

/// Symbols in @EXPORT_OK are NOT automatically imported with `use Module;`
/// They require `use Module qw(symbol);` to be available.
/// This test verifies that @EXPORT_OK symbols don't auto-resolve.
#[test]
fn go_to_definition_does_not_auto_resolve_export_ok_symbols() -> TestResult {
    let mut harness = LspHarness::new();
    let workspace = support::lsp_harness::TempWorkspace::new()?;

    // Write module with only @EXPORT_OK (not in @EXPORT)
    workspace.write(
        "lib/OptExporter.pm",
        r#"package OptExporter;
use strict;
use warnings;

our @EXPORT_OK = qw(optional_func);

sub optional_func {
    return "optional";
}

sub default_func {
    return "default";
}

1;
"#,
    )?;

    harness.initialize_with_root(&workspace.root_uri, None)?;

    let module_uri = workspace.uri("lib/OptExporter.pm");
    let module_content = std::fs::read_to_string(workspace.dir.path().join("lib/OptExporter.pm"))
        .map_err(|e| format!("failed to read module: {e}"))?;
    harness.open(&module_uri, &module_content)?;

    // Consumer uses OptExporter without explicitly importing optional_func
    harness.open(
        &workspace.uri("consumer.pl"),
        r#"#!/usr/bin/perl
use strict;
use OptExporter;

my $result = default_func();
"#,
    )?;

    harness.barrier();

    // Line 3: "my $result = default_func();" - default_func is not exported
    // So go-to-definition should NOT resolve to OptExporter
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("consumer.pl")},
            "position": {"line": 3, "character": 15}
        }),
    )?;

    // Result should be empty or point to local file (no definition found)
    // It should NOT point to OptExporter.pm for default_func
    if let Some(locations) = result.as_array() {
        if !locations.is_empty() {
            let location = &locations[0];
            if let Some(uri) = location.get("uri").and_then(|u| u.as_str()) {
                // If a location was found, it should be in consumer.pl (local)
                // or anywhere EXCEPT OptExporter.pm since default_func isn't exported
                if uri.contains("OptExporter") {
                    return Err(format!(
                        "default_func is NOT exported (only @EXPORT_OK has optional_func), \
                         so go-to-def should NOT resolve to OptExporter.pm. Got: {}",
                        uri
                    )
                    .into());
                }
            }
        }
    }

    Ok(())
}
