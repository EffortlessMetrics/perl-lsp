use perl_parser::{DiagnosticsProvider, Parser};

#[test]
fn test_hash_key_vs_variable_bareword() {
    let source = r#"
use strict;
my %h = ();
my $x = $h{key};
print FOO;
"#;

    let mut parser = Parser::new(source);
    let ast = parser.parse().unwrap();
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    let bareword_errors: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("unquoted-bareword")).collect();

    assert_eq!(bareword_errors.len(), 1);
    assert!(bareword_errors[0].message.contains("FOO"));
    assert!(!bareword_errors[0].message.contains("key"));
}

#[test]
fn test_hash_slice_bareword_keys() {
    let source = r#"
use strict;
my %h = ();
my @values = @h{key1, key2};
print STDERR;
"#;

    let mut parser = Parser::new(source);
    let ast = parser.parse().unwrap();
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    let bareword_errors: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("unquoted-bareword")).collect();

    // Only STDERR should be flagged as a bareword, not key1 or key2
    assert_eq!(bareword_errors.len(), 1);
    assert!(bareword_errors[0].message.contains("STDERR"));
    assert!(!bareword_errors[0].message.contains("key1"));
    assert!(!bareword_errors[0].message.contains("key2"));
}

#[test]
fn test_hash_slice_with_variables() {
    let source = r#"
use strict;
my %h = ();
my $k1 = "key1";
my $k2 = "key2";
my @values = @h{$k1, $k2};
"#;

    let mut parser = Parser::new(source);
    let ast = parser.parse().unwrap();
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    // No bareword errors expected - variables are used as keys
    let bareword_errors: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("unquoted-bareword")).collect();
    assert_eq!(bareword_errors.len(), 0);

    // Variables should be marked as used
    let undeclared_errors: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("undeclared-variable")).collect();
    assert_eq!(undeclared_errors.len(), 0);
}

#[test]
fn test_hash_slice_mixed_elements() {
    let source = r#"
use strict;
my %h = ();
my $k = "key1";
my @values = @h{$k, 'literal', func(), keys %h};
print BAREWORD;
"#;

    let mut parser = Parser::new(source);
    let ast = parser.parse().unwrap();
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    let bareword_errors: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("unquoted-bareword")).collect();

    // Only BAREWORD after print should be flagged
    assert_eq!(bareword_errors.len(), 1);
    assert!(bareword_errors[0].message.contains("BAREWORD"));
    // None of the hash slice elements should be flagged
    assert!(!bareword_errors[0].message.contains("literal"));
    assert!(!bareword_errors[0].message.contains("func"));
}

#[test]
fn test_nested_hash_slice_expressions() {
    let source = r#"
use strict;
my %h = ();
my @arr = qw(a b c);
my @values = @h{ @arr };
"#;

    let mut parser = Parser::new(source);
    let ast = parser.parse().unwrap();
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    // No bareword errors expected - map expression inside hash slice
    let bareword_errors: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("unquoted-bareword")).collect();
    assert_eq!(bareword_errors.len(), 0);
}

#[test]
fn test_hash_slice_with_function_calls() {
    let source = r#"
use strict;
my %h = ();
sub get_keys { return ('key1', 'key2'); }
my @values = @h{ get_keys() };
"#;

    let mut parser = Parser::new(source);
    let ast = parser.parse().unwrap();
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);

    // Function calls in hash slices should not trigger bareword warnings
    let bareword_errors: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("unquoted-bareword")).collect();
    assert_eq!(bareword_errors.len(), 0);
}
