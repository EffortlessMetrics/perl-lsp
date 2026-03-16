//! CPAN Pattern Tests: Statement Modifiers

mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn if_modifier() {
    let code = "print $x if defined $x;";
    assert_clean_parse(code);
}

#[test]
fn unless_modifier() {
    let code = "die 'not found' unless $file;";
    assert_clean_parse(code);
}

#[test]
fn while_modifier() {
    let code = "print while <STDIN>;";
    assert_clean_parse(code);
}

#[test]
fn until_modifier() {
    let code = "$count++ until $count > 10;";
    assert_clean_parse(code);
}

#[test]
fn for_modifier() {
    let code = "print $_ for @items;";
    assert_clean_parse(code);
}

#[test]
fn foreach_modifier() {
    let code = "push @results, process($_) foreach @inputs;";
    assert_clean_parse(code);
}

#[test]
fn chained_modifier_with_next() {
    let code = "next unless defined $row;";
    assert_clean_parse(code);
}

#[test]
fn return_modifier() {
    let code = "return $cached if exists $cache{$key};";
    assert_clean_parse(code);
}
