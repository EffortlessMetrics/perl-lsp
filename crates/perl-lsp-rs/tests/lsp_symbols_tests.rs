//! Tests for textDocument/documentSymbol LSP feature
//!
//! Validates the document symbol provider functionality including:
//! - Symbols in files with subroutines
//! - Symbols with package declarations
//! - Empty file handling
//! - Capability advertisement in server initialization
//! - Symbol kind correctness (Function, Variable, Package)

mod support;
use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Test that a file with subroutines returns Function symbols
#[test]
fn test_document_symbol_subroutines() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_symbols.pl";
    harness.open(
        doc_uri,
        r#"
sub greet {
    my $name = shift;
    print "Hello, $name!\n";
}

sub farewell {
    my $name = shift;
    print "Goodbye, $name!\n";
}
"#,
    )?;

    let response = harness.request(
        "textDocument/documentSymbol",
        json!({
            "textDocument": { "uri": doc_uri }
        }),
    )?;

    assert!(response.is_array(), "documentSymbol should return an array, got: {:?}", response);

    let symbols = response.as_array().ok_or("response is not an array")?;
    assert!(!symbols.is_empty(), "Should return at least one symbol");

    // Look for the greet and farewell subroutines
    let greet = symbols.iter().find(|s| s["name"].as_str() == Some("greet"));
    assert!(greet.is_some(), "Should find 'greet' symbol");
    let greet = greet.ok_or("greet symbol not found")?;
    // Function kind = 12
    assert_eq!(greet["kind"], 12, "greet should have kind 12 (Function)");
    // Must have range and selectionRange
    assert!(greet["range"].is_object(), "greet should have a range object");
    assert!(
        greet["selectionRange"].is_object() || greet["range"].is_object(),
        "greet should have selectionRange or range"
    );

    let farewell = symbols.iter().find(|s| s["name"].as_str() == Some("farewell"));
    assert!(farewell.is_some(), "Should find 'farewell' symbol");
    let farewell = farewell.ok_or("farewell symbol not found")?;
    assert_eq!(farewell["kind"], 12, "farewell should have kind 12 (Function)");

    Ok(())
}

/// Test that package declarations appear as symbols with correct kind
#[test]
fn test_document_symbol_packages() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_packages.pl";
    harness.open(
        doc_uri,
        r#"
package MyApp::Util;

use strict;
use warnings;

my $VERSION = '1.0';

sub helper {
    return 1;
}

package MyApp::Config;

sub load {
    return {};
}

1;
"#,
    )?;

    let response = harness.request(
        "textDocument/documentSymbol",
        json!({
            "textDocument": { "uri": doc_uri }
        }),
    )?;

    let symbols = response.as_array().ok_or("response is not an array")?;

    // Check for MyApp::Util package
    let util_pkg = symbols.iter().find(|s| {
        let name = s["name"].as_str().unwrap_or("");
        name == "MyApp::Util" || name == "MyApp/Util" || name.contains("Util")
    });
    assert!(
        util_pkg.is_some(),
        "Should find MyApp::Util package symbol. Symbols: {:?}",
        symbols.iter().map(|s| s["name"].as_str().unwrap_or("?")).collect::<Vec<_>>()
    );
    let util_pkg = util_pkg.ok_or("MyApp::Util not found")?;
    // Package kind is 4 or Module kind is 2
    let kind = util_pkg["kind"].as_i64().unwrap_or(0);
    assert!(
        kind == 2 || kind == 4,
        "Package should have kind 2 (Module) or 4 (Package), got {}",
        kind
    );

    // Check for MyApp::Config package
    let config_pkg = symbols.iter().find(|s| {
        let name = s["name"].as_str().unwrap_or("");
        name == "MyApp::Config" || name == "MyApp/Config" || name.contains("Config")
    });
    assert!(config_pkg.is_some(), "Should find MyApp::Config package symbol");

    // Check for subroutines
    let helper = symbols.iter().find(|s| s["name"].as_str() == Some("helper"));
    assert!(helper.is_some(), "Should find 'helper' sub symbol");

    let load = symbols.iter().find(|s| s["name"].as_str() == Some("load"));
    assert!(load.is_some(), "Should find 'load' sub symbol");

    Ok(())
}

/// Test that an empty file returns an empty symbol array
#[test]
fn test_document_symbol_empty_file() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///empty.pl";
    harness.open(doc_uri, "")?;

    let response = harness.request(
        "textDocument/documentSymbol",
        json!({
            "textDocument": { "uri": doc_uri }
        }),
    )?;

    assert!(response.is_array(), "documentSymbol should return an array for empty file");
    let symbols = response.as_array().ok_or("response is not an array")?;
    assert!(symbols.is_empty(), "Empty file should yield no symbols");

    Ok(())
}

/// Test that documentSymbolProvider capability is advertised during initialization
#[test]
fn test_document_symbol_capability_advertised() -> TestResult {
    let mut harness = LspHarness::new();
    let init_response = harness.initialize(None)?;

    let capabilities = &init_response["capabilities"];
    let has_symbol_provider = capabilities.get("documentSymbolProvider").is_some();
    assert!(
        has_symbol_provider,
        "Server should advertise documentSymbolProvider capability. Capabilities: {:?}",
        capabilities
    );

    Ok(())
}

/// Test that variable declarations appear as symbols with correct kinds
#[test]
fn test_document_symbol_variables() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_vars.pl";
    harness.open(
        doc_uri,
        r#"
my $scalar_var = 42;
my @array_var = (1, 2, 3);
my %hash_var = (key => 'value');
our $shared = "global";

sub compute {
    my $local = 10;
    return $local + $scalar_var;
}
"#,
    )?;

    let response = harness.request(
        "textDocument/documentSymbol",
        json!({
            "textDocument": { "uri": doc_uri }
        }),
    )?;

    let symbols = response.as_array().ok_or("response is not an array")?;

    // Check for scalar variable (kind 13 = Variable)
    let scalar = symbols.iter().find(|s| s["name"].as_str() == Some("$scalar_var"));
    assert!(scalar.is_some(), "Should find $scalar_var symbol");
    if let Some(sv) = scalar {
        assert_eq!(sv["kind"], 13, "$scalar_var should have kind 13 (Variable)");
    }

    // Check for array variable (kind 18 = Array)
    let array = symbols.iter().find(|s| s["name"].as_str() == Some("@array_var"));
    assert!(array.is_some(), "Should find @array_var symbol");
    if let Some(av) = array {
        assert_eq!(av["kind"], 18, "@array_var should have kind 18 (Array)");
    }

    // Check for hash variable (kind 19 = Object, closest to hash)
    let hash = symbols.iter().find(|s| s["name"].as_str() == Some("%hash_var"));
    assert!(hash.is_some(), "Should find %hash_var symbol");
    if let Some(hv) = hash {
        assert_eq!(hv["kind"], 19, "%hash_var should have kind 19 (Object)");
    }

    // Check for the compute subroutine
    let compute = symbols.iter().find(|s| s["name"].as_str() == Some("compute"));
    assert!(compute.is_some(), "Should find 'compute' sub symbol");

    Ok(())
}

/// Test that `our $VAR` package variables appear as document symbols with kind 13 (Variable).
///
/// The SymbolExtractor has always handled `our` declarators; this test pins that
/// `our $VERSION` is emitted with the correct sigil-prefixed name and Variable kind.
#[test]
fn test_document_symbol_our_variable() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_our_vars.pl";
    harness.open(
        doc_uri,
        r#"package Acme::Widget;

our $VERSION = '1.00';
our @EXPORT = ('new');
our %CONFIG = (debug => 0);

sub new {
    my $class = shift;
    return bless {}, $class;
}
"#,
    )?;

    let response = harness.request(
        "textDocument/documentSymbol",
        json!({
            "textDocument": { "uri": doc_uri }
        }),
    )?;

    let symbols = response.as_array().ok_or("documentSymbol should return an array")?;

    // Collect all symbol names for diagnostics
    let names: Vec<&str> = symbols.iter().filter_map(|s| s["name"].as_str()).collect();

    // $VERSION should appear as a Variable (kind 13)
    let version_sym = symbols.iter().find(|s| s["name"].as_str() == Some("$VERSION"));
    assert!(
        version_sym.is_some(),
        "should find '$VERSION' document symbol; got names: {:?}",
        names
    );
    if let Some(v) = version_sym {
        assert_eq!(v["kind"], 13, "$VERSION should have kind 13 (Variable)");
    }

    // @EXPORT should appear as an Array (kind 18)
    let export_sym = symbols.iter().find(|s| s["name"].as_str() == Some("@EXPORT"));
    assert!(export_sym.is_some(), "should find '@EXPORT' document symbol; got names: {:?}", names);
    if let Some(e) = export_sym {
        assert_eq!(e["kind"], 18, "@EXPORT should have kind 18 (Array)");
    }

    Ok(())
}

/// Test that Moo `has 'attr'` declarations appear as document symbols with kind 7 (Property).
///
/// The SymbolExtractor synthesizes a Symbol with declaration="has" and kind=Variable(Scalar)
/// for each Moo/Moose attribute.  The provider maps declaration="has" to LSP kind 7 (Property)
/// and drops the sigil from the displayed name.
#[test]
fn test_document_symbol_moo_has_attributes() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///moo_has_test.pl";
    harness.open(
        doc_uri,
        r#"package Demo::User;
use Moo;
has 'name' => (is => 'ro', isa => 'Str', default => sub { 'anon' });

sub greet {
    my $self = shift;
    return $self->name;
}
"#,
    )?;

    let response = harness.request(
        "textDocument/documentSymbol",
        json!({
            "textDocument": { "uri": doc_uri }
        }),
    )?;

    let symbols = response.as_array().ok_or("documentSymbol should return an array")?;
    let names: Vec<&str> = symbols.iter().filter_map(|s| s["name"].as_str()).collect();

    // `has 'name'` should produce a symbol named "name" (no sigil) with kind 7 (Property)
    let name_sym = symbols.iter().find(|s| s["name"].as_str() == Some("name"));
    assert!(
        name_sym.is_some(),
        "should find 'name' attribute as document symbol (kind 7); got names: {:?}",
        names
    );
    if let Some(ns) = name_sym {
        assert_eq!(
            ns["kind"], 7,
            "'name' Moo attribute should have LSP kind 7 (Property); got: {:?}",
            ns["kind"]
        );
    }

    Ok(())
}

/// Test that Moose `has 'attr'` declarations appear as document symbols with kind 7 (Property).
#[test]
fn test_document_symbol_moose_has_attributes() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///moose_has_test.pl";
    harness.open(
        doc_uri,
        r#"package Demo::MooseUser;
use Moose;
has 'email' => (
    is => 'rw',
    isa => 'Str',
    default => sub { 'user@example.com' },
);

sub run {
    my $self = shift;
    return $self->email;
}
"#,
    )?;

    let response = harness.request(
        "textDocument/documentSymbol",
        json!({
            "textDocument": { "uri": doc_uri }
        }),
    )?;

    let symbols = response.as_array().ok_or("documentSymbol should return an array")?;
    let names: Vec<&str> = symbols.iter().filter_map(|s| s["name"].as_str()).collect();

    // `has 'email'` should produce a symbol named "email" with kind 7 (Property)
    let email_sym = symbols.iter().find(|s| s["name"].as_str() == Some("email"));
    assert!(
        email_sym.is_some(),
        "should find 'email' Moose attribute as document symbol (kind 7); got names: {:?}",
        names
    );
    if let Some(es) = email_sym {
        assert_eq!(
            es["kind"], 7,
            "'email' Moose attribute should have LSP kind 7 (Property); got: {:?}",
            es["kind"]
        );
    }

    Ok(())
}

/// Test document symbols for a file with mixed content including comments
#[test]
fn test_document_symbol_mixed_content() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_mixed.pl";
    harness.open(
        doc_uri,
        r#"#!/usr/bin/perl
use strict;
use warnings;

# This is a utility module
package Util;

our $VERSION = '0.01';

# Add two numbers
sub add {
    my ($a, $b) = @_;
    return $a + $b;
}

# Multiply two numbers
sub multiply {
    my ($a, $b) = @_;
    return $a * $b;
}

1;
"#,
    )?;

    let response = harness.request(
        "textDocument/documentSymbol",
        json!({
            "textDocument": { "uri": doc_uri }
        }),
    )?;

    let symbols = response.as_array().ok_or("response is not an array")?;

    // Should have at least the package and two subs
    assert!(
        symbols.len() >= 3,
        "Should have at least 3 symbols (package + 2 subs), got {}",
        symbols.len()
    );

    // Verify the two subroutines are present
    let add = symbols.iter().find(|s| s["name"].as_str() == Some("add"));
    assert!(add.is_some(), "Should find 'add' symbol");

    let multiply = symbols.iter().find(|s| s["name"].as_str() == Some("multiply"));
    assert!(multiply.is_some(), "Should find 'multiply' symbol");

    // Verify each symbol has a valid range
    for symbol in symbols {
        if symbol["range"].is_object() {
            assert!(
                symbol["range"]["start"]["line"].is_number(),
                "Symbol range start line should be a number"
            );
            assert!(
                symbol["range"]["start"]["character"].is_number(),
                "Symbol range start character should be a number"
            );
        }
    }

    Ok(())
}
