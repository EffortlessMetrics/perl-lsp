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
