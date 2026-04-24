//! Tests for issue #2390: unclosed_paren parse failures in complex multi-line constructs.
//!
//! Root cause: the lexer fuses the `x` repetition operator with adjacent digit characters
//! when there is no whitespace (e.g. `x4` is lexed as one Identifier token "x4").
//! `parse_multiplicative_with` now detects this and splits the token at the boundary.
//!
//! Sub-patterns covered:
//! 1. `x<digits>` fused token -- `("")x4`, `("")x10`, `()x4`
//! 2. Split with regex inside complex paren context (real-world URI::_ldap pattern)
//! 3. Multi-line sprintf / printf argument lists
//! 4. Method chaining across lines inside paren argument list
//! 5. unless with multi-line condition block

mod cpan_test_helpers;
use cpan_test_helpers::*;

// -- Sub-pattern 1: fused `x<digits>` operator --

#[test]
fn test_x_rep_fused_empty_list_in_list() {
    // `()x4` inside an outer list -- lexer produces Identifier("x4")
    assert_clean_parse(r#"my @x = ("a", ()x4);"#);
}

#[test]
fn test_x_rep_fused_paren_str_in_list() {
    // `("")x4` -- the classic URI::_ldap pattern
    assert_clean_parse(r#"my @x = ("a", ("")x4);"#);
}

#[test]
fn test_x_rep_fused_large_count() {
    // Two-digit count: `x10`
    assert_clean_parse(r#"my @x = ("", ("")x10);"#);
}

#[test]
fn test_x_rep_fused_in_outer_paren() {
    // Outer paren list with multiple elements followed by fused x-rep
    assert_clean_parse(r#"my @bits = ((""), ("")x4);"#);
}

#[test]
fn test_x_rep_fused_complex_first_arg() {
    // Real-world URI::_ldap pattern: split() result prepended to x-rep padding
    assert_clean_parse(r#"my @bits = (split(/\?/, defined($query) ? $query : ""), ("")x4);"#);
}

// -- Sub-pattern 2: multi-line sprintf / printf --

#[test]
fn test_sprintf_multiline_args() {
    // sprintf with format string and arguments on separate lines
    assert_clean_parse(
        r#"
my $msg = sprintf(
    "Error: %s at line %d",
    $err,
    $line,
);
"#,
    );
}

#[test]
fn test_printf_to_handle_multiline() {
    // printf with filehandle and multi-line args
    assert_clean_parse(
        r#"
printf(
    "%-20s %5d\n",
    $name,
    $count,
);
"#,
    );
}

#[test]
fn test_sprintf_in_assignment_multiline() {
    // sprintf result assigned, args span multiple lines
    assert_clean_parse(
        r#"
my $label = sprintf(
    "%s/%s",
    $prefix,
    $suffix,
);
"#,
    );
}

// -- Sub-pattern 3: heredoc embedded in paren context --

#[test]
fn test_heredoc_as_first_arg_in_paren() {
    // Heredoc as the first argument in a function call
    assert_clean_parse("foo(<<END, $x);\nheredoc body\nEND\n");
}

#[test]
fn test_heredoc_as_string_in_list() {
    // Heredoc in a list context
    assert_clean_parse("my @items = (<<END, $x);\nsome text\nEND\n");
}

#[test]
fn test_heredoc_multiline_call() {
    // Heredoc used as value in a function call
    assert_clean_parse("print(<<END);\nhello world\nEND\n");
}

// -- Sub-pattern 4: method chain `->` inside paren argument list --

#[test]
fn test_method_chain_in_func_args() {
    // Multi-line method chain as argument to a function
    assert_clean_parse(
        r#"
my $result = some_func(
    $obj->method1()
        ->method2()
        ->method3(),
    $other_arg,
);
"#,
    );
}

#[test]
fn test_method_chain_single_continuation() {
    // Single method chain continuation across lines
    assert_clean_parse(
        r#"
my $r = func(
    $self->build()
         ->finalize(),
);
"#,
    );
}

#[test]
fn test_constructor_with_chained_method_in_arg() {
    // Constructor call with chained method as arg
    assert_clean_parse(
        r#"
Foo->new(
    handler => $obj->build_handler()
                   ->configure($cfg),
    name    => $name,
);
"#,
    );
}

// -- Sub-pattern 5: unless with long condition --

#[test]
fn test_unless_multiline_and_condition() {
    // unless with multi-line && condition
    assert_clean_parse(
        r#"
unless ($some_long_condition
        && $another_condition
        && $yet_another) {
    do_something();
}
"#,
    );
}

#[test]
fn test_unless_multiline_or_condition() {
    // unless with multi-line || condition
    assert_clean_parse(
        r#"
unless ($err
        || $warn) {
    proceed();
}
"#,
    );
}

#[test]
fn test_if_multiline_complex_condition() {
    // if with complex multi-line condition (mirrors unless pattern)
    assert_clean_parse(
        r#"
if ($self->is_valid()
    && defined($self->{data})
    && $self->{data} ne '') {
    return 1;
}
"#,
    );
}

#[test]
fn test_print_block_filehandle_without_comma_in_parens() {
    // Real-world pattern from Dpkg::Source::Archive:
    // print({ *$self->{tar_input} } "$file\0")
    assert_clean_parse(r#"print({ *$self->{tar_input} } "$file\0");"#);
}
