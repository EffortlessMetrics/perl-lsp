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

mod while_condition_indirect_call {
    use super::*;

    #[test]
    fn while_shift_array() {
        // File::Spec::Unix pattern — while( shift @chunks )
        let code = "while (shift @arr) { last; }";
        assert_clean_parse(code);
    }

    #[test]
    fn while_pop_array() {
        let code = "while (pop @arr) { last; }";
        assert_clean_parse(code);
    }

    #[test]
    fn while_assign_shift() {
        // Common idiom: while (my $x = shift @args) { ... }
        let code = "while (my $x = shift @args) { print $x; }";
        assert_clean_parse(code);
    }

    #[test]
    fn while_defined_my_shift() {
        // File::Spec::Unix line pattern
        let code = "while (defined(my $dir = shift @basechunks)) { push @chunks, $dir; }";
        assert_clean_parse(code);
    }

    #[test]
    fn if_shift_array() {
        let code = "if (shift @arr) { return 1; }";
        assert_clean_parse(code);
    }

    #[test]
    fn if_pop_array() {
        let code = "if (pop @arr) { return 1; }";
        assert_clean_parse(code);
    }

    #[test]
    fn elsif_shift_array() {
        let code = "if (1) { } elsif (shift @arr) { return 1; }";
        assert_clean_parse(code);
    }

    #[test]
    fn unless_shift_array() {
        let code = "unless (shift @arr) { return 1; }";
        assert_clean_parse(code);
    }

    #[test]
    fn until_shift_array() {
        let code = "until (shift @arr) { last; }";
        assert_clean_parse(code);
    }

    #[test]
    fn for_condition_shift() {
        let code = "for (my $i = 0; shift @arr; $i++) { }";
        assert_clean_parse(code);
    }

    #[test]
    fn foreach_list_shift() {
        // foreach with a complex list expression
        let code = "foreach my $x (@arr) { print $x; }";
        assert_clean_parse(code);
    }

    #[test]
    fn while_scalar_shift() {
        // scalar context shift is common
        let code = "while (my $item = shift @items) { process($item); }";
        assert_clean_parse(code);
    }
}
