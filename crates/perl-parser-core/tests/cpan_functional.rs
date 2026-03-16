//! CPAN Pattern Tests: Functional Patterns

mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn map_method_call() {
    let code = "my @names = map { $_->name } @objects;";
    assert_clean_parse(code);
}

#[test]
fn grep_method_call() {
    let code = "my @active = grep { $_->is_active } @users;";
    assert_clean_parse(code);
}

#[test]
fn map_grep_chain() {
    let code = "my @results = map { $_->name } grep { $_->is_active } @users;";
    assert_clean_parse(code);
}

#[test]
fn sort_with_custom_comparator() {
    let code = r#"my @sorted = sort { $a->{score} <=> $b->{score} || $a->{name} cmp $b->{name} } @players;"#;
    assert_clean_parse(code);
}

#[test]
fn sort_by_key() {
    let code = "my @sorted = sort { lc($a) cmp lc($b) } @words;";
    assert_clean_parse(code);
}

#[test]
fn grep_complex_condition() {
    let code = "my @valid = grep { defined $_ && length($_) > 0 && $_ !~ /^#/ } @lines;";
    assert_clean_parse(code);
}

#[test]
fn map_transform() {
    let code = "my @upper = map { uc $_ } @strings;";
    assert_clean_parse(code);
}

#[test]
fn map_expression_form() {
    let code = "my @doubled = map { $_ * 2 } 1 .. 10;";
    assert_clean_parse(code);
}

#[test]
fn chained_string_ops() {
    let code = r#"my $clean = lc(join('-', split(/\s+/, $input)));"#;
    assert_clean_parse(code);
}

#[test]
fn for_postfix() {
    let code = "print $_ for @items;";
    assert_clean_parse(code);
}

#[test]
fn foreach_with_variable() {
    let code = "foreach my $item (@list) { process($item) }";
    assert_clean_parse(code);
}
