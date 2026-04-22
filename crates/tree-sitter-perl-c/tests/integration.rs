//! Integration tests for tree-sitter-perl-c crate.
//!
//! These tests exercise the seams between components and the full CLI flow:
//! - CLI binary end-to-end parsing
//! - Parser reuse across multiple parse calls
//! - File-based parse pipeline (write → parse → verify)
//! - Language/query interoperability
//! - Error propagation through the parsing pipeline

use std::fs;
use std::path::PathBuf;

use tree_sitter::Query;
use tree_sitter_perl_c::{
    create_parser, language, parse_perl_code, parse_perl_file, try_create_parser,
};

mod common;
use common::temp_perl_file;

// ---------------------------------------------------------------------------
// CLI Binary Integration Tests
// ---------------------------------------------------------------------------

/// Integration: CLI binary parses a valid file and exits successfully.
#[test]
fn integration_cli_parses_valid_file() -> Result<(), Box<dyn std::error::Error>> {
    let path = temp_perl_file("valid", "my $x = 42;\nprint $x;\n");

    // Run the parse_c binary using cargo to ensure we get the right one
    let output = std::process::Command::new("cargo")
        .args(["run", "-p", "tree-sitter-perl-c", "--bin", "parse_c", "--"])
        .arg(&path)
        .output()?;

    // Should succeed (exit code 0)
    assert!(
        output.status.success(),
        "parse_c should exit successfully for valid Perl: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    fs::remove_file(&path).ok();
    Ok(())
}

/// Integration: CLI binary exits with error for invalid file path.
#[test]
fn integration_cli_rejects_missing_file() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::process::Command::new("cargo")
        .args(["run", "-p", "tree-sitter-perl-c", "--bin", "parse_c", "--"])
        .arg("/nonexistent/path/to/file.pl")
        .output()?;

    // Should fail (non-zero exit code)
    assert!(!output.status.success(), "parse_c should fail for missing file");

    // Should have an error message
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Error reading file"),
        "parse_c should report file read error: {}",
        stderr
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Parser Reuse Integration Tests
// ---------------------------------------------------------------------------

/// Integration: A single parser instance can parse multiple files sequentially.
/// This tests parser reuse and confirms parsers are safe to reuse.
#[test]
fn integration_parser_reuse_multiple_parses() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = try_create_parser()?;

    let sources = [
        "my $x = 1;",
        "package Foo;",
        "sub bar { return 42; }",
        "for my $i (0..10) { say $i; }",
        r#"my $re = qr/\d+/;"#,
        "package Bar;\nuse strict;\nuse warnings;\n",
    ];

    for source in sources {
        let tree = parser.parse(source, None).ok_or("parse should succeed")?;
        assert!(
            !tree.root_node().has_error(),
            "Parsed tree should not have errors for: {}",
            source
        );
    }

    Ok(())
}

/// Integration: Parsing two related files (same program, separate files) produces
/// consistent results with parser reuse.
#[test]
fn integration_parser_reuse_package_across_files() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = try_create_parser()?;

    // Parse a package declaration
    let file1_content = "package MyApp;\nour $VERSION = '1.0';\n";
    let tree1 = parser.parse(file1_content, None).ok_or("parse should succeed")?;

    // Parse a file that "uses" the first package
    let file2_content = "use MyApp;\nprint $MyApp::VERSION;\n";
    let tree2 = parser.parse(file2_content, None).ok_or("parse should succeed")?;

    // Both should parse without errors
    assert!(!tree1.root_node().has_error(), "file1 should parse cleanly");
    assert!(!tree2.root_node().has_error(), "file2 should parse cleanly");

    // Root node should be source_file for both
    assert_eq!(tree1.root_node().kind(), "source_file");
    assert_eq!(tree2.root_node().kind(), "source_file");

    Ok(())
}

// ---------------------------------------------------------------------------
// File-Based Parse Pipeline Integration Tests
// ---------------------------------------------------------------------------

/// Integration: Write Perl code to file, parse with parse_perl_file,
/// verify the tree matches parsing the string directly.
#[test]
fn integration_file_parse_matches_string_parse() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
package Calculator;
use strict;
use warnings;

sub new {
    my ($class, %args) = @_;
    return bless \%args, $class;
}

sub add {
    my ($self, $a, $b) = @_;
    return $a + $b;
}

1;
"#;

    let path = temp_perl_file("calculator", source);

    // Parse from file
    let tree_from_file = parse_perl_file(&path)?;

    // Parse from string
    let tree_from_string = parse_perl_code(source)?;

    // The S-expressions should be identical
    let sexp_from_file = tree_from_file.root_node().to_sexp();
    let sexp_from_string = tree_from_string.root_node().to_sexp();

    assert_eq!(
        sexp_from_file, sexp_from_string,
        "File-parsed and string-parsed trees should produce identical S-expressions"
    );

    // Both should have no errors
    assert!(!tree_from_file.root_node().has_error(), "File-parsed tree should have no errors");
    assert!(!tree_from_string.root_node().has_error(), "String-parsed tree should have no errors");

    fs::remove_file(&path).ok();
    Ok(())
}

/// Integration: parse_perl_file propagates file read errors correctly.
#[test]
fn integration_parse_file_error_propagation() {
    let nonexistent = PathBuf::from("/tmp/this_file_definitely_does_not_exist_12345.pl");

    let result = parse_perl_file(&nonexistent);

    assert!(result.is_err(), "parse_perl_file should return Err for missing file");

    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("No such file"),
        "Error message should mention file not found: {}",
        err
    );
}

// ---------------------------------------------------------------------------
// Language Interoperability Integration Tests
// ---------------------------------------------------------------------------

/// Integration: Language returned by crate can be used to create Query objects.
#[test]
fn integration_language_supports_query_creation() -> Result<(), Box<dyn std::error::Error>> {
    let lang = language();

    // Verify we can create a query - using simple comment pattern that should exist
    // This is a basic sanity check that Language supports query operations
    let query = Query::new(&lang, "(comment) @comment")?;

    // Query should have at least one capture
    assert!(!query.capture_names().is_empty(), "Query should have captures");

    Ok(())
}

/// Integration: Multiple parsers created independently should all have
/// the Perl language configured correctly.
#[test]
fn integration_multiple_parsers_all_configured() -> Result<(), Box<dyn std::error::Error>> {
    let parsers: Vec<_> = (0..3).map(|_| try_create_parser()).collect::<Result<Vec<_>, _>>()?;

    assert_eq!(parsers.len(), 3, "Should have created 3 parsers");

    for (i, parser) in parsers.into_iter().enumerate() {
        assert!(parser.language().is_some(), "Parser {} should have language configured", i);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Error Propagation Integration Tests
// ---------------------------------------------------------------------------

/// Integration: Malformed input propagates through the entire pipeline
/// (parse → tree) without panicking, and produces a valid tree with errors.
#[test]
fn integration_malformed_input_error_propagation() -> Result<(), Box<dyn std::error::Error>> {
    let malformed_sources = [
        "my $x = ;",           // missing rhs
        "sub foo { return 1;", // unclosed brace
        r#""unclosed string"#, // unclosed string
        "my $x = (1 + 2;",     // unclosed paren
    ];

    for source in malformed_sources {
        // Parse should return Ok with a tree (possibly with errors)
        let tree = parse_perl_code(source)?;

        // Tree should exist even for malformed input
        assert_eq!(
            tree.root_node().kind(),
            "source_file",
            "Root should be source_file even for malformed input"
        );

        // The tree should have errors flagged
        assert!(
            tree.root_node().has_error(),
            "Malformed input should produce error tree for: {}",
            source
        );
    }

    Ok(())
}

/// Integration: Tree with errors can still produce valid S-expressions.
#[test]
fn integration_error_tree_produces_valid_sexp() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = ;"; // malformed - missing rhs

    let tree = parse_perl_code(source)?;

    // Should have error nodes
    assert!(tree.root_node().has_error(), "Malformed input should produce error tree");

    // But sexp should still be producible and well-formed
    let sexp = tree.root_node().to_sexp();
    assert!(!sexp.is_empty(), "Sexp should not be empty even for error trees");

    // Sexps should have balanced parens
    let mut depth = 0;
    for c in sexp.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
    }
    assert_eq!(depth, 0, "Sexp should have balanced parens: {}", sexp);

    Ok(())
}

// ---------------------------------------------------------------------------
// Multi-Component Handoff Integration Tests
// ---------------------------------------------------------------------------

/// Integration: create_parser() → parse() → tree.root_node() → to_sexp()
/// all work together in a single chain.
#[test]
fn integration_full_pipeline_parser_to_sexp() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
package Greeting;
sub new { my ($class, $name) = @_; bless { name => $name }, $class; }
sub greet { my ($self) = @_; print "Hello, $self->{name}\n"; }
"#;

    // Step 1: Create parser via compatibility shim
    let mut parser = create_parser();
    assert!(parser.language().is_some(), "Parser should have language set");

    // Step 2: Parse (reuse parser from step 1)
    let tree = parser.parse(source, None).ok_or("parse should return Some(tree)")?;

    // Step 3: Get root node
    let root = tree.root_node();
    assert_eq!(root.kind(), "source_file", "Root should be source_file");

    // Step 4: Convert to sexp
    let sexp = root.to_sexp();
    assert!(sexp.contains("package"), "Sexp should contain 'package'");
    assert!(sexp.contains("sub"), "Sexp should contain 'sub'");

    // Step 5: Verify tree has no errors for valid input
    assert!(!root.has_error(), "Valid source should have no errors");

    Ok(())
}

/// Integration: try_create_parser() succeeds and returns configured parser.
#[test]
fn integration_try_create_parser_succeeds() -> Result<(), Box<dyn std::error::Error>> {
    // try_create_parser should succeed (language is available)
    let parser = try_create_parser()?;
    assert!(parser.language().is_some(), "Parser should have language");
    Ok(())
}

/// Integration: Verify parse_perl_code and parse_perl_file produce equivalent
/// trees through the same parser creation path.
#[test]
fn integration_string_and_file_apis_equivalent() -> Result<(), Box<dyn std::error::Error>> {
    let source = "package Test;\nmy $x = 42;\n";

    // Via string
    let tree1 = parse_perl_code(source)?;
    let sexp1 = tree1.root_node().to_sexp();

    // Via file
    let path = temp_perl_file("equiv", source);
    let tree2 = parse_perl_file(&path)?;
    let sexp2 = tree2.root_node().to_sexp();

    assert_eq!(sexp1, sexp2, "Both APIs should produce identical S-expressions");

    fs::remove_file(&path).ok();
    Ok(())
}
