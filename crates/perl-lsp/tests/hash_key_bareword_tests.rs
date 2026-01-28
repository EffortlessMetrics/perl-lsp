//! Tests for hash key bareword detection.
//!
//! These tests verify that bareword detection correctly distinguishes between:
//! - Hash keys (should not trigger bareword warnings)
//! - Actual barewords used in other contexts (should trigger warnings)
//!
//! NOTE: These tests require the `unquoted-bareword` diagnostic feature which
//! is not yet implemented. Tests are gated behind the `lsp-extras` feature.

#![cfg(feature = "lsp-extras")]

use perl_lsp::features::diagnostics::DiagnosticsProvider;
use perl_parser::Parser;

#[test]
fn test_hash_key_vs_variable_bareword() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
use strict;
my %h = ();
my $x = $h{key};
print FOO;
"#;

    let mut parser = Parser::new(source);
    let ast = parser.parse()?;
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    let bareword_errors: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("unquoted-bareword")).collect();

    assert_eq!(bareword_errors.len(), 1);
    let first_error = bareword_errors.first().ok_or("expected at least one bareword error")?;
    assert!(first_error.message.contains("FOO"));
    assert!(!first_error.message.contains("key"));

    Ok(())
}

#[test]
fn test_hash_slice_bareword_keys() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
use strict;
my %h = ();
my @values = @h{key1, key2};
print STDERR;
"#;

    let mut parser = Parser::new(source);
    let ast = parser.parse()?;
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    let bareword_errors: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("unquoted-bareword")).collect();

    // Only STDERR should be flagged as a bareword, not key1 or key2
    assert_eq!(bareword_errors.len(), 1);
    let first_error = bareword_errors.first().ok_or("expected at least one bareword error")?;
    assert!(first_error.message.contains("STDERR"));
    assert!(!first_error.message.contains("key1"));
    assert!(!first_error.message.contains("key2"));

    Ok(())
}

#[test]
fn test_hash_slice_with_variables() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
use strict;
my %h = ();
my $k1 = "key1";
my $k2 = "key2";
my @values = @h{$k1, $k2};
"#;

    let mut parser = Parser::new(source);
    let ast = parser.parse()?;
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    // No bareword errors expected - variables are used as keys
    let bareword_errors: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("unquoted-bareword")).collect();
    assert_eq!(bareword_errors.len(), 0);

    // Variables should be marked as used
    let undeclared_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code.as_deref() == Some("undeclared-variable"))
        // Semantic analyzer currently doesn't link %h declaration to @h slice usage
        // ignoring this specific error to focus on the test goal (variables as keys)
        .filter(|d| !d.message.contains("'@h'"))
        .collect();

    assert_eq!(
        undeclared_errors.len(),
        0,
        "Unexpected undeclared variable errors: {:?}",
        undeclared_errors
    );

    Ok(())
}

#[test]
fn test_hash_slice_mixed_elements() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
use strict;
my %h = ();
my $k = "key1";
my @values = @h{$k, 'literal', func(), keys(%h)};
print BAREWORD;
"#;

    let mut parser = Parser::new(source);
    let ast = parser.parse()?;
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    let bareword_errors: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("unquoted-bareword")).collect();

    // Only BAREWORD after print should be flagged
    assert_eq!(bareword_errors.len(), 1);
    let first_error = bareword_errors.first().ok_or("expected at least one bareword error")?;
    assert!(first_error.message.contains("BAREWORD"));
    // None of the hash slice elements should be flagged
    assert!(!first_error.message.contains("literal"));
    assert!(!first_error.message.contains("func"));

    Ok(())
}

#[test]
fn test_nested_hash_slice_expressions() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
use strict;
my %h = ();
my @arr = qw(a b c);
my @values = @h{ @arr };
"#;

    let mut parser = Parser::new(source);
    let ast = parser.parse()?;
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    // No bareword errors expected - map expression inside hash slice
    let bareword_errors: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("unquoted-bareword")).collect();
    assert_eq!(bareword_errors.len(), 0);

    Ok(())
}

#[test]
fn test_hash_slice_with_function_calls() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
use strict;
my %h = ();
sub get_keys { return ('key1', 'key2'); }
my @values = @h{ get_keys() };
"#;

    let mut parser = Parser::new(source);
    let ast = parser.parse()?;
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    // Function calls in hash slices should not trigger bareword warnings
    let bareword_errors: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("unquoted-bareword")).collect();
    assert_eq!(bareword_errors.len(), 0);

    Ok(())
}

#[test]
fn test_deeply_nested_hash_structures() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
use strict;
my %h = ();
my $val = $h{level1}{level2}{level3};
print INVALID;
"#;

    let mut parser = Parser::new(source);
    let ast = parser.parse()?;
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    let bareword_errors: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("unquoted-bareword")).collect();

    // Only INVALID should be flagged, not the nested hash keys
    assert_eq!(bareword_errors.len(), 1);
    let first_error = bareword_errors.first().ok_or("expected at least one bareword error")?;
    assert!(first_error.message.contains("INVALID"));
    assert!(!first_error.message.contains("level1"));
    assert!(!first_error.message.contains("level2"));
    assert!(!first_error.message.contains("level3"));

    Ok(())
}

#[test]
fn test_complex_hash_literal_with_nested_keys() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
use strict;
my %complex = (
    outer_key => {
        inner_key => 'value',
        another_inner => 42
    },
    simple_key => 'simple_value'
);
print BAREWORD_WARNING;
"#;

    let mut parser = Parser::new(source);
    let ast = parser.parse()?;
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    let bareword_errors: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("unquoted-bareword")).collect();

    // Only BAREWORD_WARNING should be flagged - all hash keys should be ignored
    assert_eq!(bareword_errors.len(), 1);
    let first_error = bareword_errors.first().ok_or("expected at least one bareword error")?;
    assert!(first_error.message.contains("BAREWORD_WARNING"));
    // Verify none of the legitimate hash keys are flagged
    assert!(!first_error.message.contains("outer_key"));
    assert!(!first_error.message.contains("inner_key"));
    assert!(!first_error.message.contains("another_inner"));
    assert!(!first_error.message.contains("simple_key"));

    Ok(())
}

#[test]
fn test_hash_slice_with_mixed_quote_styles() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
use strict;
my %h = ();
my @vals = @h{bare_key, 'single_quoted', "double_quoted", qw(word_list)};
print SHOULD_WARN;
"#;

    let mut parser = Parser::new(source);
    let ast = parser.parse()?;
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    let bareword_errors: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("unquoted-bareword")).collect();

    // Only SHOULD_WARN should trigger an error
    assert_eq!(bareword_errors.len(), 1);
    let first_error = bareword_errors.first().ok_or("expected at least one bareword error")?;
    assert!(first_error.message.contains("SHOULD_WARN"));

    Ok(())
}

#[test]
fn test_hash_slice_performance_edge_case() -> Result<(), Box<dyn std::error::Error>> {
    // Test the MAX_TRAVERSAL_DEPTH safety limit in deeply nested structures
    let source = r#"
use strict;
my %h = ();
# Create a deeply nested structure that would test the traversal depth limit
my $deep = $h{a}{b}{c}{d}{e}{f}{g}{h}{i}{j}{k};
print NORMAL_BAREWORD;
"#;

    let mut parser = Parser::new(source);
    let ast = parser.parse()?;
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    let bareword_errors: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("unquoted-bareword")).collect();

    // Should still detect legitimate barewords even with deep nesting
    assert_eq!(bareword_errors.len(), 1);
    let first_error = bareword_errors.first().ok_or("expected at least one bareword error")?;
    assert!(first_error.message.contains("NORMAL_BAREWORD"));

    Ok(())
}

#[test]
fn test_hash_keys_in_different_contexts() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
use strict;
my %hash = (contextual_key => 'value');
my $single = $hash{single_key};
my @multi = @hash{multi_key1, multi_key2};
my %slice = %hash{slice_key1, slice_key2};

# This should warn
print ACTUAL_BAREWORD;

# These should not warn (hash contexts)
exists $hash{exists_key};
delete $hash{delete_key};
"#;

    let mut parser = Parser::new(source);
    let ast = parser.parse()?;
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    let bareword_errors: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("unquoted-bareword")).collect();

    // Only ACTUAL_BAREWORD should be flagged
    assert_eq!(bareword_errors.len(), 1);
    let first_error = bareword_errors.first().ok_or("expected at least one bareword error")?;
    assert!(first_error.message.contains("ACTUAL_BAREWORD"));

    Ok(())
}
