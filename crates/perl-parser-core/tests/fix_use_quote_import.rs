mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn test_use_overload_q_brace_multi_stmt() {
    assert_clean_parse(r#"use overload q{""} => sub { my $x = 1; $x };"#);
}

#[test]
fn test_use_overload_q_paren_multi_stmt() {
    assert_clean_parse(r#"use overload q("") => sub { my $x = 1; $x };"#);
}

#[test]
fn test_use_overload_q_bracket_multi_stmt() {
    assert_clean_parse(r#"use overload q[""] => sub { my $x = 1; $x };"#);
}

// Real-world from Regexp::Common
#[test]
fn test_regexp_common_pattern() {
    assert_clean_parse(
        r#"
use overload
    q{""} => sub {
        my ($self) = @_;
        my $pat = $self->{create}->($self, $self->{flags}, $self->{args});
        return $pat;
    };
"#,
    );
}
