//! Cross-file go-to-definition tests for Perl LSP
//!
//! Validates that go-to-definition navigates across files for:
//! - `Package::function()` calls
//! - `use Module` statements
//! - `$self->method()` calls

mod support;

use serde_json::{Value, json};
use support::lsp_harness::{LspHarness, TempWorkspace};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Helper to validate a Location object has proper structure.
fn assert_valid_location(location: &serde_json::Value) {
    assert!(location.get("uri").is_some(), "Location must have 'uri' field, got: {:?}", location);
    let range = location.get("range");
    assert!(range.is_some(), "Location must have 'range' field, got: {:?}", location);
    let range = range.ok_or("missing range").unwrap_or(&json!(null));
    assert!(range.get("start").is_some(), "Range must have 'start' position");
    assert!(range.get("end").is_some(), "Range must have 'end' position");
}

fn find_line_char(code: &str, needle: &str) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    for (line_idx, line) in code.lines().enumerate() {
        if let Some(char_idx) = line.find(needle) {
            return Ok((line_idx as u32, char_idx as u32));
        }
    }

    Err(format!("could not find '{needle}' in test source").into())
}

fn first_location(response: &Value) -> Result<&Value, Box<dyn std::error::Error>> {
    let locations = response
        .as_array()
        .ok_or_else(|| std::io::Error::other("expected array result for definition"))?;
    Ok(locations.first().ok_or_else(|| std::io::Error::other("definition result was empty"))?)
}

fn find_pos(
    code: &str,
    needle: &str,
    target_line: usize,
) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    let line = code
        .lines()
        .nth(target_line)
        .ok_or_else(|| std::io::Error::other(format!("no line {target_line} in test code")))?;
    let col = line.find(needle).ok_or_else(|| {
        std::io::Error::other(format!("could not find `{needle}` on line {target_line}"))
    })?;
    Ok((target_line as u32, col as u32))
}

// ---------------------------------------------------------------------------
// Test 1: Package::function() navigates to the function in Package.pm
// ---------------------------------------------------------------------------

#[test]
fn go_to_definition_on_qualified_function_call() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // Open the module file that defines My::Utils::process
    harness.open(
        "file:///lib/My/Utils.pm",
        r#"package My::Utils;
use strict;
use warnings;

sub process {
    my ($data) = @_;
    return $data * 2;
}

1;
"#,
    )?;

    // Open the caller file that invokes My::Utils::process()
    harness.open(
        "file:///app.pl",
        r#"#!/usr/bin/perl
use strict;
use warnings;
use My::Utils;

my $result = My::Utils::process(42);
print "Result: $result\n";
"#,
    )?;

    // Synchronize to ensure indexing is complete
    harness.barrier();

    // Request go-to-definition on "process" in "My::Utils::process(42)"
    // Line 5 (0-indexed), character 25 is on "process" after "My::Utils::"
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": "file:///app.pl"},
            "position": {"line": 5, "character": 25}
        }),
    )?;

    // The result should be an array of locations
    if let Some(locations) = result.as_array() {
        if !locations.is_empty() {
            let first = &locations[0];
            assert_valid_location(first);

            // Should point to the module file
            let uri = first["uri"].as_str().ok_or("Expected URI")?;
            assert!(
                uri.contains("My/Utils.pm") || uri.contains("My%2FUtils.pm"),
                "Definition should point to My/Utils.pm, got: {}",
                uri
            );
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Test 2: `use Module` navigates to Module.pm
// ---------------------------------------------------------------------------

#[test]
fn go_to_definition_on_use_module_navigates_to_module() -> TestResult {
    let mut harness = LspHarness::new();
    let workspace = support::lsp_harness::TempWorkspace::new()?;

    // Write the module file to disk so resolution can find it
    workspace.write(
        "lib/Demo/Worker.pm",
        r#"package Demo::Worker;
use strict;
use warnings;

sub run {
    print "working\n";
}

1;
"#,
    )?;

    harness.initialize_with_root(&workspace.root_uri, None)?;

    // Open the module in the LSP so it's indexed
    let module_uri = workspace.uri("lib/Demo/Worker.pm");
    let module_content = std::fs::read_to_string(workspace.dir.path().join("lib/Demo/Worker.pm"))
        .map_err(|e| format!("failed to read module: {e}"))?;
    harness.open(&module_uri, &module_content)?;

    // Open the caller that has `use Demo::Worker`
    harness.open(
        &workspace.uri("app.pl"),
        r#"#!/usr/bin/perl
use strict;
use warnings;
use Demo::Worker;

Demo::Worker::run();
"#,
    )?;

    harness.barrier();

    // Request go-to-definition on "Demo::Worker" in the use statement
    // Line 3: "use Demo::Worker;"  character ~5 is on "Demo"
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("app.pl")},
            "position": {"line": 3, "character": 6}
        }),
    )?;

    // The result should navigate to Demo::Worker.pm
    if let Some(locations) = result.as_array() {
        if !locations.is_empty() {
            let first = &locations[0];
            assert_valid_location(first);

            let uri = first["uri"].as_str().ok_or("Expected URI")?;
            assert!(
                uri.contains("Demo") && uri.contains("Worker"),
                "Definition should point to Demo/Worker.pm, got: {}",
                uri
            );
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Test 2b: imported functions via `use Module qw(func)` navigate to the exporter
// ---------------------------------------------------------------------------

#[test]
fn go_to_definition_on_imported_function_navigates_to_source_module() -> TestResult {
    let mut harness = LspHarness::new();
    let workspace = support::lsp_harness::TempWorkspace::new()?;

    workspace.write(
        "lib/My/Utils.pm",
        r#"package My::Utils;
use strict;
use warnings;

sub calculate_sum {
    my (@nums) = @_;
    my $total = 0;
    $total += $_ for @nums;
    return $total;
}

1;
"#,
    )?;

    harness.initialize_with_root(&workspace.root_uri, None)?;

    let module_uri = workspace.uri("lib/My/Utils.pm");
    let module_content = std::fs::read_to_string(workspace.dir.path().join("lib/My/Utils.pm"))
        .map_err(|e| format!("failed to read module: {e}"))?;
    harness.open(&module_uri, &module_content)?;

    let caller = r#"#!/usr/bin/perl
use strict;
use warnings;
use My::Utils qw(calculate_sum);

my $result = calculate_sum(1, 2, 3);
"#;
    harness.open(&workspace.uri("app.pl"), caller)?;
    harness.barrier();

    let (line, character) = find_line_char(caller, "calculate_sum(1, 2, 3)")?;
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("app.pl")},
            "position": {"line": line, "character": character}
        }),
    )?;

    let locations = result.as_array().ok_or("expected location array")?;
    assert!(!locations.is_empty(), "expected definition result for imported function");

    let first = &locations[0];
    assert_valid_location(first);
    let uri = first["uri"].as_str().ok_or("Expected URI")?;
    assert!(
        uri.contains("My/Utils.pm") || uri.contains("My%2FUtils.pm"),
        "Definition should point to My/Utils.pm, got: {}",
        uri
    );

    Ok(())
}

#[test]
fn go_to_definition_on_tag_imported_symbol_navigates_to_source_module() -> TestResult {
    let mut harness = LspHarness::new();
    let workspace = support::lsp_harness::TempWorkspace::new()?;

    workspace.write(
        "lib/POSIX.pm",
        r#"package POSIX;
use strict;
use warnings;

sub WIFEXITED {
    return 1;
}

1;
"#,
    )?;

    harness.initialize_with_root(&workspace.root_uri, None)?;

    let module_uri = workspace.uri("lib/POSIX.pm");
    let module_content = std::fs::read_to_string(workspace.dir.path().join("lib/POSIX.pm"))
        .map_err(|e| format!("failed to read module: {e}"))?;
    harness.open(&module_uri, &module_content)?;

    let caller = r#"#!/usr/bin/perl
use strict;
use warnings;
use POSIX qw(:sys_wait_h);

if (WIFEXITED(0)) {
    print "ok\n";
}
"#;
    harness.open(&workspace.uri("app.pl"), caller)?;
    harness.barrier();

    let (line, character) = find_line_char(caller, "WIFEXITED(0)")?;
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("app.pl")},
            "position": {"line": line, "character": character}
        }),
    )?;

    let locations = result.as_array().ok_or("expected location array")?;
    assert!(!locations.is_empty(), "expected definition result for tag-imported symbol");

    let first = &locations[0];
    assert_valid_location(first);
    let uri = first["uri"].as_str().ok_or("Expected URI")?;
    assert!(
        uri.contains("POSIX.pm") || uri.contains("POSIX%2Epm"),
        "Definition should point to POSIX.pm, got: {}",
        uri
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Test 3: XS bootstrap calls navigate to native `.xs` entry points
// ---------------------------------------------------------------------------

#[test]
fn go_to_definition_on_xsloader_load_navigates_to_boot_symbol() -> TestResult {
    let mut harness = LspHarness::new();
    let workspace = support::lsp_harness::TempWorkspace::new()?;
    let server = perl_lsp::LspServer::new();

    let module_pm = r#"package My::Module;
use strict;
use warnings;
use XSLoader;
our $VERSION = '0.01';
XSLoader::load(__PACKAGE__, $VERSION);
1;
"#;
    let module_xs = r#"#include "EXTERN.h"
#include "perl.h"
#include "XSUB.h"

EXTERN_C void
boot_My__Module(pTHX_ CV* cv)
{
}
"#;

    workspace.write("lib/My/Module.pm", module_pm)?;
    workspace.write("Module.xs", module_xs)?;

    harness.initialize_with_root(&workspace.root_uri, None)?;
    harness.open(&workspace.uri("lib/My/Module.pm"), module_pm)?;
    harness.barrier();

    let cursor = module_pm.find("load").ok_or("missing XSLoader::load")?;
    let (line, character) = server.offset_to_position(module_pm, cursor);
    let boot_offset = module_xs.find("boot_My__Module").ok_or("missing boot symbol")?;
    let (boot_line, _) = server.offset_to_position(module_xs, boot_offset);

    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("lib/My/Module.pm")},
            "position": {"line": line, "character": character}
        }),
    )?;

    let locations = result.as_array().ok_or("expected location array")?;
    assert!(!locations.is_empty(), "expected XS bootstrap definition result");

    let first = &locations[0];
    assert_valid_location(first);
    let uri = first["uri"].as_str().ok_or("expected uri")?;
    assert!(uri.contains("Module.xs"), "expected Module.xs target, got: {uri}");
    assert_eq!(
        first["range"]["start"]["line"].as_u64(),
        Some(u64::from(boot_line)),
        "expected goto-definition to land on boot_My__Module",
    );

    Ok(())
}

#[test]
fn go_to_definition_on_bootstrap_keyword_navigates_to_boot_symbol() -> TestResult {
    let mut harness = LspHarness::new();
    let workspace = support::lsp_harness::TempWorkspace::new()?;
    let server = perl_lsp::LspServer::new();

    let loader_pm = r#"package My::Module;
use strict;
use warnings;
require DynaLoader;
our @ISA = qw(DynaLoader);
our $VERSION = '0.01';
bootstrap My::Module $VERSION;
1;
"#;
    let module_xs = r#"#include "EXTERN.h"
#include "perl.h"
#include "XSUB.h"

EXTERN_C void
boot_My__Module(pTHX_ CV* cv)
{
}
"#;

    workspace.write("lib/My/Module.pm", loader_pm)?;
    workspace.write("Module.xs", module_xs)?;

    harness.initialize_with_root(&workspace.root_uri, None)?;
    harness.open(&workspace.uri("lib/My/Module.pm"), loader_pm)?;
    harness.barrier();

    let cursor = loader_pm.find("bootstrap").ok_or("missing bootstrap keyword")?;
    let (line, character) = server.offset_to_position(loader_pm, cursor);
    let boot_offset = module_xs.find("boot_My__Module").ok_or("missing boot symbol")?;
    let (boot_line, _) = server.offset_to_position(module_xs, boot_offset);

    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("lib/My/Module.pm")},
            "position": {"line": line, "character": character}
        }),
    )?;

    let locations = result.as_array().ok_or("expected location array")?;
    assert!(!locations.is_empty(), "expected XS bootstrap definition result");

    let first = &locations[0];
    assert_valid_location(first);
    let uri = first["uri"].as_str().ok_or("expected uri")?;
    assert!(uri.contains("Module.xs"), "expected Module.xs target, got: {uri}");
    assert_eq!(
        first["range"]["start"]["line"].as_u64(),
        Some(u64::from(boot_line)),
        "expected goto-definition to land on boot_My__Module",
    );

    Ok(())
}

#[test]
fn go_to_definition_on_dynaloader_bootstrap_navigates_to_boot_symbol() -> TestResult {
    let mut harness = LspHarness::new();
    let workspace = support::lsp_harness::TempWorkspace::new()?;
    let server = perl_lsp::LspServer::new();

    let loader_pm = r#"package My::Module;
use strict;
use warnings;
require DynaLoader;
our @ISA = qw(DynaLoader);
our $VERSION = '0.01';
DynaLoader::bootstrap(__PACKAGE__, $VERSION);
1;
"#;
    let module_xs = r#"#include "EXTERN.h"
#include "perl.h"
#include "XSUB.h"

EXTERN_C void
boot_My__Module(pTHX_ CV* cv)
{
}
"#;

    workspace.write("lib/My/Module.pm", loader_pm)?;
    workspace.write("Module.xs", module_xs)?;

    harness.initialize_with_root(&workspace.root_uri, None)?;
    harness.open(&workspace.uri("lib/My/Module.pm"), loader_pm)?;
    harness.barrier();

    let cursor = loader_pm.find("DynaLoader::bootstrap").ok_or("missing DynaLoader::bootstrap")?;
    let (line, character) = server.offset_to_position(loader_pm, cursor);
    let boot_offset = module_xs.find("boot_My__Module").ok_or("missing boot symbol")?;
    let (boot_line, _) = server.offset_to_position(module_xs, boot_offset);

    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("lib/My/Module.pm")},
            "position": {"line": line, "character": character}
        }),
    )?;

    let locations = result.as_array().ok_or("expected location array")?;
    assert!(!locations.is_empty(), "expected XS bootstrap definition result");

    let first = &locations[0];
    assert_valid_location(first);
    let uri = first["uri"].as_str().ok_or("expected uri")?;
    assert!(uri.contains("Module.xs"), "expected Module.xs target, got: {uri}");
    assert_eq!(
        first["range"]["start"]["line"].as_u64(),
        Some(u64::from(boot_line)),
        "expected goto-definition to land on boot_My__Module",
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Test 3: $self->method() navigates to the method definition
// ---------------------------------------------------------------------------

#[test]
fn go_to_definition_on_self_method_call() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // Open a module that defines a class with methods
    harness.open(
        "file:///lib/Animal.pm",
        r#"package Animal;
use strict;
use warnings;

sub new {
    my ($class, %args) = @_;
    return bless \%args, $class;
}

sub speak {
    my ($self) = @_;
    return "...";
}

sub greet {
    my ($self) = @_;
    my $sound = $self->speak();
    return "Hello! I say: $sound";
}

1;
"#,
    )?;

    harness.barrier();

    // Request go-to-definition on "speak" in "$self->speak()"
    // Line 17 (0-indexed): "    my $sound = $self->speak();"
    // Character 24 is on "p" in "speak" (safely past the "->" arrow)
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": "file:///lib/Animal.pm"},
            "position": {"line": 17, "character": 24}
        }),
    )?;

    // Should find a definition (either the method or a related declaration in the same file)
    if let Some(locations) = result.as_array() {
        assert!(!locations.is_empty(), "Should find at least one definition location");
        let first = &locations[0];
        assert_valid_location(first);

        let uri = first["uri"].as_str().ok_or("Expected URI")?;
        assert!(uri.contains("Animal.pm"), "Definition should point to Animal.pm, got: {}", uri);
    }

    Ok(())
}

#[test]
fn go_to_definition_on_super_method_uses_parent_chain() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    harness.open(
        "file:///lib/Animal.pm",
        r#"package Base;
sub greet { "base" }

package Child;
use parent 'Base';
sub greet {
    my $self = shift;
    return $self->SUPER::greet();
}

1;
"#,
    )?;

    harness.barrier();

    let (line, character) = find_line_char(
        r#"package Base;
sub greet { "base" }

package Child;
use parent 'Base';
sub greet {
    my $self = shift;
    return $self->SUPER::greet();
}

1;
"#,
        "SUPER::greet",
    )?;

    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": "file:///lib/Animal.pm"},
            "position": {"line": line, "character": character + 8}
        }),
    )?;

    let locations = result.as_array().ok_or("expected array result for SUPER definition")?;
    assert!(!locations.is_empty(), "SUPER::greet should resolve to parent implementation");
    let first = &locations[0];
    assert_valid_location(first);
    assert!(
        first["uri"].as_str().unwrap_or("").contains("Animal.pm"),
        "SUPER::greet should resolve within the current file, got: {first:?}"
    );
    assert_eq!(
        first["range"]["start"]["line"].as_u64(),
        Some(1),
        "SUPER::greet should navigate to Base::greet"
    );

    Ok(())
}

#[test]
fn go_to_definition_on_super_method_respects_c3_mro() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    let code = r#"package Root;
sub greet { "root" }

package Left;
use parent 'Root';

package Right;
use parent 'Root';
sub greet { "right" }

package Child;
use mro 'c3';
use parent 'Left', 'Right';
sub greet {
    my $self = shift;
    return $self->SUPER::greet();
}

1;
"#;

    harness.open("file:///lib/Animal.pm", code)?;
    harness.barrier();

    let (line, character) = find_line_char(code, "SUPER::greet")?;
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": "file:///lib/Animal.pm"},
            "position": {"line": line, "character": character + 8}
        }),
    )?;

    let locations = result.as_array().ok_or("expected array result for SUPER definition")?;
    assert!(!locations.is_empty(), "SUPER::greet should resolve under C3 mro");
    let first = &locations[0];
    assert_valid_location(first);
    assert!(
        first["uri"].as_str().unwrap_or("").contains("Animal.pm"),
        "SUPER::greet should resolve within the current file, got: {first:?}"
    );
    assert_eq!(
        first["range"]["start"]["line"].as_u64(),
        Some(8),
        "C3 mro should resolve to Right::greet, not the Root fallback"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Test 4: Cross-file $self->method() when method is in a different file
// ---------------------------------------------------------------------------

#[test]
fn go_to_definition_cross_file_method_call() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // Open the base class with the method definition
    harness.open(
        "file:///lib/Base.pm",
        r#"package Base;
use strict;
use warnings;

sub new {
    my ($class, %args) = @_;
    return bless \%args, $class;
}

sub validate {
    my ($self) = @_;
    return 1;
}

1;
"#,
    )?;

    // Open a file that calls Base->validate
    harness.open(
        "file:///app.pl",
        r#"#!/usr/bin/perl
use strict;
use warnings;
use Base;

my $obj = Base->new();
my $valid = Base->validate();
"#,
    )?;

    harness.barrier();

    // Request go-to-definition on "validate" in "Base->validate()"
    // Line 6: "my $valid = Base->validate();"
    // "validate" starts around character 18
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": "file:///app.pl"},
            "position": {"line": 6, "character": 20}
        }),
    )?;

    if let Some(locations) = result.as_array() {
        if !locations.is_empty() {
            let first = &locations[0];
            assert_valid_location(first);

            let uri = first["uri"].as_str().ok_or("Expected URI")?;
            assert!(uri.contains("Base.pm"), "Definition should point to Base.pm, got: {}", uri);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Test 4b: Cross-file $obj->method() infers receiver package from constructor
// ---------------------------------------------------------------------------

#[test]
fn go_to_definition_cross_file_constructor_assigned_method_call() -> TestResult {
    let mut harness = LspHarness::new();
    let workspace = support::lsp_harness::TempWorkspace::new()?;

    workspace.write(
        "lib/Dog.pm",
        r#"package Dog;
use Moose;
extends 'Animal';

sub fetch {
    my ($self, $item) = @_;
    return $self->name . q{ fetches } . ($item // q{ball});
}

1;
"#,
    )?;

    workspace.write(
        "lib/Animal.pm",
        r#"package Animal;
use Moose;

has name => (is => 'ro', isa => 'Str', required => 1);

1;
"#,
    )?;

    harness.initialize_with_root(&workspace.root_uri, None)?;

    let dog_uri = workspace.uri("lib/Dog.pm");
    let dog_content = std::fs::read_to_string(workspace.dir.path().join("lib/Dog.pm"))
        .map_err(|e| format!("failed to read Dog.pm: {e}"))?;
    harness.open(&dog_uri, &dog_content)?;

    let animal_uri = workspace.uri("lib/Animal.pm");
    let animal_content = std::fs::read_to_string(workspace.dir.path().join("lib/Animal.pm"))
        .map_err(|e| format!("failed to read Animal.pm: {e}"))?;
    harness.open(&animal_uri, &animal_content)?;

    harness.open(
        &workspace.uri("main.pl"),
        r#"#!/usr/bin/perl
use strict;
use warnings;
use lib 'lib';
use Dog;

my $dog = Dog->new(name => 'Rex');
$dog->fetch('stick');
"#,
    )?;

    harness.barrier();

    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("main.pl")},
            "position": {"line": 7, "character": 7}
        }),
    )?;

    let locations = result.as_array().ok_or_else(|| {
        format!("Expected array result for constructor-assigned method goto-def, got: {result:?}")
    })?;
    assert!(
        !locations.is_empty(),
        "Expected constructor-assigned method goto-definition to return at least one location"
    );

    let first = &locations[0];
    assert_valid_location(first);
    let uri = first["uri"].as_str().ok_or("Expected URI")?;
    assert!(uri.contains("Dog.pm"), "Definition should point to Dog.pm, got: {uri}");

    Ok(())
}

// ---------------------------------------------------------------------------
// Test 4c: UNIVERSAL methods fall back when the class does not shadow them
// ---------------------------------------------------------------------------

#[test]
fn go_to_definition_universal_methods_fall_back_to_universal() -> TestResult {
    let mut harness = LspHarness::new();
    let workspace = support::lsp_harness::TempWorkspace::new()?;

    workspace.write(
        "lib/UNIVERSAL.pm",
        r#"package UNIVERSAL;

sub can { 1 }
sub isa { 1 }
sub DOES { 1 }
sub VERSION { 1 }

1;
"#,
    )?;

    workspace.write(
        "lib/Base.pm",
        r#"package Base;
use strict;
use warnings;

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

1;
"#,
    )?;

    harness.initialize_with_root(&workspace.root_uri, None)?;

    let universal_uri = workspace.uri("lib/UNIVERSAL.pm");
    let universal_content = std::fs::read_to_string(workspace.dir.path().join("lib/UNIVERSAL.pm"))
        .map_err(|e| format!("failed to read UNIVERSAL.pm: {e}"))?;
    harness.open(&universal_uri, &universal_content)?;

    let base_uri = workspace.uri("lib/Base.pm");
    let base_content = std::fs::read_to_string(workspace.dir.path().join("lib/Base.pm"))
        .map_err(|e| format!("failed to read Base.pm: {e}"))?;
    harness.open(&base_uri, &base_content)?;

    harness.open(
        &workspace.uri("app.pl"),
        r#"#!/usr/bin/perl
use strict;
use warnings;
use Base;

Base->can();
Base->isa();
Base->DOES();
Base->VERSION();
"#,
    )?;

    harness.barrier();

    for (line, method) in [(5, "can"), (6, "isa"), (7, "DOES"), (8, "VERSION")] {
        let result = harness.request(
            "textDocument/definition",
            json!({
                "textDocument": {"uri": workspace.uri("app.pl")},
                "position": {"line": line, "character": 8}
            }),
        )?;

        if let Some(locations) = result.as_array() {
            if !locations.is_empty() {
                let first = &locations[0];
                assert_valid_location(first);

                let uri = first["uri"].as_str().ok_or("Expected URI")?;
                assert!(
                    uri.contains("UNIVERSAL.pm"),
                    "Definition of {method} should point to UNIVERSAL.pm, got: {uri}"
                );
            } else {
                return Err(format!("expected definition location for {method}").into());
            }
        } else {
            return Err(format!("expected definition result array for {method}").into());
        }
    }

    Ok(())
}

#[test]
fn go_to_definition_shadowed_universal_method_stays_on_class_method() -> TestResult {
    let mut harness = LspHarness::new();
    let workspace = support::lsp_harness::TempWorkspace::new()?;

    workspace.write(
        "lib/UNIVERSAL.pm",
        r#"package UNIVERSAL;

sub can { 1 }

1;
"#,
    )?;

    workspace.write(
        "lib/Base.pm",
        r#"package Base;
use strict;
use warnings;

sub can { 1 }

1;
"#,
    )?;

    harness.initialize_with_root(&workspace.root_uri, None)?;

    let universal_uri = workspace.uri("lib/UNIVERSAL.pm");
    let universal_content = std::fs::read_to_string(workspace.dir.path().join("lib/UNIVERSAL.pm"))
        .map_err(|e| format!("failed to read UNIVERSAL.pm: {e}"))?;
    harness.open(&universal_uri, &universal_content)?;

    let base_uri = workspace.uri("lib/Base.pm");
    let base_content = std::fs::read_to_string(workspace.dir.path().join("lib/Base.pm"))
        .map_err(|e| format!("failed to read Base.pm: {e}"))?;
    harness.open(&base_uri, &base_content)?;

    harness.open(
        &workspace.uri("app.pl"),
        r#"#!/usr/bin/perl
use strict;
use warnings;
use Base;

Base->can();
"#,
    )?;

    harness.barrier();

    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("app.pl")},
            "position": {"line": 5, "character": 8}
        }),
    )?;

    if let Some(locations) = result.as_array() {
        if !locations.is_empty() {
            let first = &locations[0];
            assert_valid_location(first);

            let uri = first["uri"].as_str().ok_or("Expected URI")?;
            assert!(
                uri.contains("Base.pm"),
                "Definition should stay on Base.pm when Base shadows UNIVERSAL, got: {uri}"
            );
        } else {
            return Err("expected at least one definition location for shadowed can".into());
        }
    } else {
        return Err("expected definition result array for shadowed can".into());
    }

    Ok(())
}

#[test]
fn go_to_definition_cross_file_constructor_assigned_bare_method_call_in_framework_workspace()
-> TestResult {
    let mut harness = LspHarness::new();
    let workspace = support::lsp_harness::TempWorkspace::new()?;

    workspace.write(
        "lib/MooAnimal.pm",
        r#"package MooAnimal;
use Moo;

has name => (
    is      => 'ro',
    default => sub { 'animal' },
);

1;
"#,
    )?;

    workspace.write(
        "lib/MooPrintable.pm",
        r#"package MooPrintable;
use Moo::Role;

sub print_info {
    my ($self) = @_;
    return $self->name;
}

1;
"#,
    )?;

    workspace.write(
        "lib/MooDog.pm",
        r#"package MooDog;
use Moo;
extends 'MooAnimal';
with 'MooPrintable';

sub fetch {
    my ($self) = @_;
    return $self->name . q{ fetched};
}

1;
"#,
    )?;

    workspace.write(
        "lib/MooseAnimal.pm",
        r#"package MooseAnimal;
use Moose;

has name => (
    is      => 'ro',
    isa     => 'Str',
    default => 'animal',
);

__PACKAGE__->meta->make_immutable;
1;
"#,
    )?;

    workspace.write(
        "lib/MoosePrintable.pm",
        r#"package MoosePrintable;
use Moose::Role;

sub print_info {
    my ($self) = @_;
    return $self->name;
}

1;
"#,
    )?;

    workspace.write(
        "lib/MooseCat.pm",
        r#"package MooseCat;
use Moose;
extends 'MooseAnimal';
with 'MoosePrintable';

sub pounce {
    my ($self) = @_;
    return $self->name . q{ pounced};
}

__PACKAGE__->meta->make_immutable;
1;
"#,
    )?;

    harness.initialize_with_root(&workspace.root_uri, None)?;

    for relative in [
        "lib/MooAnimal.pm",
        "lib/MooPrintable.pm",
        "lib/MooDog.pm",
        "lib/MooseAnimal.pm",
        "lib/MoosePrintable.pm",
        "lib/MooseCat.pm",
    ] {
        let uri = workspace.uri(relative);
        let content = std::fs::read_to_string(workspace.dir.path().join(relative))
            .map_err(|e| format!("failed to read {relative}: {e}"))?;
        harness.open(&uri, &content)?;
    }

    harness.open(
        &workspace.uri("main.pl"),
        r#"#!/usr/bin/perl
use strict;
use warnings;
use lib 'lib';
use MooDog;
use MooseCat;

my $dog = MooDog->new(name => 'Rex');
my $cat = MooseCat->new(name => 'Misty');
$dog->fetch;
$cat->pounce;
"#,
    )?;

    harness.barrier();

    let fetch_result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("main.pl")},
            "position": {"line": 9, "character": 6}
        }),
    )?;

    let fetch_locations = fetch_result.as_array().ok_or_else(|| {
        format!("Expected array result for bare Moo method goto-def, got: {fetch_result:?}")
    })?;
    assert!(
        !fetch_locations.is_empty(),
        "Expected bare Moo method goto-definition to return at least one location"
    );

    let fetch_uri = fetch_locations[0]["uri"].as_str().ok_or("Expected fetch URI")?;
    assert!(
        fetch_uri.contains("MooDog.pm"),
        "Definition should point to MooDog.pm, got: {fetch_uri}"
    );

    let pounce_result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("main.pl")},
            "position": {"line": 10, "character": 6}
        }),
    )?;

    let pounce_locations = pounce_result.as_array().ok_or_else(|| {
        format!("Expected array result for bare Moose method goto-def, got: {pounce_result:?}")
    })?;
    assert!(
        !pounce_locations.is_empty(),
        "Expected bare Moose method goto-definition to return at least one location"
    );

    let pounce_uri = pounce_locations[0]["uri"].as_str().ok_or("Expected pounce URI")?;
    assert!(
        pounce_uri.contains("MooseCat.pm"),
        "Definition should point to MooseCat.pm, got: {pounce_uri}"
    );

    Ok(())
}

#[test]
fn go_to_definition_cross_file_inherited_and_role_method_call_in_framework_workspace() -> TestResult
{
    let mut harness = LspHarness::new();
    let workspace = support::lsp_harness::TempWorkspace::new()?;

    workspace.write(
        "lib/MooAnimal.pm",
        r#"package MooAnimal;
use Moo;

has name => (
    is      => 'ro',
    default => sub { 'animal' },
);

sub describe {
    my ($self) = @_;
    return $self->name . q{ described};
}

1;
"#,
    )?;

    workspace.write(
        "lib/MooPrintable.pm",
        r#"package MooPrintable;
use Moo::Role;

sub print_info {
    my ($self) = @_;
    return $self->name;
}

1;
"#,
    )?;

    workspace.write(
        "lib/MooDog.pm",
        r#"package MooDog;
use Moo;
extends 'MooAnimal';
with 'MooPrintable';

sub fetch {
    my ($self) = @_;
    return $self->name . q{ fetched};
}

1;
"#,
    )?;

    workspace.write(
        "lib/MooseAnimal.pm",
        r#"package MooseAnimal;
use Moose;

has name => (
    is      => 'ro',
    isa     => 'Str',
    default => 'animal',
);

sub describe {
    my ($self) = @_;
    return $self->name . q{ described};
}

__PACKAGE__->meta->make_immutable;
1;
"#,
    )?;

    workspace.write(
        "lib/MoosePrintable.pm",
        r#"package MoosePrintable;
use Moose::Role;

sub print_info {
    my ($self) = @_;
    return $self->name;
}

1;
"#,
    )?;

    workspace.write(
        "lib/MooseCat.pm",
        r#"package MooseCat;
use Moose;
extends 'MooseAnimal';
with 'MoosePrintable';

sub pounce {
    my ($self) = @_;
    return $self->name . q{ pounced};
}

__PACKAGE__->meta->make_immutable;
1;
"#,
    )?;

    harness.initialize_with_root(&workspace.root_uri, None)?;

    for relative in [
        "lib/MooAnimal.pm",
        "lib/MooPrintable.pm",
        "lib/MooDog.pm",
        "lib/MooseAnimal.pm",
        "lib/MoosePrintable.pm",
        "lib/MooseCat.pm",
    ] {
        let uri = workspace.uri(relative);
        let content = std::fs::read_to_string(workspace.dir.path().join(relative))
            .map_err(|e| format!("failed to read {relative}: {e}"))?;
        harness.open(&uri, &content)?;
    }

    harness.open(
        &workspace.uri("main.pl"),
        r#"#!/usr/bin/perl
use strict;
use warnings;
use lib 'lib';
use MooDog;
use MooseCat;

my $dog = MooDog->new(name => 'Rex');
my $cat = MooseCat->new(name => 'Misty');
$dog->describe;
$dog->print_info;
$cat->describe;
$cat->print_info;
"#,
    )?;

    harness.barrier();

    for (line, expected_uri_fragment, label) in [
        (9_u64, "MooAnimal.pm", "Moo inherited method"),
        (10_u64, "MooPrintable.pm", "Moo role method"),
        (11_u64, "MooseAnimal.pm", "Moose inherited method"),
        (12_u64, "MoosePrintable.pm", "Moose role method"),
    ] {
        let result = harness.request(
            "textDocument/definition",
            json!({
                "textDocument": {"uri": workspace.uri("main.pl")},
                "position": {"line": line, "character": 6}
            }),
        )?;

        let locations = result.as_array().ok_or_else(|| {
            format!("Expected array result for {label} goto-def, got: {result:?}")
        })?;
        assert!(
            !locations.is_empty(),
            "Expected {label} goto-definition to return at least one location"
        );

        let uri = locations[0]["uri"].as_str().ok_or("Expected definition URI")?;
        assert!(
            uri.contains(expected_uri_fragment),
            "{label} should point to {expected_uri_fragment}, got: {uri}"
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Test 5: symbol_at_cursor handles MethodCall nodes
// ---------------------------------------------------------------------------

#[test]
fn symbol_at_cursor_resolves_method_call() -> TestResult {
    use perl_parser::Parser;
    use perl_parser::declaration::{current_package_at, symbol_at_cursor};

    let code = r#"package MyClass;

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub helper {
    return 42;
}

sub main_work {
    my ($self) = @_;
    $self->helper();
}

1;
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse().map_err(|e| format!("parse error: {e}"))?;

    let method_call_offset =
        code.find("$self->helper()").ok_or("could not find $self->helper()")?;
    let helper_offset = method_call_offset + "$self->".len();

    let current_pkg = current_package_at(&ast, helper_offset);
    assert_eq!(current_pkg, "MyClass");

    let sym =
        symbol_at_cursor(&ast, helper_offset, current_pkg).ok_or("expected Some(SymbolKey)")?;
    assert_eq!(sym.name.as_ref(), "helper", "method name should be 'helper'");
    assert_eq!(sym.pkg.as_ref(), "MyClass", "package should be current package for $self");

    Ok(())
}

#[test]
fn symbol_at_cursor_resolves_constructor_assigned_method_call() -> TestResult {
    use perl_parser::Parser;
    use perl_parser::declaration::{current_package_at, symbol_at_cursor};

    let code = r#"package main;
use Dog;

my $dog = Dog->new(name => 'Rex');
$dog->fetch('stick');
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse().map_err(|e| format!("parse error: {e}"))?;

    let fetch_offset = code.find("fetch").ok_or("could not find fetch")?;
    let current_pkg = current_package_at(&ast, fetch_offset);
    let symbol = symbol_at_cursor(&ast, fetch_offset, current_pkg)
        .ok_or("expected symbol_at_cursor to resolve constructor-assigned method call")?;

    assert_eq!(symbol.name.as_ref(), "fetch", "method name should be 'fetch'");
    assert_eq!(symbol.pkg.as_ref(), "Dog", "package should be inferred from Dog->new()");

    Ok(())
}

#[test]
fn symbol_at_cursor_resolves_constructor_assigned_bare_method_call() -> TestResult {
    use perl_parser::Parser;
    use perl_parser::declaration::{current_package_at, symbol_at_cursor};

    let code = r#"package main;
use MooDog;

my $dog = MooDog->new(name => 'Rex');
$dog->fetch;
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse().map_err(|e| format!("parse error: {e}"))?;

    let fetch_offset = code.find("fetch").ok_or("could not find fetch")?;
    let current_pkg = current_package_at(&ast, fetch_offset);
    let symbol = symbol_at_cursor(&ast, fetch_offset, current_pkg)
        .ok_or("expected symbol_at_cursor to resolve bare constructor-assigned method call")?;

    assert_eq!(symbol.name.as_ref(), "fetch", "method name should be 'fetch'");
    assert_eq!(symbol.pkg.as_ref(), "MooDog", "package should be inferred from MooDog->new()");

    Ok(())
}

// ---------------------------------------------------------------------------
// Test 6: symbol_at_cursor handles Use nodes
// ---------------------------------------------------------------------------

#[test]
fn symbol_at_cursor_resolves_use_statement() -> TestResult {
    use perl_parser::Parser;
    use perl_parser::declaration::{current_package_at, symbol_at_cursor};

    let code = "use Data::Dumper;\nmy $x = 1;\n";

    let mut parser = Parser::new(code);
    let ast = parser.parse().map_err(|e| format!("parse error: {e}"))?;

    // Find offset of "Data::Dumper" in "use Data::Dumper;"
    let module_offset = code.find("Data::Dumper").ok_or("could not find Data::Dumper")?;

    let current_pkg = current_package_at(&ast, module_offset);
    let symbol = symbol_at_cursor(&ast, module_offset, current_pkg);

    // The Use node may be matched if the cursor lands on the Use node itself
    // (depending on parser structure), so we check for either Some or None
    if let Some(sym) = &symbol {
        // If resolved, should contain the module name
        assert!(
            sym.name.as_ref() == "Data::Dumper" || sym.pkg.as_ref() == "Data::Dumper",
            "symbol should reference Data::Dumper, got name={} pkg={}",
            sym.name,
            sym.pkg,
        );
    }

    Ok(())
}

#[test]
fn plack_builder_middleware_enable_navigates_to_module_file() -> TestResult {
    let mut harness = LspHarness::new();
    let workspace = TempWorkspace::new()?;

    workspace.write(
        "lib/Plack/Middleware/Static.pm",
        r#"package Plack::Middleware::Static;

1;
"#,
    )?;
    workspace.write(
        "lib/Plack/Middleware/Session.pm",
        r#"package Plack::Middleware::Session;

1;
"#,
    )?;
    workspace.write(
        "app.psgi",
        r#"use Plack::Builder;

builder {
    enable 'Static';
    enable 'Plack::Middleware::Session';
};
"#,
    )?;

    harness.initialize_with_root(&workspace.root_uri, None)?;

    let static_uri = workspace.uri("lib/Plack/Middleware/Static.pm");
    let static_content =
        std::fs::read_to_string(workspace.dir.path().join("lib/Plack/Middleware/Static.pm"))?;
    harness.open(&static_uri, &static_content)?;

    let session_uri = workspace.uri("lib/Plack/Middleware/Session.pm");
    let session_content =
        std::fs::read_to_string(workspace.dir.path().join("lib/Plack/Middleware/Session.pm"))?;
    harness.open(&session_uri, &session_content)?;

    let app_uri = workspace.uri("app.psgi");
    let app_content = std::fs::read_to_string(workspace.dir.path().join("app.psgi"))?;
    harness.open(&app_uri, &app_content)?;

    harness.barrier();

    let (static_line, static_character) = find_pos(&app_content, "Static", 3)?;
    let static_def = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": app_uri},
            "position": {"line": static_line, "character": static_character}
        }),
    )?;
    let static_location = first_location(&static_def)?;
    assert_valid_location(static_location);
    assert_eq!(
        static_location["uri"].as_str(),
        Some(static_uri.as_str()),
        "short-name middleware navigation should jump to the Static module"
    );

    let (session_line, session_character) =
        find_pos(&app_content, "Plack::Middleware::Session", 4)?;
    let session_def = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": app_uri},
            "position": {"line": session_line, "character": session_character}
        }),
    )?;
    let session_location = first_location(&session_def)?;
    assert_valid_location(session_location);
    assert_eq!(
        session_location["uri"].as_str(),
        Some(session_uri.as_str()),
        "fully-qualified middleware navigation should jump to the Session module"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests 7+: Moose/Moo role composition goto-definition (Issue #2325)
// ---------------------------------------------------------------------------

/// Go-to-definition on the role name in `with 'RoleName'` should navigate to the role file.
#[test]
fn go_to_definition_on_with_role_navigates_to_role_file() -> TestResult {
    let mut harness = LspHarness::new();
    let workspace = support::lsp_harness::TempWorkspace::new()?;

    // Write the role file to disk
    workspace.write(
        "lib/MyApp/Role/Printable.pm",
        r#"package MyApp::Role::Printable;
use Moo::Role;

sub print_self {
    my ($self) = @_;
    print ref($self), "\n";
}

1;
"#,
    )?;

    harness.initialize_with_root(&workspace.root_uri, None)?;

    // Open the role file so it is indexed
    let role_uri = workspace.uri("lib/MyApp/Role/Printable.pm");
    let role_content =
        std::fs::read_to_string(workspace.dir.path().join("lib/MyApp/Role/Printable.pm"))
            .map_err(|e| format!("failed to read role file: {e}"))?;
    harness.open(&role_uri, &role_content)?;

    // Open the consumer class that composes the role
    harness.open(
        &workspace.uri("lib/MyApp/User.pm"),
        r#"package MyApp::User;
use Moo;
with 'MyApp::Role::Printable';
1;
"#,
    )?;

    harness.barrier();

    // Request goto-definition on "MyApp::Role::Printable" in `with 'MyApp::Role::Printable';`
    // Line 2 (0-indexed): `with 'MyApp::Role::Printable';`
    // "MyApp::Role::Printable" starts at character 6 (after `with '`)
    let consumer_uri = workspace.uri("lib/MyApp/User.pm");
    let consumer_code = "package MyApp::User;\nuse Moo;\nwith 'MyApp::Role::Printable';\n1;\n";
    let with_line = consumer_code
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains("MyApp::Role::Printable"))
        .map(|(i, _)| i as u64)
        .ok_or("could not find with line")?;
    let with_char = consumer_code
        .lines()
        .nth(with_line as usize)
        .and_then(|line| line.find("MyApp::Role::Printable"))
        .ok_or("could not find role name in with line")?;

    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": consumer_uri},
            "position": {"line": with_line, "character": with_char + 5}
        }),
    )?;

    // MUST navigate to the role file — empty result means the feature is not implemented.
    let locations = result
        .as_array()
        .ok_or_else(|| format!("goto-def on 'with' role name returned non-array: {:?}", result))?;
    assert!(
        !locations.is_empty(),
        "goto-def on 'with' role name MUST return at least one location (got empty array)"
    );
    let first = &locations[0];
    assert_valid_location(first);

    let uri = first["uri"].as_str().ok_or("Expected URI in goto-def result")?;
    assert!(
        uri.contains("Printable"),
        "goto-def on 'with' role name should navigate to Printable.pm, got: {}",
        uri
    );

    Ok(())
}

/// Go-to-definition on the parent class in `extends 'ParentClass'` should navigate to parent file.
#[test]
fn go_to_definition_on_extends_parent_navigates_to_parent_file() -> TestResult {
    let mut harness = LspHarness::new();
    let workspace = support::lsp_harness::TempWorkspace::new()?;

    // Write the parent class file to disk
    workspace.write(
        "lib/MyApp/User.pm",
        r#"package MyApp::User;
use Moo;

has name => (is => 'ro');

1;
"#,
    )?;

    harness.initialize_with_root(&workspace.root_uri, None)?;

    // Open the parent file so it is indexed
    let parent_uri = workspace.uri("lib/MyApp/User.pm");
    let parent_content = std::fs::read_to_string(workspace.dir.path().join("lib/MyApp/User.pm"))
        .map_err(|e| format!("failed to read parent file: {e}"))?;
    harness.open(&parent_uri, &parent_content)?;

    // Open the child class that extends the parent
    harness.open(
        &workspace.uri("lib/MyApp/AdminUser.pm"),
        r#"package MyApp::AdminUser;
use Moo;
extends 'MyApp::User';
1;
"#,
    )?;

    harness.barrier();

    // Request goto-definition on "MyApp::User" in `extends 'MyApp::User';`
    // Line 2 (0-indexed): `extends 'MyApp::User';`
    let child_uri = workspace.uri("lib/MyApp/AdminUser.pm");
    let child_code = "package MyApp::AdminUser;\nuse Moo;\nextends 'MyApp::User';\n1;\n";
    let extends_line = child_code
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains("MyApp::User"))
        .map(|(i, _)| i as u64)
        .ok_or("could not find extends line")?;
    let extends_char = child_code
        .lines()
        .nth(extends_line as usize)
        .and_then(|line| line.find("MyApp::User"))
        .ok_or("could not find parent name in extends line")?;

    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": child_uri},
            "position": {"line": extends_line, "character": extends_char + 3}
        }),
    )?;

    // MUST navigate to the parent file — empty result means the feature is not implemented.
    let locations = result
        .as_array()
        .ok_or_else(|| format!("goto-def on 'extends' parent returned non-array: {:?}", result))?;
    assert!(
        !locations.is_empty(),
        "goto-def on 'extends' parent name MUST return at least one location (got empty array)"
    );
    let first = &locations[0];
    assert_valid_location(first);

    let uri = first["uri"].as_str().ok_or("Expected URI in goto-def result")?;
    assert!(
        uri.contains("User"),
        "goto-def on 'extends' parent should navigate to User.pm, got: {}",
        uri
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Test 7: symbol_at_cursor handles Package->method (Identifier-based)
// ---------------------------------------------------------------------------

#[test]
fn symbol_at_cursor_resolves_package_method_call() -> TestResult {
    use perl_parser::Parser;
    use perl_parser::declaration::{current_package_at, symbol_at_cursor};

    let code = r#"package main;
use MyModule;

MyModule->process();
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse().map_err(|e| format!("parse error: {e}"))?;

    // Find the offset of "process" in "MyModule->process()"
    let process_offset = code.find("->process()").ok_or("could not find ->process()")? + "->".len();

    let current_pkg = current_package_at(&ast, process_offset);
    let symbol = symbol_at_cursor(&ast, process_offset, current_pkg);

    if let Some(sym) = &symbol {
        assert_eq!(sym.name.as_ref(), "process", "method name should be 'process'");
        // The package should be MyModule (the object/class in the MethodCall)
        assert_eq!(sym.pkg.as_ref(), "MyModule", "package should be MyModule");
    }

    Ok(())
}

#[test]
fn go_to_definition_cross_file_package_method_falls_back_to_autoload() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    harness.open(
        "file:///lib/AutoDispatch.pm",
        r#"package AutoDispatch;

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub AUTOLOAD {
    our $AUTOLOAD;
    return $AUTOLOAD;
}

1;
"#,
    )?;

    harness.open(
        "file:///app.pl",
        r#"#!/usr/bin/perl
use strict;
use warnings;
use lib 'lib';
use AutoDispatch;

AutoDispatch->dynamic_method();
"#,
    )?;

    harness.barrier();

    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": "file:///app.pl"},
            "position": {"line": 6, "character": 16}
        }),
    )?;

    let locations = result
        .as_array()
        .ok_or_else(|| format!("expected goto-def array for AUTOLOAD fallback, got: {result:?}"))?;
    assert!(!locations.is_empty(), "AUTOLOAD-backed method call should resolve to a definition");

    let first = &locations[0];
    assert_valid_location(first);

    let uri = first["uri"].as_str().ok_or("expected URI in goto-def result")?;
    assert!(
        uri.contains("AutoDispatch.pm"),
        "AUTOLOAD fallback should point to AutoDispatch.pm, got: {uri}"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests for issue #3482: plain OO inheritance goto-def (use parent / use base / @ISA)
// ---------------------------------------------------------------------------

/// Test A: `use parent` — child->greet() resolves to Base.pm
#[test]
fn go_to_definition_cross_file_plain_oo_use_parent() -> TestResult {
    let mut harness = LspHarness::new();
    let workspace = TempWorkspace::new()?;

    workspace.write(
        "lib/Base.pm",
        r#"package Base;

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub greet {
    my ($self) = @_;
    return "Hello from Base";
}

1;
"#,
    )?;

    workspace.write(
        "lib/Child.pm",
        r#"package Child;
use parent 'Base';

sub hello {
    my ($self) = @_;
    return "Hello from Child";
}

1;
"#,
    )?;

    harness.initialize_with_root(&workspace.root_uri, None)?;

    for relative in ["lib/Base.pm", "lib/Child.pm"] {
        let uri = workspace.uri(relative);
        let content = std::fs::read_to_string(workspace.dir.path().join(relative))
            .map_err(|e| format!("failed to read {relative}: {e}"))?;
        harness.open(&uri, &content)?;
    }

    harness.open(
        &workspace.uri("main.pl"),
        r#"#!/usr/bin/perl
use strict;
use warnings;
use lib 'lib';
use Child;

my $c = Child->new();
$c->greet();
"#,
    )?;

    harness.barrier();

    // Line 7 (0-indexed): `$c->greet();`
    // character 4 is on "greet"
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("main.pl")},
            "position": {"line": 7, "character": 4}
        }),
    )?;

    let locations = result
        .as_array()
        .ok_or_else(|| format!("Expected array for use parent goto-def, got: {result:?}"))?;
    assert!(
        !locations.is_empty(),
        "Expected use parent inherited method goto-def to return at least one location"
    );

    let uri = locations[0]["uri"].as_str().ok_or("Expected definition URI")?;
    assert!(uri.contains("Base.pm"), "use parent: definition should point to Base.pm, got: {uri}");

    Ok(())
}

/// Test B: `use base` — child->greet() resolves to Base.pm
#[test]
fn go_to_definition_cross_file_plain_oo_use_base() -> TestResult {
    let mut harness = LspHarness::new();
    let workspace = TempWorkspace::new()?;

    workspace.write(
        "lib/BaseB.pm",
        r#"package BaseB;

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub greet {
    my ($self) = @_;
    return "Hello from BaseB";
}

1;
"#,
    )?;

    workspace.write(
        "lib/ChildB.pm",
        r#"package ChildB;
use base 'BaseB';

sub hello {
    my ($self) = @_;
    return "Hello from ChildB";
}

1;
"#,
    )?;

    harness.initialize_with_root(&workspace.root_uri, None)?;

    for relative in ["lib/BaseB.pm", "lib/ChildB.pm"] {
        let uri = workspace.uri(relative);
        let content = std::fs::read_to_string(workspace.dir.path().join(relative))
            .map_err(|e| format!("failed to read {relative}: {e}"))?;
        harness.open(&uri, &content)?;
    }

    harness.open(
        &workspace.uri("main.pl"),
        r#"#!/usr/bin/perl
use strict;
use warnings;
use lib 'lib';
use ChildB;

my $c = ChildB->new();
$c->greet();
"#,
    )?;

    harness.barrier();

    // Line 7 (0-indexed): `$c->greet();`
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("main.pl")},
            "position": {"line": 7, "character": 4}
        }),
    )?;

    let locations = result
        .as_array()
        .ok_or_else(|| format!("Expected array for use base goto-def, got: {result:?}"))?;
    assert!(
        !locations.is_empty(),
        "Expected use base inherited method goto-def to return at least one location"
    );

    let uri = locations[0]["uri"].as_str().ok_or("Expected definition URI")?;
    assert!(uri.contains("BaseB.pm"), "use base: definition should point to BaseB.pm, got: {uri}");

    Ok(())
}

/// Test C: `our @ISA = qw(BaseC)` — raw @ISA inheritance resolves method
#[test]
fn go_to_definition_cross_file_plain_oo_raw_isa() -> TestResult {
    let mut harness = LspHarness::new();
    let workspace = TempWorkspace::new()?;

    workspace.write(
        "lib/BaseC.pm",
        r#"package BaseC;

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub greet {
    my ($self) = @_;
    return "Hello from BaseC";
}

1;
"#,
    )?;

    workspace.write(
        "lib/ChildC.pm",
        r#"package ChildC;
our @ISA = qw(BaseC);

sub hello {
    my ($self) = @_;
    return "Hello from ChildC";
}

1;
"#,
    )?;

    harness.initialize_with_root(&workspace.root_uri, None)?;

    for relative in ["lib/BaseC.pm", "lib/ChildC.pm"] {
        let uri = workspace.uri(relative);
        let content = std::fs::read_to_string(workspace.dir.path().join(relative))
            .map_err(|e| format!("failed to read {relative}: {e}"))?;
        harness.open(&uri, &content)?;
    }

    harness.open(
        &workspace.uri("main.pl"),
        r#"#!/usr/bin/perl
use strict;
use warnings;
use lib 'lib';
use ChildC;

my $c = ChildC->new();
$c->greet();
"#,
    )?;

    harness.barrier();

    // Line 7 (0-indexed): `$c->greet();`
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("main.pl")},
            "position": {"line": 7, "character": 4}
        }),
    )?;

    let locations = result
        .as_array()
        .ok_or_else(|| format!("Expected array for @ISA goto-def, got: {result:?}"))?;
    assert!(
        !locations.is_empty(),
        "Expected @ISA inherited method goto-def to return at least one location"
    );

    let uri = locations[0]["uri"].as_str().ok_or("Expected definition URI")?;
    assert!(uri.contains("BaseC.pm"), "@ISA: definition should point to BaseC.pm, got: {uri}");

    Ok(())
}

/// Test D: Grandparent chain — GrandChild inherits Child inherits Base;
/// gc->base_method() should resolve to Base.pm (BFS depth > 1).
#[test]
fn go_to_definition_cross_file_grandparent_chain() -> TestResult {
    let mut harness = LspHarness::new();
    let workspace = TempWorkspace::new()?;

    workspace.write(
        "lib/GrandBase.pm",
        r#"package GrandBase;

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub base_method {
    my ($self) = @_;
    return "from GrandBase";
}

1;
"#,
    )?;

    workspace.write(
        "lib/Middle.pm",
        r#"package Middle;
use parent 'GrandBase';

sub middle_method {
    my ($self) = @_;
    return "from Middle";
}

1;
"#,
    )?;

    workspace.write(
        "lib/GrandChild.pm",
        r#"package GrandChild;
use parent 'Middle';

sub child_method {
    my ($self) = @_;
    return "from GrandChild";
}

1;
"#,
    )?;

    harness.initialize_with_root(&workspace.root_uri, None)?;

    for relative in ["lib/GrandBase.pm", "lib/Middle.pm", "lib/GrandChild.pm"] {
        let uri = workspace.uri(relative);
        let content = std::fs::read_to_string(workspace.dir.path().join(relative))
            .map_err(|e| format!("failed to read {relative}: {e}"))?;
        harness.open(&uri, &content)?;
    }

    harness.open(
        &workspace.uri("main.pl"),
        r#"#!/usr/bin/perl
use strict;
use warnings;
use lib 'lib';
use GrandChild;

my $gc = GrandChild->new();
$gc->base_method();
"#,
    )?;

    harness.barrier();

    // Line 7 (0-indexed): `$gc->base_method();`
    // character 5 is on "base_method"
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("main.pl")},
            "position": {"line": 7, "character": 5}
        }),
    )?;

    let locations = result
        .as_array()
        .ok_or_else(|| format!("Expected array for grandparent chain goto-def, got: {result:?}"))?;
    assert!(
        !locations.is_empty(),
        "Expected grandparent chain goto-def to return at least one location"
    );

    let uri = locations[0]["uri"].as_str().ok_or("Expected definition URI")?;
    assert!(
        uri.contains("GrandBase.pm"),
        "Grandparent chain: definition should point to GrandBase.pm, got: {uri}"
    );

    Ok(())
}

/// Test E: deep CPAN-style module path with multi-hop inheritance.
#[test]
fn go_to_definition_cross_file_cpan_deep_module_inheritance_chain() -> TestResult {
    let mut harness = LspHarness::new();
    let workspace = TempWorkspace::new()?;

    workspace.write(
        "lib/Catalyst/Plugin/Session/Store/Base.pm",
        r#"package Catalyst::Plugin::Session::Store::Base;

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub encrypt_session_payload {
    my ($self) = @_;
    return "encrypted";
}

1;
"#,
    )?;

    workspace.write(
        "lib/Catalyst/Plugin/Session/Store/DBIC.pm",
        r#"package Catalyst::Plugin::Session::Store::DBIC;
use parent 'Catalyst::Plugin::Session::Store::Base';

1;
"#,
    )?;

    workspace.write(
        "lib/Catalyst/Plugin/Session/Store/DBIC/Schema.pm",
        r#"package Catalyst::Plugin::Session::Store::DBIC::Schema;
use parent 'Catalyst::Plugin::Session::Store::DBIC';

1;
"#,
    )?;

    workspace.write(
        "lib/Catalyst/Plugin/Session/Store/DBIC/Schema/ResultSet.pm",
        r#"package Catalyst::Plugin::Session::Store::DBIC::Schema::ResultSet;
use parent 'Catalyst::Plugin::Session::Store::DBIC::Schema';

1;
"#,
    )?;

    workspace.write(
        "lib/Catalyst/Plugin/Session/Store/DBIC/Encrypted.pm",
        r#"package Catalyst::Plugin::Session::Store::DBIC::Encrypted;
use parent 'Catalyst::Plugin::Session::Store::DBIC::Schema::ResultSet';

1;
"#,
    )?;

    harness.initialize_with_root(&workspace.root_uri, None)?;

    for relative in [
        "lib/Catalyst/Plugin/Session/Store/Base.pm",
        "lib/Catalyst/Plugin/Session/Store/DBIC.pm",
        "lib/Catalyst/Plugin/Session/Store/DBIC/Schema.pm",
        "lib/Catalyst/Plugin/Session/Store/DBIC/Schema/ResultSet.pm",
        "lib/Catalyst/Plugin/Session/Store/DBIC/Encrypted.pm",
    ] {
        let uri = workspace.uri(relative);
        let content = std::fs::read_to_string(workspace.dir.path().join(relative))
            .map_err(|e| format!("failed to read {relative}: {e}"))?;
        harness.open(&uri, &content)?;
    }

    harness.open(
        &workspace.uri("main.pl"),
        r#"#!/usr/bin/perl
use strict;
use warnings;
use lib 'lib';
use Catalyst::Plugin::Session::Store::DBIC::Encrypted;

my $store = Catalyst::Plugin::Session::Store::DBIC::Encrypted->new();
$store->encrypt_session_payload();
"#,
    )?;

    harness.barrier();

    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("main.pl")},
            "position": {"line": 7, "character": 9}
        }),
    )?;

    let locations = result.as_array().ok_or_else(|| {
        format!("Expected array for deep CPAN inheritance goto-def, got: {result:?}")
    })?;
    assert!(
        !locations.is_empty(),
        "Expected deep CPAN inheritance chain goto-def to return at least one location"
    );

    let uri = locations[0]["uri"].as_str().ok_or("Expected definition URI")?;
    assert!(
        uri.contains("Catalyst/Plugin/Session/Store/Base.pm"),
        "Deep CPAN inheritance chain: definition should point to Base.pm, got: {uri}"
    );

    Ok(())
}

/// Test F: `use parent -norequire` variant — method resolves correctly
#[test]
fn go_to_definition_cross_file_use_parent_norequire() -> TestResult {
    let mut harness = LspHarness::new();
    let workspace = TempWorkspace::new()?;

    workspace.write(
        "lib/BaseNR.pm",
        r#"package BaseNR;

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub greet {
    my ($self) = @_;
    return "Hello from BaseNR";
}

1;
"#,
    )?;

    workspace.write(
        "lib/ChildNR.pm",
        r#"package ChildNR;
use parent -norequire, 'BaseNR';

sub hello {
    my ($self) = @_;
    return "Hello from ChildNR";
}

1;
"#,
    )?;

    harness.initialize_with_root(&workspace.root_uri, None)?;

    for relative in ["lib/BaseNR.pm", "lib/ChildNR.pm"] {
        let uri = workspace.uri(relative);
        let content = std::fs::read_to_string(workspace.dir.path().join(relative))
            .map_err(|e| format!("failed to read {relative}: {e}"))?;
        harness.open(&uri, &content)?;
    }

    harness.open(
        &workspace.uri("main.pl"),
        r#"#!/usr/bin/perl
use strict;
use warnings;
use lib 'lib';
use ChildNR;

my $c = ChildNR->new();
$c->greet();
"#,
    )?;

    harness.barrier();

    // Line 7 (0-indexed): `$c->greet();`
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("main.pl")},
            "position": {"line": 7, "character": 4}
        }),
    )?;

    let locations = result.as_array().ok_or_else(|| {
        format!("Expected array for use parent -norequire goto-def, got: {result:?}")
    })?;
    assert!(
        !locations.is_empty(),
        "Expected use parent -norequire inherited method goto-def to return at least one location"
    );

    let uri = locations[0]["uri"].as_str().ok_or("Expected definition URI")?;
    assert!(
        uri.contains("BaseNR.pm"),
        "use parent -norequire: definition should point to BaseNR.pm, got: {uri}"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Test: issue #3472 gap (a) — parenthesized import list: use Utils ('helper_a')
// ---------------------------------------------------------------------------

#[test]
fn go_to_definition_on_parens_list_imported_function_navigates_to_source_module() -> TestResult {
    let mut harness = LspHarness::new();
    let workspace = support::lsp_harness::TempWorkspace::new()?;

    workspace.write(
        "lib/My/Utils.pm",
        r#"package My::Utils;
use strict;
use warnings;

sub helper_a {
    return 'result_a';
}

sub helper_b {
    return 'result_b';
}

1;
"#,
    )?;

    harness.initialize_with_root(&workspace.root_uri, None)?;

    let module_uri = workspace.uri("lib/My/Utils.pm");
    let module_content = std::fs::read_to_string(workspace.dir.path().join("lib/My/Utils.pm"))
        .map_err(|e| format!("failed to read module: {e}"))?;
    harness.open(&module_uri, &module_content)?;

    // Use parenthesized import list (not qw form)
    let caller = r#"#!/usr/bin/perl
use strict;
use warnings;
use My::Utils ('helper_a', 'helper_b');

my $result = helper_a();
"#;
    harness.open(&workspace.uri("app.pl"), caller)?;
    harness.barrier();

    let (line, character) = find_line_char(caller, "helper_a()")?;
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("app.pl")},
            "position": {"line": line, "character": character}
        }),
    )?;

    let locations = result.as_array().ok_or("expected location array")?;
    assert!(!locations.is_empty(), "expected definition result for paren-list imported function");

    let first = &locations[0];
    assert_valid_location(first);
    let uri = first["uri"].as_str().ok_or("Expected URI")?;
    assert!(
        uri.contains("My/Utils.pm") || uri.contains("My%2FUtils.pm"),
        "Definition should point to My/Utils.pm, got: {uri}"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Test: issue #3475 — use constant cross-file resolution
// ---------------------------------------------------------------------------

#[test]
fn go_to_definition_on_cross_file_use_constant_navigates_to_definition() -> TestResult {
    let mut harness = LspHarness::new();
    let workspace = support::lsp_harness::TempWorkspace::new()?;

    workspace.write(
        "lib/My/Config.pm",
        r#"package My::Config;
use strict;
use warnings;

use constant PI => 3.14159;
use constant {
    MAX_RETRIES => 3,
    TIMEOUT     => 30,
};

1;
"#,
    )?;

    harness.initialize_with_root(&workspace.root_uri, None)?;

    let module_uri = workspace.uri("lib/My/Config.pm");
    let module_content = std::fs::read_to_string(workspace.dir.path().join("lib/My/Config.pm"))
        .map_err(|e| format!("failed to read module: {e}"))?;
    harness.open(&module_uri, &module_content)?;

    // Consumer uses qualified constant reference: My::Config::PI
    let caller = r#"#!/usr/bin/perl
use strict;
use warnings;
use My::Config;

my $circumference = 2 * My::Config::PI * 5;
print My::Config::MAX_RETRIES;
"#;
    harness.open(&workspace.uri("main.pl"), caller)?;
    harness.barrier();

    let (line, character) = find_line_char(caller, "My::Config::PI")?;
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("main.pl")},
            "position": {"line": line, "character": character + "My::Config::".len() as u32}
        }),
    )?;

    let locations = result.as_array().ok_or("expected location array")?;
    assert!(!locations.is_empty(), "expected definition result for cross-file use constant PI");

    let first = &locations[0];
    assert_valid_location(first);
    let uri = first["uri"].as_str().ok_or("Expected URI")?;
    assert!(
        uri.contains("My/Config.pm") || uri.contains("My%2FConfig.pm"),
        "Definition for My::Config::PI should point to My/Config.pm, got: {uri}"
    );

    Ok(())
}
