/// Tests for `new(...)` used as a plain function call (no class name).
/// In Perl, `new()` can be called without a class name inside a sub —
/// e.g. `new($rtsig, $val, $flags)` in POSIX.pm STORE method.
///
/// Bug: The parser's "new" arm in primary.rs unconditionally calls
/// parse_qualified_identifier() which eats the `(` token, causing a parse error.
mod cpan_test_helpers;
use cpan_test_helpers::*;

/// Simplest case: `new(args)` as a plain function call.
#[test]
fn test_new_with_parens_no_class() {
    assert_clean_parse(r#"new($a, $b, $c);"#);
}

/// POSIX.pm STORE: new() inside a sub body.
#[test]
fn test_new_parens_in_sub_body() {
    assert_clean_parse(
        r#"sub STORE { my $rtsig = &_check; new($rtsig, $_[2], $SIGACTION_FLAGS) }"#,
    );
}

/// new() with a single argument.
#[test]
fn test_new_parens_single_arg() {
    assert_clean_parse(r#"new($x);"#);
}

/// new() with no arguments (empty parens).
#[test]
fn test_new_parens_empty() {
    assert_clean_parse(r#"new();"#);
}

/// new() assigned to a variable.
#[test]
fn test_new_parens_assigned() {
    assert_clean_parse(r#"my $obj = new($class, %opts);"#);
}

/// Classic indirect-object form should still work: new ClassName(args).
#[test]
fn test_new_indirect_class_still_works() {
    assert_clean_parse(r#"my $obj = new Foo($x, $y);"#);
}

/// Qualified class still works: new IO::Handle().
#[test]
fn test_new_qualified_class_still_works() {
    assert_clean_parse(r#"my $fh = new IO::Handle();"#);
}

/// new() result used in method chain: new($x)->method() must chain correctly.
/// The FunctionCall node must surface to parse_postfix for chaining.
#[test]
fn test_new_parens_method_chain() {
    assert_clean_parse(r#"my $v = new($class)->new_method();"#);
}

/// new() chained into hash subscript.
#[test]
fn test_new_parens_hash_chain() {
    assert_clean_parse(r#"my $v = new($class)->{key};"#);
}
