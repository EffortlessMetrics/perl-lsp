mod cpan_test_helpers;
use cpan_test_helpers::*;

// foreach (LIST) — implicit $_ topic variable
#[test]
fn foreach_implicit_topic_no_variable() {
    assert_clean_parse("foreach (@INC) { print; }");
}

#[test]
fn foreach_implicit_topic_keys() {
    assert_clean_parse("foreach (keys %ENV) { print \"$_\\n\"; }");
}

#[test]
fn foreach_implicit_topic_array_ref() {
    assert_clean_parse("foreach (@{$list}) { process($_); }");
}

// foreach $var (LIST) — bare scalar without my
#[test]
fn foreach_bare_scalar_no_my() {
    assert_clean_parse("foreach $mod (@ISA) { eval \"require $mod\"; }");
}

#[test]
fn foreach_bare_scalar_dirs() {
    assert_clean_parse("foreach $dir (@dirs) { push @found, $dir if -d $dir; }");
}

#[test]
fn for_bare_scalar_no_my() {
    assert_clean_parse("for $mod (@ISA) { print $mod; }");
}

// Regression: existing working cases still work
#[test]
fn foreach_with_my_still_works() {
    assert_clean_parse("foreach my $item (@list) { print $item; }");
}

#[test]
fn for_with_my_still_works() {
    assert_clean_parse("for my $i (1..10) { print $i; }");
}

// Labeled foreach with implicit $_
#[test]
fn labeled_foreach_implicit_topic() {
    assert_clean_parse("LOOP: foreach (@args) { next LOOP if $_ eq 'skip'; }");
}
