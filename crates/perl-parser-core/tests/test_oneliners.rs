mod cpan_test_helpers;
use cpan_test_helpers::*;

fn first_errors(code: &str) -> Vec<String> {
    let mut parser = perl_parser_core::Parser::new(code);
    let _ = parser.parse();
    parser.errors().iter().take(5).map(|e| format!("{}", e)).collect()
}

// DBI::ProfileData pattern
#[test]
fn test_sub_oneliner_scalar_deref() {
    let code = r#"sub count { scalar @{shift->{_nodes}} }"#;
    let errors = first_errors(code);
    for e in &errors {
        eprintln!("  Error: {}", e);
    }
    assert_clean_parse(code);
}

// Tk::DragDrop::Rect pattern
#[test]
fn test_sub_oneliner_scalar_deref_method_chain() {
    let code = r#"sub ancestor { ${shift->widget->toplevel->WindowId} }"#;
    let errors = first_errors(code);
    for e in &errors {
        eprintln!("  Error: {}", e);
    }
    assert_clean_parse(code);
}

// PPI::Token::HereDoc pattern
#[test]
fn test_sub_oneliner_array_deref_shift() {
    let code = r#"sub heredoc { @{shift->{_heredoc}} }"#;
    let errors = first_errors(code);
    for e in &errors {
        eprintln!("  Error: {}", e);
    }
    assert_clean_parse(code);
}

// Template::Plugin::Filter pattern
#[test]
fn test_return_method_or_error() {
    let code = r#"
sub new {
    my $self = bless {
        _CONTEXT => $context,
        _STATIC  => 0,
    }, $class;
    return $self->init($config)
        || $class->error($self->error());
}
"#;
    let errors = first_errors(code);
    for e in &errors {
        eprintln!("  Error: {}", e);
    }
    assert_clean_parse(code);
}

// Tk::Image pattern
#[test]
fn test_symbolic_delete_deref() {
    let code = r#"delete ${"$class\::"}{'::ISA::CACHE::'};"#;
    let errors = first_errors(code);
    for e in &errors {
        eprintln!("  Error: {}", e);
    }
    assert_clean_parse(code);
}

// POE pattern
#[test]
fn test_grep_exists_forbidden() {
    let code = r#"
push(
    @forbidden_handlers,
    grep { exists $forbidden_handlers{$_} }
    @$handlers
);
"#;
    let errors = first_errors(code);
    for e in &errors {
        eprintln!("  Error: {}", e);
    }
    assert_clean_parse(code);
}

// Exporter::Heavy pattern
#[test]
fn test_foreach_delete_hash_element() {
    let code = r#"foreach $sym (@names) { delete $imports{$sym} }"#;
    let errors = first_errors(code);
    for e in &errors {
        eprintln!("  Error: {}", e);
    }
    assert_clean_parse(code);
}
