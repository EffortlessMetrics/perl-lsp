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
