mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn test_c_style_for_basic() {
    let source = r#"for (my $i = 0; $i < 10; $i++) { print $i; }"#;
    assert_clean_parse(source);
}

#[test]
fn test_c_style_for_empty_init() {
    let source = r#"for (; $i < 10; $i++) { print $i; }"#;
    assert_clean_parse(source);
}

#[test]
fn test_c_style_for_empty_all() {
    let source = r#"for (;;) { last; }"#;
    assert_clean_parse(source);
}

#[test]
fn test_c_style_for_complex_update() {
    let source = r#"for (my $i = 0; $i < scalar @items; $i += 2) { print $items[$i]; }"#;
    assert_clean_parse(source);
}

#[test]
fn test_c_style_for_multiple_vars() {
    let source = r#"for (my $i = 0; $i <= $#array; $i++) { $result{$array[$i]} = $i; }"#;
    assert_clean_parse(source);
}

#[test]
fn test_c_style_for_multi_statement_body() {
    let source = r#"for (my $i = 0; $i < 10; $i++) {
        my $x = $i * 2;
        print "$x\n";
    }"#;
    assert_clean_parse(source);
}

#[test]
fn test_c_style_for_nested() {
    let source = r#"for (my $i = 0; $i < 10; $i++) {
        for (my $j = 0; $j < 10; $j++) {
            print "$i $j\n";
        }
    }"#;
    assert_clean_parse(source);
}

#[test]
fn test_c_style_for_expression_init() {
    let source = r#"for ($i = 0; $i < 10; $i++) { print $i; }"#;
    assert_clean_parse(source);
}

// Patterns that may cause "expected expression, found ';'" on CPAN

#[test]
fn test_empty_statement_in_block() {
    // Double semicolon or empty statement
    let source = r#"{ my $x = 1;; my $y = 2; }"#;
    assert_clean_parse(source);
}

#[test]
fn test_semicolon_after_close_brace() {
    let source = r#"if (1) { print "yes"; }; print "done";"#;
    assert_clean_parse(source);
}

#[test]
fn test_do_while_semicolon() {
    // do { ... } while (cond); -- the trailing semicolon
    let source = r#"do { print "hello"; } while (1);"#;
    assert_clean_parse(source);
}

#[test]
fn test_eval_block_semicolon() {
    let source = r#"eval { die "error"; }; print "survived";"#;
    assert_clean_parse(source);
}

#[test]
fn test_ternary_with_semicolons_nearby() {
    let source = r#"my $x = $a ? $b : $c;"#;
    assert_clean_parse(source);
}

#[test]
fn test_chained_method_calls_semicolons() {
    let source = r#"$obj->method1(); $obj->method2();"#;
    assert_clean_parse(source);
}

#[test]
fn test_hash_ref_constructor() {
    let source = r#"my $h = { a => 1, b => 2 };"#;
    assert_clean_parse(source);
}

#[test]
fn test_complex_assignment_with_semicolon() {
    let source = r#"my ($a, $b, $c) = (1, 2, 3);"#;
    assert_clean_parse(source);
}

#[test]
fn test_semicolons_in_prototype() {
    // Prototypes use ; to separate required and optional args
    let source = r#"sub foo ($$;@) { return @_; }"#;
    assert_clean_parse(source);
}

#[test]
fn test_for_with_last_and_next() {
    let source = r#"for (my $i = 0; $i < 100; $i++) {
        next if $i % 2;
        last if $i > 50;
        print $i;
    }"#;
    assert_clean_parse(source);
}

#[test]
fn test_postfix_deref_with_c_style_for() {
    let source = r#"for (my $i = 0; $i < $data->@*; $i++) {
        print $data->[$i];
    }"#;
    assert_clean_parse(source);
}
