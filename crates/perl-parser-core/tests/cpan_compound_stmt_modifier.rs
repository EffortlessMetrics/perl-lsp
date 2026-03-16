//! CPAN Pattern Tests: Compound Statement Modifiers

mod cpan_test_helpers;
use cpan_test_helpers::*;

/// Two consecutive if-blocks must NOT be misread as if-block + postfix modifier.
#[test]
fn two_consecutive_if_blocks() {
    let code = r#"
if ($a) { foo(); }
if ($b) { bar(); }
"#;
    assert_clean_parse(code);
}

/// while-block followed by a bare if-block.
#[test]
fn while_block_then_if_block() {
    let code = r#"
while (1) { last; }
if ($done) { return; }
"#;
    assert_clean_parse(code);
}

/// for-block followed by another for-block.
#[test]
fn for_block_then_for_block() {
    let code = r#"
for my $i (1..10) { print $i; }
for my $j (1..5) { print $j; }
"#;
    assert_clean_parse(code);
}

/// foreach-block followed by an if-block.
#[test]
fn foreach_block_then_if_block() {
    let code = r#"
foreach my $item (@list) { process($item); }
if (@list) { done(); }
"#;
    assert_clean_parse(code);
}

/// sub definition followed by an if-block.
#[test]
fn sub_then_if_block() {
    let code = r#"
sub foo { return 1; }
if ($x) { foo(); }
"#;
    assert_clean_parse(code);
}

/// Postfix modifier on a plain expression statement still works.
#[test]
fn postfix_if_on_expression() {
    let code = "print $x if $debug;";
    assert_clean_parse(code);
}

/// Postfix unless on a plain expression statement still works.
#[test]
fn postfix_unless_on_expression() {
    let code = "return if $done;";
    assert_clean_parse(code);
}

/// Postfix while on a plain expression statement still works.
#[test]
fn postfix_while_on_expression() {
    let code = "do_something() while $running;";
    assert_clean_parse(code);
}

/// Common OO pattern: multiple method definitions followed by logic.
#[test]
fn multiple_subs_then_if() {
    let code = r#"
sub init { return 1; }
sub run  { return 2; }
if ($start) { init(); run(); }
"#;
    assert_clean_parse(code);
}
