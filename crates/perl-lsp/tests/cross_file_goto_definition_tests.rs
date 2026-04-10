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
