mod cpan_test_helpers;
use cpan_test_helpers::parse;

/// Strict clean-parse check that catches ALL error variants including
/// uppercase `(ERROR "...")` from error recovery.
fn assert_no_errors(source: &str) {
    let ast = parse(source);
    let sexp = ast.to_sexp();
    let error_markers = [
        "(error ",
        "(Error ",
        "(ERROR ",
        "(missing_expression",
        "(missing_statement",
        "(missing_identifier",
        "(missing_block",
        "MissingExpression",
        "MissingStatement",
        "MissingIdentifier",
        "MissingBlock",
    ];
    for marker in &error_markers {
        assert!(
            !sexp.contains(marker),
            "Parse produced error for source:\n{}\n\nMarker: {}\nSexp: {}",
            source,
            marker,
            sexp,
        );
    }
}

// --- Function call argument lists with fat arrow ---

#[test]
fn test_method_call_with_fat_arrow_args() {
    assert_no_errors(r#"$obj->has(name => 'foo');"#);
}

#[test]
fn test_method_call_with_multiple_fat_arrow_args() {
    assert_no_errors(r#"$obj->method(key => 'value', another => 42);"#);
}

#[test]
fn test_function_call_with_fat_arrow_args() {
    assert_no_errors(r#"foo(key => 'value');"#);
}

#[test]
fn test_function_call_with_multiple_fat_arrow_args() {
    assert_no_errors(r#"my $x = func(a => 1, b => 2);"#);
}

#[test]
fn test_nested_parens_with_fat_arrow() {
    assert_no_errors(r#"Moo::has(name => (is => 'ro'));"#);
}

#[test]
fn test_method_call_mixed_args() {
    assert_no_errors(r#"$obj->method('positional', key => 'value');"#);
}

#[test]
fn test_fat_arrow_with_arrayref_value() {
    assert_no_errors(r#"$obj->has(isa => ['Str', 'Int']);"#);
}

#[test]
fn test_fat_arrow_with_hashref_value() {
    assert_no_errors(r#"$obj->configure(options => { verbose => 1 });"#);
}

#[test]
fn test_fat_arrow_with_coderef_value() {
    assert_no_errors(r#"$obj->add(trigger => sub { 1 });"#);
}

#[test]
fn test_has_moose_style() {
    assert_no_errors(r#"has 'name' => (is => 'ro', isa => 'Str', default => 'foo');"#);
}

#[test]
fn test_has_method_call_moose_style() {
    assert_no_errors(r#"$meta->add_attribute(name => (is => 'rw', isa => 'Str'));"#);
}

// --- Issue #2147: use/no parenthesized import list with fat arrow ---

#[test]
fn test_use_paren_import_with_fat_arrow() {
    assert_no_errors(r#"use parent (key => 'value');"#);
}

#[test]
fn test_use_paren_import_multiple_fat_arrows() {
    assert_no_errors(r#"use Module (foo => 1, bar => 2);"#);
}

#[test]
fn test_use_paren_import_mixed_comma_fat_arrow() {
    assert_no_errors(r#"use Module ('export1', key => 'value');"#);
}

#[test]
fn test_no_paren_import_with_fat_arrow() {
    assert_no_errors(r#"no Module (key => 'value');"#);
}

// --- Coderef call with fat arrow args ---

#[test]
fn test_coderef_call_with_fat_arrow() {
    assert_no_errors(r#"&$func(key => 'value');"#);
}

#[test]
fn test_dbi_connect_style() {
    assert_no_errors(r#"DBI->connect($dsn, $user, $pass, { RaiseError => 1 });"#);
}
