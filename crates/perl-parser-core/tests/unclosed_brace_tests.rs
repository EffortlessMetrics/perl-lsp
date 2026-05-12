mod cpan_test_helpers;
use cpan_test_helpers::*;

// --- @{Package::Name} dereference patterns ---

#[test]
fn test_array_deref_package_name() {
    let source = r#"my @items = @{Foo::Bar::items};"#;
    assert_clean_parse(source);
}

#[test]
fn test_array_deref_nested_package() {
    let source = r#"my @list = @{Some::Deep::Package::list()};"#;
    assert_clean_parse(source);
}

#[test]
fn test_hash_deref_package_name() {
    let source = r#"my %data = %{Config::Data::hash};"#;
    assert_clean_parse(source);
}

#[test]
fn test_array_deref_function_call() {
    let source = r#"my @r = @{get_items()};"#;
    assert_clean_parse(source);
}

#[test]
fn test_array_deref_simple_ident() {
    let source = r#"my @r = @{arrayref};"#;
    assert_clean_parse(source);
}

// --- use if / use unless with eval blocks ---

#[test]
fn test_use_if_eval_block() {
    let source = r#"use if eval { require Foo; 1 }, 'Foo';"#;
    assert_clean_parse(source);
}

#[test]
fn test_use_if_eval_block_complex() {
    let source = r#"use if eval { require Some::Module; 1; }, 'Some::Module', qw(func1 func2);"#;
    assert_clean_parse(source);
}

#[test]
fn test_use_if_simple_condition() {
    let source = r#"use if $] >= 5.010, 'mro';"#;
    assert_clean_parse(source);
}

#[test]
fn test_use_unless_condition() {
    let source = r#"use unless $ENV{NO_FOO}, 'Foo::Bar';"#;
    assert_clean_parse(source);
}

#[test]
fn test_use_if_negation() {
    let source = r#"use if !$ENV{SKIP}, 'Module::Name';"#;
    assert_clean_parse(source);
}

#[test]
fn fields_warnings_register_fallback() {
    // From fields.pm: an unless condition may contain an eval block, followed
    // by a qualified typeglob assignment to an anonymous sub fallback.
    let source = r#"
unless( eval {require warnings::register; warnings::register->import; 1} ) {
    *warnings::warnif = sub {
        require Carp;
        Carp::carp(@_);
    }
}
"#;
    assert_clean_parse(source);
}

#[test]
fn test_more_unless_diag_heredoc() {
    // From Test::More: heredoc terminators inside an unless block must close the
    // diagnostic call without leaving the block body waiting for another `}`.
    let source = r#"unless($ok) {
    chomp $eval_error;
    $tb->diag(<<DIAGNOSTIC);
    Tried to require '$module'.
    Error:  $eval_error
DIAGNOSTIC

}
"#;
    assert_clean_parse(source);
}

// --- Combined patterns from CPAN corpus ---

#[test]
fn test_mixed_deref_and_use_if() {
    let source = r#"
use if eval { require JSON::XS; 1 }, 'JSON::XS';
my @keys = @{Some::Config::keys};
my %opts = %{Default::Options::hash};
"#;
    assert_clean_parse(source);
}

#[test]
fn test_use_if_version_check() {
    let source = r#"use if $] >= 5.008001, 'utf8';"#;
    assert_clean_parse(source);
}
