mod cpan_test_helpers;

use perl_parser_core::Parser;

// Tests for #1354: Parser incorrectly flags method calls in interpolated strings.
// Bug: find_unclosed_interpolation_delimiter wrongly tries to balance parens for
// method calls like $obj->method() inside double-quoted strings.
// Fix: In Perl, method calls are NOT interpolated (only $scalar, $array->[idx],
// $hash->{key} are). Remove the two ->(  / ->identifier( check blocks.

fn parse_has_no_errors(source: &str) -> bool {
    let mut parser = Parser::new(source);
    let _ = parser.parse();
    parser.get_errors().is_empty()
}

#[test]
fn method_call_string_simple() {
    // Simple method call in a double-quoted string should parse cleanly.
    // In Perl, "$obj->method()" interpolates $obj and leaves "->method()" as literal.
    let source = r#"my $x = "$obj->method()";"#;
    assert!(parse_has_no_errors(source), "Expected no errors for simple method call");
}

#[test]
fn method_call_with_args() {
    // Method call with arguments should not trigger false paren-balancing check.
    let source = r#"my $y = "$obj->foo(bar, baz)";"#;
    assert!(parse_has_no_errors(source), "Expected no errors for method call with args");
}

#[test]
fn nested_method_calls_in_string() {
    // Chained method calls should not trigger cascading errors.
    let source = r#"my $z = "$x->method1()->method2()";"#;
    assert!(parse_has_no_errors(source), "Expected no errors for chained method calls");
}

#[test]
fn dbi_pm_line_785_reproduction() {
    // Real-world case from DBI.pm line 785 (CPAN corpus).
    // This should parse cleanly without "Unclosed ( delimiter in interpolated string" error.
    let source = r#"
my $class = "Class";
my $driver = "Driver";
$class->trace_msg("    -> $class->install_driver($driver"
        .") for $^O\n");
"#;
    assert!(parse_has_no_errors(source), "Expected no errors for DBI.pm case");
}

#[test]
fn hash_dereference_in_string() {
    // Hash dereference IS interpolated in Perl — must still work correctly.
    let source = r#"my $h = "$obj->{key}";"#;
    assert!(parse_has_no_errors(source), "Expected no errors for hash dereference");
}

#[test]
fn hash_and_array_dereference_are_valid() {
    // Both array and hash derefs are interpolated — must not regress.
    let source = r#"
my $a = "$obj->{key}";
my $b = "$obj->[0]";
"#;
    assert!(parse_has_no_errors(source), "Expected no errors for array/hash derefs");
}

#[test]
fn method_call_with_empty_parens() {
    // Simplest case: $obj->method() with no arguments.
    let source = r#"print "$obj->method()";"#;
    assert!(parse_has_no_errors(source), "Expected no errors for method call with empty parens");
}

#[test]
fn method_call_with_numeric_argument() {
    // Method call with a numeric argument (common pattern).
    let source = r#"my $val = "$obj->foo(42)";"#;
    assert!(parse_has_no_errors(source), "Expected no errors for method call with numeric arg");
}

#[test]
fn method_call_with_string_argument() {
    // Method call with a string argument.
    let source = r#"my $val = "$obj->foo('bar')";"#;
    assert!(parse_has_no_errors(source), "Expected no errors for method call with string arg");
}

#[test]
fn method_call_underscore_prefix() {
    // Underscore is valid in identifiers (PARSER-3 adversarial test).
    let source = r#"my $x = "$obj->_method()";"#;
    assert!(parse_has_no_errors(source), "Expected no errors for method with underscore prefix");
}

#[test]
fn method_call_all_caps() {
    // Reserved method names like DESTROY and new (PARSER-3 adversarial test).
    let source = r#"
my $d = "$obj->DESTROY()";
my $n = "$obj->new()";
"#;
    assert!(parse_has_no_errors(source), "Expected no errors for reserved method names");
}

#[test]
fn mixed_valid_and_invalid_deref_in_string() {
    // PARSER-2 adversarial test: valid hash deref next to method call.
    // Both should parse cleanly.
    let source = r#"my $mixed = "$a->{key}$b->method()";"#;
    assert!(parse_has_no_errors(source), "Expected no errors for mixed valid/invalid derefs");
}

#[test]
fn escaped_paren_in_array_deref() {
    // PARSER-1 adversarial test: escaped paren must not affect balance state.
    let source = r#"my $escaped = "$x->[\)]";"#;
    assert!(parse_has_no_errors(source), "Expected no errors for escaped paren in array deref");
}

#[test]
fn method_call_followed_by_text() {
    // Method call followed by literal text in the string.
    let source = r#"my $after = "$obj->method() more text";"#;
    assert!(parse_has_no_errors(source), "Expected no errors for method call followed by text");
}

#[test]
fn multiple_method_calls_separate_scalars() {
    // Multiple separate scalar interpolations, each with method calls.
    let source = r#"my $multi = "$obj1->foo() and $obj2->bar()";"#;
    assert!(parse_has_no_errors(source), "Expected no errors for multiple method calls");
}
