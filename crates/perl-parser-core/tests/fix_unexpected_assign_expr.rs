mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn b_deparse_bare_substr_lvalue_assignment_with_trailing_comma() {
    assert_clean_parse(
        r#"
sub lex_in_scope {
    my ($self, $name, $our) = @_;
    substr $name, 0, 0, = $our ? 'o' : 'm';
}
"#,
    );
}

#[test]
fn b_deparse_bare_substr_lvalue_assignment_after_and() {
    assert_clean_parse(
        r#"
sub qualify {
    my ($self, $kid, $fq) = @_;
    $fq and substr $kid, 0, 0, = $self->{'curstash'} . '::';
}
"#,
    );
}
