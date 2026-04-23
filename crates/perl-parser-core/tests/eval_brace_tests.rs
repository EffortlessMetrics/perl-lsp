mod cpan_test_helpers;
use cpan_test_helpers::*;

// Tests for the unclosed_brace_semicolon error bucket.
// These patterns trigger "expected '}', found ';'" because the parser
// treats `identifier { ... }` as hash element access instead of a
// function call with a block argument.

#[test]
fn test_eval_block_in_list_context() {
    let source = r#"
my $result = eval { my $x = 1; $x + 2; };
"#;
    assert_clean_parse(source);
}

#[test]
fn test_eval_block_in_if_condition() {
    let source = r#"
if (eval { require Some::Module; 1 }) {
    print "loaded\n";
}
"#;
    assert_clean_parse(source);
}

#[test]
fn test_eval_block_ternary() {
    let source = r#"
my $val = eval { dangerous(); 1 } ? "ok" : "fail";
"#;
    assert_clean_parse(source);
}

#[test]
fn test_capture_block_with_eval() {
    let source = r#"
my $output = capture { eval { die "oops"; }; print "hello"; };
"#;
    assert_clean_parse(source);
}

#[test]
fn test_bare_func_block_simple() {
    let source = r#"
scope_guard { cleanup(); };
"#;
    assert_clean_parse(source);
}

#[test]
fn test_bare_func_block_multiple_stmts() {
    let source = r#"
transaction { my $x = 1; do_work($x); commit(); };
"#;
    assert_clean_parse(source);
}

#[test]
fn test_bare_func_block_with_trailing_args() {
    let source = r#"
my @results = filter { $_->is_valid; } @items;
"#;
    assert_clean_parse(source);
}

#[test]
fn test_qualified_func_block() {
    let source = r#"
Tk::catch { die "caught"; };
"#;
    assert_clean_parse(source);
}

#[test]
fn test_moose_where_block() {
    let source = r#"
subtype 'PositiveInt', as 'Int', where { $_ > 0 };
"#;
    assert_clean_parse(source);
}

#[test]
fn test_moose_inline_as_block() {
    let source = r#"
coerce 'MyType', from 'Str', via { MyType->new($_) };
"#;
    assert_clean_parse(source);
}

#[test]
fn test_begin_eval_require() {
    let source = r#"
BEGIN { eval { require Some::Module; }; }
"#;
    assert_clean_parse(source);
}

#[test]
fn test_nested_eval_blocks() {
    let source = r#"
my $r = eval {
    my $inner = eval { die "inner"; };
    handle($inner);
    1;
};
"#;
    assert_clean_parse(source);
}

#[test]
fn test_eval_or_die() {
    let source = r#"
eval { require Foo::Bar; 1; } or die "Cannot load Foo::Bar: $@";
"#;
    assert_clean_parse(source);
}

#[test]
fn test_eval_and_check() {
    let source = r#"
my $ok = eval { $obj->method(); 1; } && $extra_check;
"#;
    assert_clean_parse(source);
}

#[test]
fn test_bare_func_block_in_sub() {
    let source = r#"
sub setup {
    my $guard = scope_guard { cleanup(); };
    do_work();
}
"#;
    assert_clean_parse(source);
}

#[test]
fn test_bare_func_block_chained() {
    let source = r#"
before_each { setup(); };
after_each { teardown(); };
"#;
    assert_clean_parse(source);
}

#[test]
fn test_declare_with_block() {
    let source = r#"
declare "MyType", where { defined($_) && length($_) > 0 };
"#;
    assert_clean_parse(source);
}
