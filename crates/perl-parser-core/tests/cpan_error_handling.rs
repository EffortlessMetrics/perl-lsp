//! CPAN Pattern Tests: Error Handling

mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn eval_die_pattern() {
    let code = "eval { die 'oops' }; warn $@ if $@;";
    assert_clean_parse(code);
}

#[test]
fn eval_with_error_check() {
    let code = r#"
eval {
    require Some::Module;
    Some::Module->import;
};
if ($@) {
    warn "Module not available: $@";
}
"#;
    assert_clean_parse(code);
}

#[test]
fn die_with_reference() {
    let code = r#"die { code => 404, message => "Not found" };"#;
    assert_clean_parse(code);
}

#[test]
fn croak_confess() {
    let code = r#"
use Carp qw(croak confess);
croak "Invalid argument" unless defined $arg;
confess "Deep error: $msg";
"#;
    assert_clean_parse(code);
}

#[test]
fn local_sig_warn() {
    let code = "local $SIG{__WARN__} = sub { };";
    assert_clean_parse(code);
}

#[test]
fn local_sig_die() {
    let code = "local $SIG{__DIE__} = sub { log_error($_[0]) };";
    assert_clean_parse(code);
}

#[test]
fn conditional_require() {
    let code = "eval { require JSON::XS }; my $json = $@ ? JSON::PP->new : JSON::XS->new;";
    assert_clean_parse(code);
}

#[test]
fn string_eval() {
    let code = r#"eval "use $module";"#;
    assert_clean_parse(code);
}
